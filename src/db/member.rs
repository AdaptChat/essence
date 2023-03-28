use crate::{cache, db::DbExt, models::Member, snowflake::with_model_type, NotFoundExt};
use itertools::Itertools;

macro_rules! query_member {
    ($where:literal, $($arg:expr),* $(,)?) => {{
        sqlx::query!(
            r#"SELECT
                m.id,
                m.guild_id,
                m.nick AS nick,
                m.joined_at AS joined_at,
                u.username AS username,
                u.discriminator AS discriminator,
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
    ($data:ident, $roles:expr) => {{
        use $crate::models::{MaybePartialUser, User, UserFlags};

        Member {
            user: MaybePartialUser::Full(User {
                id: $data.id as _,
                username: $data.username,
                discriminator: $data.discriminator as _,
                avatar: $data.avatar,
                banner: $data.banner,
                bio: $data.bio,
                flags: UserFlags::from_bits_truncate($data.flags as _),
            }),
            guild_id: $data.guild_id as _,
            nick: $data.nick,
            roles: $roles,
            joined_at: $data.joined_at,
        }
    }};
}

use crate::db::{get_pool, UserDbExt};
use crate::http::member::{EditClientMemberPayload, EditMemberPayload};
use crate::models::{MaybePartialUser, ModelType};
pub(crate) use construct_member;

#[async_trait::async_trait]
pub trait MemberDbExt<'t>: DbExt<'t> {
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
            "SELECT role_id FROM role_data WHERE guild_id = $1",
            guild_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .into_group_map_by(|r| r.role_id as u64);

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
    ) -> crate::Result<Member> {
        let mut member = get_pool()
            .fetch_member_by_id(guild_id, user_id)
            .await?
            .ok_or_not_found("member", "member not found")?;

        member.nick = payload.nick.into_option_or_if_absent(member.nick);

        sqlx::query!(
            "UPDATE members SET nick = $1 WHERE guild_id = $2 AND id = $3",
            member.nick,
            guild_id as i64,
            user_id as i64,
        )
        .execute(self.transaction())
        .await?;

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
                r#"INSERT INTO
                    role_data
                SELECT
                    $1, $2, out.*
                FROM
                    UNNEST($3)
                AS
                    out(role_id)
                WHERE
                    role_id IN (SELECT id FROM roles WHERE guild_id = $1)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(guild_id as i64)
            .bind(user_id as i64)
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

        Ok(member)
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
            },
        )
        .await
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
    ) -> crate::Result<Option<Member>> {
        let user = get_pool().fetch_user_by_id(user_id).await?.map_or(
            MaybePartialUser::Partial { id: user_id },
            MaybePartialUser::Full,
        );
        let member = sqlx::query!(
            "INSERT INTO members (guild_id, id) VALUES ($1, $2)
            ON CONFLICT (guild_id, id) DO NOTHING RETURNING joined_at",
            guild_id as i64,
            user_id as i64,
        )
        .fetch_optional(self.transaction())
        .await?
        .map(|m| Member {
            guild_id,
            user,
            nick: None,
            joined_at: m.joined_at,
            roles: None,
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
}

impl<'t, T> MemberDbExt<'t> for T where T: DbExt<'t> {}
