use super::{get_pool, DbExt};
use crate::models::user::{ClientUser, User, UserFlags};

macro_rules! construct_user {
    ($data:ident) => {{
        User {
            id: $data.id as _,
            username: $data.username,
            discriminator: $data.discriminator as _,
            avatar: $data.avatar,
            banner: $data.banner,
            bio: $data.bio,
            flags: UserFlags::from_bits_truncate($data.flags as _),
        }
    }};
}

macro_rules! fetch_user {
    ($self:ident, $query:literal, $($arg:expr),* $(,)?) => {{
        let result = sqlx::query!($query, $($arg),*)
            .fetch_optional($self.executor())
            .await?
            .map(|r| construct_user!(r));

        Ok(result)
    }};
}

#[async_trait::async_trait]
pub trait UserDbExt: for<'a> DbExt<'a> {
    /// Fetches a user from the database with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
    /// returned.
    async fn fetch_user_by_id(&self, id: u64) -> sqlx::Result<Option<User>> {
        fetch_user!(self, "SELECT * FROM users WHERE id = $1", id as i64)
    }

    /// Fetches a user from the database with the given username and discriminator.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
    /// returned.
    async fn fetch_user_by_tag(
        &self,
        username: &str,
        discriminator: u16,
    ) -> sqlx::Result<Option<User>> {
        fetch_user!(
            self,
            "SELECT * FROM users WHERE username = $1 AND discriminator = $2",
            username,
            discriminator as i16,
        )
    }

    /// Fetches the client user from the database.
    ///
    /// # Errors
    /// * If an error occurs with fetching the client user.
    async fn fetch_client_user(&self, id: u64) -> sqlx::Result<Option<ClientUser>> {
        let result = sqlx::query!("SELECT * FROM users WHERE id = $1", id as i64)
            .fetch_optional(get_pool())
            .await?
            .map(|r| ClientUser {
                user: construct_user!(r),
                email: r.email,
                relationships: vec![],
            });

        Ok(result)
    }
}

impl<T> UserDbExt for T where T: for<'a> DbExt<'a> {}
