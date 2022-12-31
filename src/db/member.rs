use crate::{db::DbExt, models::Member};
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
                members m
            CROSS JOIN LATERAL (
                SELECT * FROM users u WHERE u.id = m.id
            ) AS u
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
}

impl<'t, T> MemberDbExt<'t> for T where T: DbExt<'t> {}
