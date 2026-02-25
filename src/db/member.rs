use crate::{
    NotFoundExt, cache,
    db::{DbExt, UserDbExt, get_pool},
    http::member::{BanMemberPayload, EditClientMemberPayload, EditMemberPayload},
    models::{GuildBan, MaybePartialUser, Member, ModelType, Permissions, User, UserFlags},
    snowflake::with_model_type,
};
use itertools::Itertools;

macro_rules! query_member {
    ($where:literal, $($arg:expr_2021),* $(,)?) => {{
        sqlx::query!(
            r#"SELECT
                m.id,
                m.guild_id,
                m.nick AS nick,
                m.joined_at AS joined_at,
                m.permissions AS permissions,
                u.username AS username,
                u.display_name AS display_name,
                u.avatar AS avatar,
                u.banner AS banner,
                u.bio AS bio,
                u.flags AS flags
            FROM
                members AS m
            INNER JOIN
                users AS u ON u.id = m.id
            "# + $where,
            $($arg),*
        )
    }};
}

macro_rules! construct_member {
    ($data:ident, $roles:expr_2021) => {{
        use $crate::models::{MaybePartialUser, User, UserFlags};

        Member {
            user: MaybePartialUser::Full(User {
                id: $data.id as _,
                username: $data.username,
                display_name: $data.display_name as _,
                avatar: $data.avatar,
                banner: $data.banner,
                bio: $data.bio,
                flags: UserFlags::from_bits_truncate($data.flags as _),
            }),
            guild_id: $data.guild_id as _,
            nick: $data.nick,
            roles: $roles,
            joined_at: $data.joined_at,
            permissions: Permissions::from_bits_truncate($data.permissions),
        }
    }};
}

pub(crate) use construct_member;

