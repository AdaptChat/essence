use crate::{
    db::{channel::ChannelDbExt, DbExt, MemberDbExt, RoleDbExt},
    http::guild::{CreateGuildPayload, GetGuildQuery},
    models::{
        Guild, GuildChannel, GuildFlags, GuildMemberCount, MaybePartialUser, Member, PartialGuild,
        PermissionPair, Permissions, Role, RoleFlags,
    },
};

#[async_trait::async_trait]
pub trait GuildDbExt<'t>: DbExt<'t> {
    /// Fetches a partial guild from the database with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the guild. If the guild is not found, `Ok(None)` is
    /// returned.
    async fn fetch_partial_guild(&self, guild_id: u64) -> sqlx::Result<Option<PartialGuild>> {
        let guild = sqlx::query!(
            r#"SELECT
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
        .map(|r| PartialGuild {
            id: guild_id,
            name: r.name,
            description: r.description,
            icon: r.icon,
            banner: r.banner,
            owner_id: r.owner_id as _,
            flags: GuildFlags::from_bits_truncate(r.flags as _),
            member_count: Some(GuildMemberCount {
                total: r.member_count as _,
                online: None, // TODO
            }),
            vanity_url: r.vanity_url,
        });

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
            guild_id as i64,
            role_id as i64,
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
