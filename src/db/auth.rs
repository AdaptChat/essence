use crate::db::DbExt;
use crate::models::UserFlags;

#[async_trait::async_trait]
pub trait AuthDbExt<'t>: DbExt<'t> {
    /// Fetches the password hash for the given user ID and verifies it against the given password.
    ///
    /// # Note
    /// This assumes that the user is not a bot account and has a password that is not NULL.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user.
    /// * If the user is not found.
    async fn verify_password(&self, user_id: u64, password: String) -> crate::Result<bool> {
        let hashed: String = sqlx::query!(
            r#"SELECT password AS "password!" FROM users WHERE id = $1"#,
            user_id as i64,
        )
        .fetch_one(self.executor())
        .await?
        .password;

        Ok(crate::auth::verify_password(password, hashed).await?)
    }

    /// Fetches a user token from the database with the given user ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user token. If the user token is not found,
    /// `Ok(None)` is returned.
    async fn fetch_token(&self, user_id: u64) -> sqlx::Result<Option<String>> {
        sqlx::query!(
            "SELECT token FROM tokens WHERE user_id = $1",
            user_id as i64
        )
        .fetch_optional(self.executor())
        .await
        .map(|r| r.map(|r| r.token))
    }

    /// Resolves a user ID and their user flags from a token. Returns `(user_id, flags)`.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user token. If the user token is not found,
    /// `Ok(None)` is returned.
    async fn fetch_user_info_by_token(
        &self,
        token: impl AsRef<str> + Send,
    ) -> sqlx::Result<Option<(u64, UserFlags)>> {
        sqlx::query!(
            "SELECT id, flags FROM users WHERE id = (SELECT user_id FROM tokens WHERE token = $1)",
            token.as_ref(),
        )
        .fetch_optional(self.executor())
        .await
        .map(|r| r.map(|r| (r.id as u64, UserFlags::from_bits_truncate(r.flags as u32))))
    }

    /// Creates a new token for the given user ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with creating the token.
    async fn create_token(
        &mut self,
        user_id: u64,
        token: impl AsRef<str> + Send,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO tokens (user_id, token) VALUES ($1, $2)",
            user_id as i64,
            token.as_ref(),
        )
        .execute(self.transaction())
        .await
        .map(|_| ())
    }

    /// Deletes all tokens associated with the given user ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the tokens.
    async fn delete_all_tokens(&mut self, user_id: u64) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM tokens WHERE user_id = $1", user_id as i64)
            .execute(self.transaction())
            .await
            .map(|_| ())
    }
}

impl<'t, T> AuthDbExt<'t> for T where T: DbExt<'t> {}
