use super::DbExt;
use crate::{
    db::get_pool,
    http::user::EditUserPayload,
    models::user::{ClientUser, User, UserFlags},
    Error, NotFoundExt,
};

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

macro_rules! fetch_client_user {
    ($self:ident, $query:literal, $($arg:expr),* $(,)?) => {{
        let result = sqlx::query!($query, $($arg),*)
            .fetch_optional($self.executor())
            .await?
            .map(|r| ClientUser {
                user: construct_user!(r),
                email: r.email,
                password: r.password,
                relationships: vec![],
            });

        Ok(result)
    }};
}

#[async_trait::async_trait]
pub trait UserDbExt<'t>: DbExt<'t> {
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
    async fn fetch_client_user_by_id(&self, id: u64) -> sqlx::Result<Option<ClientUser>> {
        fetch_client_user!(self, "SELECT * FROM users WHERE id = $1", id as i64)
    }

    /// Fetches the client user from the database by email.
    ///
    /// # Errors
    /// * If an error occurs with fetching the client user. If the user is not found, `Ok(None)` is
    /// returned.
    async fn fetch_client_user_by_email(
        &self,
        email: impl AsRef<str> + Send,
    ) -> sqlx::Result<Option<ClientUser>> {
        fetch_client_user!(self, "SELECT * FROM users WHERE email = $1", email.as_ref())
    }

    /// Returns `true` if the given email is taken.
    ///
    /// # Errors
    /// * If an error occurs with the database.
    async fn is_email_taken(&self, email: impl AsRef<str> + Send) -> sqlx::Result<bool> {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)",
            email.as_ref()
        )
        .fetch_one(self.executor())
        .await?
        .exists;

        Ok(result.unwrap())
    }

    /// Registers a user in the database with the given payload. No validation is done, they must
    /// be done before calling this method. Hashing of the password is done here.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with registering the user.
    #[cfg(feature = "auth")]
    async fn register_user(
        &mut self,
        id: u64,
        username: impl AsRef<str> + Send,
        email: impl AsRef<str> + Send,
        password: impl AsRef<str> + Send,
    ) -> crate::Result<()> {
        let password = password.as_ref();
        let hashed = crate::auth::hash_password(password).await?;

        let discriminator = sqlx::query!(
            "INSERT INTO
                users (id, username, email, password)
            VALUES
                ($1, $2, $3, $4)
            RETURNING
                discriminator",
            id as i64,
            username.as_ref(),
            email.as_ref(),
            hashed,
        )
        .fetch_optional(self.transaction())
        .await?;

        if discriminator.is_none() {
            return Err(Error::AlreadyTaken {
                what: "username",
                message: "Username is already taken".to_string(),
            });
        }

        Ok(())
    }

    /// Edits a user in the database with the given payload. No validation is done, they must
    /// be done before calling this method.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with editing the user.
    /// * If the user is not found.
    /// * If the user is trying to change their username to one that is already taken.
    async fn edit_user(&mut self, id: u64, payload: EditUserPayload) -> crate::Result<User> {
        let mut user = get_pool()
            .fetch_user_by_id(id)
            .await?
            .ok_or_not_found("user", "user not found")?;

        if let Some(username) = payload.username {
            let discriminator = if sqlx::query!(
                "SELECT discriminator FROM users WHERE username = $1 AND discriminator = $2",
                username,
                user.discriminator as i16,
            )
            .fetch_optional(get_pool())
            .await?
            .is_none()
            {
                user.discriminator as i16
            } else {
                let discriminator =
                    sqlx::query!("SELECT generate_discriminator($1) AS out", username)
                        .fetch_one(get_pool())
                        .await?
                        .out;

                discriminator.ok_or_else(|| Error::AlreadyTaken {
                    what: "username",
                    message: "Username is already taken".to_string(),
                })?
            };

            sqlx::query!(
                "UPDATE users SET username = $1, discriminator = $2 WHERE id = $3",
                username,
                discriminator,
                id as i64,
            )
            .execute(self.transaction())
            .await?;

            user.username = username;
            user.discriminator = discriminator as u16;
        }

        user.avatar = payload.avatar.into_option_or_if_absent(user.avatar);
        user.banner = payload.banner.into_option_or_if_absent(user.banner);
        user.bio = payload.bio.into_option_or_if_absent(user.bio);

        sqlx::query!(
            r#"UPDATE users SET avatar = $1, banner = $2, bio = $3 WHERE id = $4"#,
            user.avatar,
            user.banner,
            user.bio,
            id as i64,
        )
        .execute(self.transaction())
        .await?;

        Ok(user)
    }

    /// Deletes a user from the database.
    ///
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the user.
    async fn delete_user(&mut self, id: u64) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM users WHERE id = $1", id as i64)
            .execute(self.transaction())
            .await?;

        Ok(())
    }
}

impl<'t, T> UserDbExt<'t> for T where T: DbExt<'t> {}
