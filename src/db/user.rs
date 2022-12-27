use super::get_pool;
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
    ($query:literal, $($arg:expr),* $(,)?) => {{
        let result = sqlx::query!($query, $($arg),*)
            .fetch_optional(get_pool())
            .await?
            .map(|r| construct_user!(r));

        Ok(result)
    }};
}

/// Fetches a user from the database with the given ID.
///
/// # Errors
/// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
/// returned.
pub async fn fetch_user_by_id(id: u64) -> sqlx::Result<Option<User>> {
    fetch_user!("SELECT * FROM users WHERE id = $1", id as i64)
}

/// Fetches a user from the database with the given username and discriminator.
///
/// # Errors
/// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
/// returned.
pub async fn fetch_user_by_tag(username: &str, discriminator: u16) -> sqlx::Result<Option<User>> {
    fetch_user!(
        "SELECT * FROM users WHERE username = $1 AND discriminator = $2",
        username,
        discriminator as i16,
    )
}

/// Fetches the client user from the database.
///
/// # Errors
/// * If an error occurs with fetching the client user.
pub async fn fetch_client_user(id: u64) -> sqlx::Result<Option<ClientUser>> {
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
