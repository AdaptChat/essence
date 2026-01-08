use crate::db::EmojiDbExt;
use crate::db::emoji::construct_emoji;
use crate::db::role::{RoleRecord, query_roles};
use crate::models::CustomEmoji;
use crate::{
    Error, NotFoundExt, cache,
    db::{
        ChannelDbExt, DbExt, MemberDbExt, RoleDbExt, channel::query_channels, get_pool,
        member::construct_member,
    },
    http::guild::{CreateGuildPayload, EditGuildPayload, GetGuildQuery},
    models::{
        Guild, GuildChannel, GuildFlags, GuildMemberCount, MaybePartialUser, Member, PartialGuild,
        PermissionPair, Permissions, Role, RoleFlags,
    },
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

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
    /// Builds a cache of all known guild IDs.
    async fn build_guild_cache(&self) -> crate::Result<()> {
        let mut guild_ids = HashSet::new();

        for guild_id in sqlx::query!("SELECT id FROM guilds")
            .fetch_all(self.executor())
            .await?
        {
            guild_ids.insert(guild_id.id as u64);
        }

        cache::insert_guilds(guild_ids.into_iter().collect::<Vec<u64>>()).await?;
        Ok(())
    }

    /// Asserts a guild with the given ID exists.
    async fn assert_guild_exists(&self, guild_id: u64) -> crate::Result<()> {
        let guild_cached = cache::guild_exist(guild_id).await?;

        let guild_exists = guild_cached.is_some() || {
            self.build_guild_cache().await?;
            cache::guild_exist(guild_id).await?.is_some()
        };

        if !guild_exists {
            return Err(Error::NotFound {
                entity: "guild".to_string(),
                message: format!("Guild with ID {guild_id} does not exist"),
            });
        }

        Ok(())
    }

    /// Builds a cache of all known member IDs for the given guild ID.
    async fn build_member_cache(&self, guild_id: u64) -> crate::Result<()> {
        let mut member_ids = HashSet::new();

        for member_id in sqlx::query!(
            "SELECT id FROM members WHERE guild_id = $1",
            guild_id as i64
        )
        .fetch_all(self.executor())
        .await?
        {
            member_ids.insert(member_id.id as u64);
        }

        cache::update_members_of_guild(guild_id, member_ids.into_iter().collect::<Vec<u64>>())
            .await?;

        Ok(())
    }

    /// Asserts the given user is a member of the given guild.
    ///
    /// # Errors
    /// * If an error occurs with fetching the member.
    async fn base_assert_in_guild(
        &self,
        guild_id: u64,
        user_id: u64,
        error: Error,
    ) -> crate::Result<()> {
        self.assert_guild_exists(guild_id).await?;

        let cached = cache::is_member_of_guild(guild_id, user_id).await?;

        let member_in_guild = cached.is_some() || {
            self.build_member_cache(guild_id).await?;
            cache::is_member_of_guild(guild_id, user_id)
                .await?
                .is_some()
        };

        if !member_in_guild {
            return Err(error);
        }
        Ok(())
    }

    /// Asserts the given user is a member of the given guild, given that the user is the invoker
    /// of this assertion. (In other words, this references the user as "you")
    ///
    /// # Errors
    /// * If an error occurs with fetching the member.
    async fn assert_invoker_in_guild(&self, guild_id: u64, user_id: u64) -> crate::Result<()> {
        self.base_assert_in_guild(
            guild_id,
            user_id,
            Error::NotMember {
                guild_id,
                message: String::from(
                    "You must be a member of the guild to perform the requested action.",
                ),
            },
        )
        .await
    }

    /// Asserts the given user is a member of the given guild, and treats the user as a foreign
    /// user. (In other words, this references the user in the third person)
    ///
    /// # Errors
    /// * If an error occurs with fetching the member.
    async fn assert_member_in_guild(&self, guild_id: u64, user_id: u64) -> crate::Result<()> {
        self.base_assert_in_guild(
            guild_id,
            user_id,
            Error::NotFound {
                entity: "member".to_string(),
                message: format!("Member with ID {user_id} does not exist in guild {guild_id}"),
            },
        )
        .await
    }

    /// Returns `true` if the given user is the owner of the guild.
    async fn is_guild_owner(&self, guild_id: u64, user_id: u64) -> crate::Result<bool> {
        self.assert_guild_exists(guild_id).await?;
        let cached_owner_id = cache::owner_of_guild(guild_id).await?;

        Ok(if let Some(owner_id) = cached_owner_id {
            owner_id == user_id
        } else {
            let owner_id =
                sqlx::query!("SELECT owner_id FROM guilds WHERE id = $1", guild_id as i64)
                    .fetch_one(self.executor())
                    .await?
                    .owner_id as u64;

            cache::update_owner_of_guild(guild_id, owner_id).await?;

            owner_id == user_id
        })
    }

    /// Asserts the given user is the owner of the given guild.
    async fn assert_member_is_owner(&self, guild_id: u64, user_id: u64) -> crate::Result<()> {
        if !self.is_guild_owner(guild_id, user_id).await? {
            return Err(Error::NotOwner {
                guild_id,
                message: String::from(
                    "You must be the owner of the guild to perform the requested action.",
                ),
            });
        }

        Ok(())
    }

    async fn fetch_member_permissions_prefer_db(
        &self,
        guild_id: u64,
        user_id: u64,
        channel_id: Option<u64>,
    ) -> crate::Result<Permissions> {
        self.assert_invoker_in_guild(guild_id, user_id).await?;
        if self.is_guild_owner(guild_id, user_id).await? {
            return Ok(Permissions::all());
        }

        let base = sqlx::query!(
            "SELECT permissions FROM members WHERE id = $1",
            user_id as i64
        )
        .fetch_one(self.executor())
        .await?
        .permissions;
        let mut roles = self.fetch_all_roles_for_member(guild_id, user_id).await?;
        let overwrites = match channel_id {
            Some(channel_id) => Some(self.fetch_channel_overwrites(channel_id).await?),
            None => None,
        };

        Ok(crate::calculate_permissions(
            user_id,
            Permissions::from_bits_truncate(base),
            &mut roles,
            overwrites.as_ref().map(AsRef::as_ref),
        ))
    }

    /// Fetches the calculated permissions value for the given member in the given guild. A channel
    /// ID may be provided to calculate the permissions for a specific channel, otherwise the
    /// permissions for the guild will be calculated.
    ///
    /// # Errors
    /// * If an error occurs with the database.
    async fn fetch_member_permissions(
        &self,
        guild_id: u64,
        user_id: u64,
        channel_id: Option<u64>,
    ) -> crate::Result<Permissions> {
        let cached_permissions = cache::permissions_for(guild_id, user_id, channel_id).await?;

        if let Some(permissions) = cached_permissions {
            Ok(permissions)
        } else {
            let perms = self
                .fetch_member_permissions_prefer_db(guild_id, user_id, channel_id)
                .await?;

            cache::update_permissions_for(guild_id, user_id, channel_id, perms).await?;

            Ok(perms)
        }
    }

    /// Internally used, see [`Self::assert_member_has_permissions`] instead.
    fn assert_member_has_permissions_with(
        &self,
        guild_id: u64,
        member_permissions: Permissions,
        required_permissions: Permissions,
    ) -> crate::Result<()> {
        if !member_permissions.contains(required_permissions) {
            return Err(Error::MissingPermissions {
                guild_id,
                permissions: required_permissions,
                message: "You do not have permission to perform the requested action.".to_string(),
            });
        }

        Ok(())
    }

    /// Asserts the given user has the given permissions in the given guild. A channel ID may be
    /// provided to assert the permissions for a specific channel, otherwise the permissions for the
    /// guild will be asserted.
    ///
    /// # Errors
    /// * If an error occurs with the database.
    /// * If the user does not have the given permissions.
    async fn assert_member_has_permissions(
        &self,
        guild_id: u64,
        user_id: u64,
        channel_id: Option<u64>,
        permissions: Permissions,
    ) -> crate::Result<()> {
        let member_permissions = self
            .fetch_member_permissions(guild_id, user_id, channel_id)
            .await?;

        self.assert_member_has_permissions_with(guild_id, member_permissions, permissions)
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
        let Some(partial) = self.fetch_partial_guild(guild_id).await? else {
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

        let emojis = if query.emojis {
            Some(self.fetch_all_emojis_in_guild(guild_id).await?)
        } else {
            None
        };

        Ok(Some(Guild {
            partial,
            members,
            roles,
            channels,
            emojis,
        }))
    }

    /// Fetches the IDs of all guilds that a user is a member of. This is a much more efficient
    /// method than [`Self::fetch_all_guilds_for_user`] if you only need the IDs.
    ///
    /// # Errors
    /// * If an error occurs with fetching the guilds.
    async fn fetch_all_guild_ids_for_user(&self, user_id: u64) -> crate::Result<Vec<u64>> {
        let guild_ids = sqlx::query!("SELECT guild_id FROM members WHERE id = $1", user_id as i64)
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(|r| r.guild_id as u64)
            .collect();

        Ok(guild_ids)
    }

    /// Fetches the guild count of a user.
    async fn fetch_guild_count(&self, user_id: u64) -> crate::Result<u64> {
        let guild_count = sqlx::query!(
            r#"SELECT COUNT(*) AS "count!" FROM members WHERE id = $1"#,
            user_id as i64,
        )
        .fetch_one(self.executor())
        .await?
        .count as u64;

        Ok(guild_count)
    }

    /// Fetches all guilds that a user is a member of as a partial guild.
    ///
    /// # Errors
    /// * If an error occurs with fetching the guilds.
    async fn fetch_all_partial_guilds_for_user(
        &self,
        user_id: u64,
    ) -> crate::Result<Vec<PartialGuild>> {
        let guilds = sqlx::query!(
            r#"SELECT
                guilds.*,
                (SELECT COUNT(*) FROM members WHERE guild_id = $1) AS "member_count!"
            FROM
                guilds
            WHERE
                EXISTS (SELECT 1 FROM members WHERE members.id = $1 AND guild_id = guilds.id)
            "#,
            user_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| construct_partial_guild!(r))
        .collect();

        Ok(guilds)
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
        let mut guilds: HashMap<u64, Guild> = self
            .fetch_all_partial_guilds_for_user(user_id)
            .await?
            .into_iter()
            .map(|partial| Guild {
                partial,
                members: None,
                roles: None,
                channels: None,
                emojis: None,
            })
            .map(|guild| (guild.partial.id, guild))
            .collect();
        let guild_ids = guilds.keys().map(|&k| k as i64).collect::<Vec<_>>();

        if query.channels {
            let mut overwrites = self.fetch_channel_overwrites_where(
                "guild_id IS NOT NULL AND guild_id = ANY(SELECT guild_id FROM members WHERE id = $1)",
                user_id,
            )
            .await?;

            let out = query_channels!("guild_id = ANY($1::BIGINT[])", &guild_ids)
                .fetch_all(self.executor())
                .await?;

            let channel_ids: Vec<_> = out.iter().map(|r| r.id).collect();
            let mut last_messages = self.fetch_last_message_map(&channel_ids).await?;

            let channels = out
                .into_iter()
                .map(|r| {
                    let id = r.id as u64;
                    r.into_guild_channel(
                        overwrites
                            .get_mut(&id)
                            .unwrap_or(&mut None)
                            .take()
                            .unwrap_or_default(),
                        last_messages.remove(&id),
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
            let roles = query_roles!(
                "guild_id = ANY($1::BIGINT[]) ORDER BY position ASC",
                &guild_ids
            )
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(RoleRecord::into_role)
            .into_group_map_by(|r| r.guild_id);

            for (guild_id, roles) in roles {
                if let Some(guild) = guilds.get_mut(&guild_id) {
                    guild.roles = Some(roles);
                }
            }
        }

        if query.members {
            let role_data = sqlx::query!(
                "SELECT role_id, user_id, guild_id FROM role_data \
                WHERE guild_id = ANY($1::BIGINT[])",
                &guild_ids,
            )
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .into_group_map_by(|r| (r.guild_id as u64, r.user_id as u64));

            let members: HashMap<u64, Vec<Member>> = sqlx::query!(
                r#"SELECT
                    members.*,
                    users.username,
                    users.display_name,
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
                    guild_id = ANY($1::BIGINT[])
                "#,
                &guild_ids,
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

        if query.emojis {
            let emojis: HashMap<u64, Vec<CustomEmoji>> = sqlx::query!(
                "SELECT * FROM emojis WHERE guild_id = ANY($1::BIGINT[])",
                &guild_ids,
            )
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(|r| construct_emoji!(r))
            .into_group_map_by(|e: &CustomEmoji| e.guild_id);

            for (guild_id, emojis) in emojis {
                if let Some(guild) = guilds.get_mut(&guild_id) {
                    guild.emojis = Some(emojis);
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
    #[allow(clippy::too_many_lines)]
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
            payload.name.trim(),
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
        let allowed_permissions = Permissions::DEFAULT;
        let denied_permissions = Permissions::empty();

        sqlx::query!(
            r#"INSERT INTO roles
                (id, guild_id, name, flags, position, allowed_permissions, denied_permissions)
            VALUES
                ($1, $2, 'Default', $3, 0, $4, $5);
            "#,
            role_id as i64,
            guild_id as i64,
            role_flags.bits() as i32,
            allowed_permissions.bits(),
            denied_permissions.bits(),
        )
        .execute(self.transaction())
        .await?;

        // NOTE: we intentionally do not insert the default role into the role_data table as they
        // are implied to all members.

        let permissions = PermissionPair {
            allow: allowed_permissions,
            deny: denied_permissions,
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
            permissions: Permissions::empty(),
        };

        cache::insert_guild(guild_id).await?;
        cache::update_owner_of_guild(guild_id, owner_id).await?;

        Ok(Guild {
            partial,
            members: Some(vec![member]),
            roles: Some(vec![role]),
            channels: Some(vec![channel]),
            emojis: Some(Vec::new()),
        })
    }

    /// Edits the guild with the given ID with the given payload. Validation should be done before
    /// calling this function. Returns a tuple of two [`PartialGuild`]s on success: the first
    /// element is the original guild and the second element is the guild with updated fields.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with editing the guild.
    /// * If the guild does not exist.
    async fn edit_guild(
        &mut self,
        guild_id: u64,
        payload: EditGuildPayload,
    ) -> crate::Result<(PartialGuild, PartialGuild)> {
        let old = get_pool()
            .fetch_partial_guild(guild_id)
            .await?
            .ok_or_not_found("guild", format!("Guild with ID {guild_id} does not exist"))?;
        let mut guild = old.clone();

        if let Some(name) = payload.name {
            guild.name = name;
        }

        guild.description = payload
            .description
            .into_option_or_if_absent(guild.description);
        guild.icon = payload.icon.into_option_or_if_absent(guild.icon);
        guild.banner = payload.banner.into_option_or_if_absent(guild.banner);

        match payload.public {
            Some(true) => guild.flags.insert(GuildFlags::PUBLIC),
            Some(false) => guild.flags.remove(GuildFlags::PUBLIC),
            None => (),
        }

        sqlx::query!(
            r#"UPDATE
                guilds
            SET
                name = $1, description = $2, icon = $3, banner = $4, flags = $5
            WHERE
                id = $6
            "#,
            guild.name,
            guild.description,
            guild.icon,
            guild.banner,
            guild.flags.bits() as i32,
            guild_id as i64,
        )
        .execute(self.transaction())
        .await?;

        Ok((old, guild))
    }

    /// Deletes a guild from the database with the given ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the guild.
    /// * If the guild does not exist.
    async fn delete_guild(&mut self, guild_id: u64) -> crate::Result<()> {
        sqlx::query!("DELETE FROM guilds WHERE id = $1", guild_id as i64)
            .execute(self.transaction())
            .await?;

        cache::remove_guild(guild_id).await?;
        Ok(())
    }
}

impl<'t, T> GuildDbExt<'t> for T where T: DbExt<'t> {}
