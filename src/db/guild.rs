use crate::{
    db::{
        channel::{construct_guild_channel, query_guild_channels},
        member::construct_member,
        role::construct_role,
        ChannelDbExt, DbExt, MemberDbExt, RoleDbExt,
    },
    http::guild::{CreateGuildPayload, GetGuildQuery},
    models::{
        Guild, GuildChannel, GuildFlags, GuildMemberCount, MaybePartialUser, Member, PartialGuild,
        PermissionPair, Permissions, Role, RoleFlags,
    },
    Error,
};
use itertools::Itertools;
use std::collections::HashMap;

macro_rules! construct_partial_guild {
    ($data:ident) => {{
        PartialGuild {
            id: $data.id as _,
            name: $data.name,
            description: $data.description,
            icon: $data.icon,
            banner: $data.banner,
            owner_id: $data.owner_id as _,
            flags: GuildFlags::from_bits_truncate($data.flags as _),
            member_count: Some(GuildMemberCount {
                total: $data.member_count as _,
                online: None, // TODO
            }),
            vanity_url: $data.vanity_url,
        }
    }};
}

#[async_trait::async_trait]
pub trait GuildDbExt<'t>: DbExt<'t> {
    /// Asserts a guild with the given ID exists.
    async fn assert_guild_exists(&self, guild_id: u64) -> crate::Result<()> {
        if !sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM guilds WHERE id = $1)",
            guild_id as i64
        )
        .fetch_one(self.executor())
        .await?
        .exists
        .unwrap_or(false)
        {
            return Err(Error::NotFound {
                entity: "guild",
                message: format!("Guild with ID {guild_id} does not exist"),
            });
        }

        Ok(())
    }

    /// Asserts the given user is a member of the given guild.
    ///
    /// # Errors
    /// * If an error occurs with fetching the member.
    async fn assert_member_in_guild(&self, guild_id: u64, user_id: u64) -> crate::Result<()> {
        self.assert_guild_exists(guild_id).await?;

        if !sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM members WHERE guild_id = $1 AND id = $2)",
            guild_id as i64,
            user_id as i64,
        )
        .fetch_one(self.executor())
        .await?
        .exists
        .unwrap_or(false)
        {
            return Err(Error::NotMember {
                guild_id,
                message: "You must be a member of the guild to perform the requested action.",
            });
        }

        Ok(())
    }

    /// Fetches a partial guild from the database with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the guild. If the guild is not found, `Ok(None)` is
    /// returned.
    async fn fetch_partial_guild(&self, guild_id: u64) -> sqlx::Result<Option<PartialGuild>> {
        let guild = sqlx::query!(
            r#"SELECT
                id,
                name,
                description,
                icon,
                banner,
                owner_id,
                flags,
                vanity_url,
                (SELECT COUNT(*) FROM members WHERE guild_id = $1) AS "member_count!"
            FROM
                guilds
            WHERE
                id = $1"#,
            guild_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|r| construct_partial_guild!(r));

        Ok(guild)
    }

    /// Fetches a guild from the database with the given ID and query.
    ///
    /// # Errors
    /// * If an error occurs with fetching the guild. If the guild is not found, `Ok(None)` is
    /// returned.
    async fn fetch_guild(
        &self,
        guild_id: u64,
        query: GetGuildQuery,
    ) -> crate::Result<Option<Guild>> {
        let partial = if let Some(partial) = self.fetch_partial_guild(guild_id).await? {
            partial
        } else {
            return Ok(None);
        };

        let channels = if query.channels {
            Some(self.fetch_all_channels_in_guild(guild_id).await?)
        } else {
            None
        };

        let roles = if query.roles {
            Some(self.fetch_all_roles_in_guild(guild_id).await?)
        } else {
            None
        };

        let members = if query.members {
            Some(self.fetch_all_members_in_guild(guild_id).await?)
        } else {
            None
        };

        Ok(Some(Guild {
            partial,
            members,
            roles,
            channels,
        }))
    }

    /// Fetches all guilds that a user is a member of, abiding by the query.
    ///
    /// # Errors
    /// * If an error occurs with fetching the guilds.
    #[allow(clippy::too_many_lines)]
    async fn fetch_all_guilds_for_user(
        &self,
        user_id: u64,
        query: GetGuildQuery,
    ) -> crate::Result<Vec<Guild>> {
        let mut guilds: HashMap<u64, Guild> = sqlx::query!(
            r#"SELECT 
                guilds.*,
                (SELECT COUNT(*) FROM members WHERE guild_id = $1) AS "member_count!"
            FROM
                guilds 
            WHERE 
                id IN (SELECT guild_id FROM members WHERE id = $1)
            "#,
            user_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| Guild {
            partial: construct_partial_guild!(r),
            members: None,
            roles: None,
            channels: None,
        })
        .map(|guild| (guild.partial.id, guild))
        .collect();

        if query.channels {
            let mut overwrites = self.fetch_channel_overwrites_where(
                "guild_id IS NOT NULL AND guild_id IN (SELECT guild_id FROM members WHERE id = $1)",
                user_id,
            )
            .await?;

            let channels = query_guild_channels!(
                "guild_id IN (SELECT guild_id FROM members WHERE id = $1)",
                user_id as i64
            )
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(|r| {
                construct_guild_channel!(
                    r,
                    overwrites
                        .get_mut(&(r.id as u64))
                        .unwrap_or(&mut None)
                        .take()
                        .unwrap_or_default()
                )
            })
            .collect::<crate::Result<Vec<_>>>()?
            .into_iter()
            .into_group_map_by(|c| c.guild_id);

            for (guild_id, channels) in channels {
                if let Some(guild) = guilds.get_mut(&guild_id) {
                    guild.channels = Some(channels);
                }
            }
        }

        if query.roles {
            let roles = sqlx::query!(
                r#"SELECT
                    *
                FROM
                    roles
                WHERE
                    guild_id IN (SELECT guild_id FROM members WHERE id = $1)
                ORDER BY
                    position ASC
                "#,
                user_id as i64,
            )
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(|r| construct_role!(r))
            .into_group_map_by(|r| r.guild_id);

            for (guild_id, roles) in roles {
                if let Some(guild) = guilds.get_mut(&guild_id) {
                    guild.roles = Some(roles);
                }
            }
        }

        if query.members {
            let role_data = sqlx::query!(
                r#"SELECT
                    role_id,
                    user_id,
                    guild_id
                FROM
                    role_data
                WHERE
                    guild_id IN (SELECT guild_id FROM members WHERE id = $1)
                "#,
                user_id as i64,
            )
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .into_group_map_by(|r| (r.guild_id as u64, r.user_id as u64));

            let members: HashMap<u64, Vec<Member>> = sqlx::query!(
                r#"SELECT
                    members.*,
                    users.username,
                    users.discriminator,
                    users.avatar,
                    users.banner,
                    users.bio,
                    users.flags
                FROM
                    members
                INNER JOIN
                    users
                ON
                    members.id = users.id
                WHERE
                    guild_id IN (SELECT guild_id FROM members WHERE id = $1)
                "#,
                user_id as i64,
            )
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(|r| construct_member!(r, None))
            .into_group_map_by(|m| m.guild_id);

            for (guild_id, mut members) in members {
                if let Some(guild) = guilds.get_mut(&guild_id) {
                    for member in &mut members {
                        if let Some(roles) = role_data.get(&(guild_id, member.user_id())) {
                            member.roles = Some(roles.iter().map(|r| r.role_id as u64).collect());
                        }
                    }

                    guild.members = Some(members);
                }
            }
        }

        Ok(guilds.into_values().collect())
    }

    /// Creates a guild in the database with the given ID, owner ID and payload. Validation should
    /// be done before calling this function.
    ///
    /// * `channel_id` is the ID of the default `general` channel all guilds come with.
    /// * `role_id` is the ID of the default role (the `@everyone` role).
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with creating the guild.
    async fn create_guild(
        &mut self,
        guild_id: u64,
        channel_id: u64,
        role_id: u64,
        owner_id: u64,
        payload: CreateGuildPayload,
    ) -> crate::Result<Guild> {
        let flags = payload
            .public
            .then_some(GuildFlags::PUBLIC)
            .unwrap_or_default();

        sqlx::query!(
            r#"INSERT INTO
                guilds (id, name, description, icon, banner, owner_id, flags)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            "#,
            guild_id as i64,
            payload.name,
            payload.description,
            payload.icon,
            payload.banner,
            owner_id as i64,
            flags.bits() as i32,
        )
        .execute(self.transaction())
        .await?;

        let joined_at = sqlx::query!(
            "INSERT INTO members (id, guild_id) VALUES ($1, $2) RETURNING joined_at",
            owner_id as i64,
            guild_id as i64,
        )
        .fetch_one(self.transaction())
        .await?
        .joined_at;

        let role_flags = RoleFlags::DEFAULT;
        let perms = sqlx::query!(
            r#"INSERT INTO roles
                (id, guild_id, name, flags, position)
            VALUES
                ($1, $2, 'Default', $3, 0)
            RETURNING
                allowed_permissions AS "allowed_permissions!",
                denied_permissions AS "denied_permissions!"
            "#,
            role_id as i64,
            guild_id as i64,
            role_flags.bits() as i32,
        )
        .fetch_one(self.transaction())
        .await?;

        let permissions = PermissionPair {
            allow: Permissions::from_bits_truncate(perms.allowed_permissions),
            deny: Permissions::from_bits_truncate(perms.denied_permissions),
        };

        sqlx::query!(
            r#"INSERT INTO channels
                (id, guild_id, type, name, position, slowmode, nsfw, locked)
            VALUES
                ($1, $2, 'text', 'general', 0, 0, false, false)"#,
            channel_id as i64,
            guild_id as i64,
        )
        .execute(self.transaction())
        .await?;

        let partial = PartialGuild {
            id: guild_id,
            name: payload.name,
            description: payload.description,
            icon: payload.icon,
            banner: payload.banner,
            owner_id,
            flags,
            member_count: Some(GuildMemberCount {
                total: 1,
                online: None, // TODO
            }),
            vanity_url: None,
        };

        let role = Role {
            id: role_id,
            guild_id,
            name: "Default".to_string(),
            permissions,
            flags: role_flags,
            ..Role::default()
        };

        let channel = GuildChannel {
            id: channel_id,
            guild_id,
            ..GuildChannel::default()
        };

        let member = Member {
            user: MaybePartialUser::Partial { id: owner_id },
            guild_id,
            nick: None,
            roles: Some(vec![role_id]),
            joined_at,
        };

        Ok(Guild {
            partial,
            members: Some(vec![member]),
            roles: Some(vec![role]),
            channels: Some(vec![channel]),
        })
    }
}

impl<'t, T> GuildDbExt<'t> for T where T: DbExt<'t> {}