#[async_trait::async_trait]
pub trait MemberDbExt<'t>: DbExt<'t> {
    /// Fetches a user's member record across multiple guilds.
    /// Returns a mapping `guild_id -> member`
    ///
    /// # Errors
    /// * If an error occurs with fetching the members.
    async fn fetch_members_for_user_in_guilds(
        &self,
        user_id: u64,
        guild_ids: &[i64],
    ) -> sqlx::Result<std::collections::HashMap<u64, Member>> {
        let roles = sqlx::query!(
            "SELECT guild_id, role_id FROM role_data WHERE guild_id = ANY($1::BIGINT[]) AND user_id = $2",
            guild_ids,
            user_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .into_group_map_by(|r| r.guild_id as u64);

        let members = query_member!(
            r"WHERE m.guild_id = ANY($1::BIGINT[]) AND m.id = $2",
            guild_ids,
            user_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|m| {
            let gid = m.guild_id as u64;
            let role_ids = roles
                .get(&gid)
                .map(|rs| rs.iter().map(|r| r.role_id as u64).collect::<Vec<_>>());
            (gid, construct_member!(m, role_ids))
        })
        .collect();

        Ok(members)
    }

    /// Fetches a member from the database with the given guild and user ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the member. If the member is not found, `Ok(None)` is
    /// returned.
    async fn fetch_member_by_id(
        &self,
        guild_id: u64,
        user_id: u64,
    ) -> sqlx::Result<Option<Member>> {
        let roles = sqlx::query!(
            "SELECT role_id FROM role_data WHERE guild_id = $1 AND user_id = $2",
            guild_id as i64,
            user_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| r.role_id as u64)
        .collect::<Vec<_>>();

        let member = query_member!(
            "WHERE guild_id = $1 AND m.id = $2",
            guild_id as i64,
            user_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|m| construct_member!(m, Some(roles)));

        Ok(member)
    }

    /// Fetches all members from the database with the given guild ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the members. If the members are not found, `Ok(None)` is
    /// returned.
    /// * If an error occurs with fetching the roles for a member.
    async fn fetch_all_members_in_guild(&self, guild_id: u64) -> sqlx::Result<Vec<Member>> {
        let roles = sqlx::query!(
            "SELECT user_id, role_id FROM role_data WHERE guild_id = $1",
            guild_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .into_group_map_by(|r| r.user_id as u64);

        let members = query_member!("WHERE guild_id = $1", guild_id as i64)
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(|m| {
                construct_member!(
                    m,
                    roles
                        .get(&(m.id as u64))
                        .map(|r| r.iter().map(|r| r.role_id as u64).collect::<Vec<_>>())
                )
            })
            .collect::<Vec<_>>();

        Ok(members)
    }

    /// Edits a member in the database with the given guild, user ID, and payload. The payload
    /// should be validated prior to calling this method.
    ///
    /// **This includes roles; roles must be validated prior to calling this method and roles
    /// that are managed or do not meet required permissions should be removed from the payload.**
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with editing the member.
    async fn edit_member(
        &mut self,
        guild_id: u64,
        user_id: u64,
        payload: EditMemberPayload,
    ) -> crate::Result<(Member, Member)> {
        let mut member = get_pool()
            .fetch_member_by_id(guild_id, user_id)
            .await?
            .ok_or_not_found("member", "member not found")?;
        let old = member.clone();

        member.nick = payload.nick.into_option_or_if_absent(member.nick);
        member.permissions = payload.permissions.unwrap_or(member.permissions);

        sqlx::query!(
            "UPDATE members SET nick = $1, permissions = $2 WHERE guild_id = $3 AND id = $4",
            member.nick,
            member.permissions.bits(),
            guild_id as i64,
            user_id as i64,
        )
        .execute(self.transaction())
        .await?;

        if payload.permissions.is_some() {
            cache::delete_permissions_for_user(guild_id, user_id).await?;
        }

        if let Some(roles) = payload.roles {
            let default_role_id = with_model_type(guild_id, ModelType::Role);
            sqlx::query!(
                r"DELETE FROM role_data WHERE guild_id = $1 AND user_id = $2 AND role_id != $3",
                guild_id as i64,
                user_id as i64,
                default_role_id as i64,
            )
            .execute(self.transaction())
            .await?;

            sqlx::query(
                r"INSERT INTO
                    role_data
                SELECT
                    out.*, $1, $2
                FROM
                    UNNEST($3)
                AS
                    out(role_id)
                WHERE
                    role_id IN (SELECT id FROM roles WHERE guild_id = $2)
                ON CONFLICT DO NOTHING
                ",
            )
            .bind(user_id as i64)
            .bind(guild_id as i64)
            .bind(roles.into_iter().map(|r| r as i64).collect::<Vec<_>>())
            .fetch_all(self.transaction())
            .await?;

            member.roles = Some(
                sqlx::query!(
                    "SELECT role_id FROM role_data WHERE guild_id = $1 AND user_id = $2",
                    guild_id as i64,
                    user_id as i64,
                )
                .fetch_all(self.transaction())
                .await?
                .into_iter()
                .map(|r| r.role_id as u64)
                .collect::<Vec<_>>(),
            );
        }

        Ok((old, member))
    }

    /// Edits a member in the database with the given guild, user ID, and a
    /// [`EditClientMemberPayload`].
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with editing the member.
    async fn edit_client_member(
        &mut self,
        guild_id: u64,
        user_id: u64,
        payload: EditClientMemberPayload,
    ) -> crate::Result<Member> {
        self.edit_member(
            guild_id,
            user_id,
            EditMemberPayload {
                nick: payload.nick,
                roles: None,
                permissions: None,
            },
        )
        .await
        .map(|(_, m)| m)
    }

    /// Creates a member in the database with the given guild and user ID. If the user is already
    /// in the guild, this returns `None`.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with creating the member.
    async fn create_member(
        &mut self,
        guild_id: u64,
        user_id: u64,
        permissions: Permissions,
    ) -> crate::Result<Option<Member>> {
        let user = get_pool().fetch_user_by_id(user_id).await?.map_or(
            MaybePartialUser::Partial { id: user_id },
            MaybePartialUser::Full,
        );
        let member = sqlx::query!(
            "INSERT INTO members (guild_id, id, permissions) VALUES ($1, $2, $3)
            ON CONFLICT (guild_id, id) DO NOTHING RETURNING joined_at",
            guild_id as i64,
            user_id as i64,
            permissions.bits(),
        )
        .fetch_optional(self.transaction())
        .await?
        .map(|m| Member {
            guild_id,
            user,
            nick: None,
            joined_at: m.joined_at,
            roles: None,
            permissions,
        });

        cache::update_member_of_guild(guild_id, user_id).await?;

        Ok(member)
    }

    /// Deletes a member from the database with the given guild and user ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the member.
    async fn delete_member(&mut self, guild_id: u64, user_id: u64) -> crate::Result<()> {
        sqlx::query!(
            "DELETE FROM members WHERE guild_id = $1 AND id = $2",
            guild_id as i64,
            user_id as i64,
        )
        .execute(self.transaction())
        .await?;

        cache::remove_member_from_guild(guild_id, user_id).await?;
        Ok(())
    }

    /// Fetches a ban entry from the database with the given guild and user ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the ban.
    async fn fetch_ban(&self, guild_id: u64, user_id: u64) -> crate::Result<Option<GuildBan>> {
        let ban = sqlx::query!(
            r#"SELECT
                bans.guild_id, bans.user_id, bans.moderator_id, bans.reason, bans.banned_at,
                users.username, users.display_name, users.avatar, users.banner, users.bio, users.flags
            FROM bans
            INNER JOIN users ON bans.user_id = users.id
            WHERE bans.guild_id = $1 AND bans.user_id = $2"#,
            guild_id as i64,
            user_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|r| GuildBan {
            guild_id: r.guild_id as u64,
            user: MaybePartialUser::Full(User {
                id: r.user_id as u64,
                username: r.username,
                display_name: r.display_name,
                avatar: r.avatar,
                banner: r.banner,
                bio: r.bio,
                flags: UserFlags::from_bits_truncate(r.flags as _),
            }),
            moderator_id: r.moderator_id as u64,
            reason: r.reason,
            banned_at: r.banned_at,
        });

        Ok(ban)
    }

    /// Fetches all ban entries for the given guild.
    ///
    /// # Errors
    /// * If an error occurs with fetching the bans.
    async fn fetch_all_bans(&self, guild_id: u64) -> crate::Result<Vec<GuildBan>> {
        let bans = sqlx::query!(
            r#"SELECT
                bans.guild_id, bans.user_id, bans.moderator_id, bans.reason, bans.banned_at,
                users.username, users.display_name, users.avatar, users.banner, users.bio, users.flags
            FROM bans
            INNER JOIN users ON bans.user_id = users.id
            WHERE bans.guild_id = $1"#,
            guild_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| GuildBan {
            guild_id: r.guild_id as u64,
            user: MaybePartialUser::Full(User {
                id: r.user_id as u64,
                username: r.username,
                display_name: r.display_name,
                avatar: r.avatar,
                banner: r.banner,
                bio: r.bio,
                flags: UserFlags::from_bits_truncate(r.flags as _),
            }),
            moderator_id: r.moderator_id as u64,
            reason: r.reason,
            banned_at: r.banned_at,
        })
        .collect();

        Ok(bans)
    }

    /// Bans a user from a guild. If the user is currently a member, they are removed. Returns the
    /// created [`GuildBan`] entry.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If the user is already banned.
    /// * If an error occurs with banning the user.
    async fn ban_member(
        &mut self,
        guild_id: u64,
        user_id: u64,
        moderator_id: u64,
        payload: BanMemberPayload,
    ) -> crate::Result<GuildBan> {
        let ban_exists = cache::is_banned(guild_id, user_id).await?
            || sqlx::query!(
                "SELECT 1 AS exists FROM bans WHERE guild_id = $1 AND user_id = $2",
                guild_id as i64,
                user_id as i64,
            )
            .fetch_optional(self.executor())
            .await?
            .is_some();

        if ban_exists {
            return Err(crate::Error::AlreadyExists {
                what: "ban".to_string(),
                message: format!("User {user_id} is already banned from guild {guild_id}"),
            });
        }

        sqlx::query!(
            "DELETE FROM members WHERE guild_id = $1 AND id = $2",
            guild_id as i64,
            user_id as i64,
        )
        .execute(self.transaction())
        .await?;
        cache::remove_member_from_guild(guild_id, user_id).await?;

        let ban = sqlx::query!(
            "INSERT INTO bans (guild_id, user_id, moderator_id, reason)
             VALUES ($1, $2, $3, $4)
             RETURNING banned_at",
            guild_id as i64,
            user_id as i64,
            moderator_id as i64,
            payload.reason.as_deref(),
        )
        .fetch_one(self.transaction())
        .await
        .map(|r| GuildBan {
            guild_id,
            user: MaybePartialUser::Partial { id: user_id },
            moderator_id,
            reason: payload.reason,
            banned_at: r.banned_at,
        })?;

        cache::add_ban(guild_id, user_id).await?;
        Ok(ban)
    }

    /// Removes a ban for the given user from the guild. Returns `true` if a ban was removed,
    /// `false` if the user was not banned.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with removing the ban.
    async fn unban_member(&mut self, guild_id: u64, user_id: u64) -> crate::Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM bans WHERE guild_id = $1 AND user_id = $2",
            guild_id as i64,
            user_id as i64,
        )
        .execute(self.transaction())
        .await?;

        cache::remove_ban(guild_id, user_id).await?;
        Ok(result.rows_affected() > 0)
    }
}

impl<'t, T> MemberDbExt<'t> for T where T: DbExt<'t> {}
