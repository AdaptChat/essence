use crate::cache;
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
    #[cfg(feature = "auth")]
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
        token: impl AsRef<str> + Send + Sync,
    ) -> crate::Result<Option<(u64, UserFlags)>> {
        if let Some(cached) = cache::user_info_for_token(token.as_ref()).await? {
            return Ok(Some(cached));
        }

        if let Some(out @ (user_id, flags)) = sqlx::query!(
            "SELECT id, flags FROM users WHERE id = (SELECT user_id FROM tokens WHERE token = $1)",
            token.as_ref(),
        )
        .fetch_optional(self.executor())
        .await?
        .map(|r| (r.id as u64, UserFlags::from_bits_truncate(r.flags as u32)))
        {
            let token = token.as_ref();
            cache::cache_token(token, user_id, flags).await?;
            Ok(Some(out))
        } else {
            Ok(None)
        }
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
    async fn delete_all_tokens(&mut self, user_id: u64) -> crate::Result<()> {
        sqlx::query!("DELETE FROM tokens WHERE user_id = $1", user_id as i64)
            .execute(self.transaction())
            .await?;

        cache::invalidate_tokens_for(user_id).await?;
        Ok(())
    }

    /// Fetches all push notification registration keys associated with the given user ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the keys.
    /// * If the user is not found.
    /// * If the user is a bot account.
    async fn fetch_push_keys(&self, user_id: u64) -> crate::Result<Vec<String>> {
        let rows = sqlx::query!(
            "SELECT registration_key AS key FROM push_registration_keys WHERE user_id = $1",
            user_id as i64,
        )
        .fetch_all(self.executor())
        .await?;

        Ok(rows.into_iter().map(|r| r.key).collect())
    }

    /// Fetches the ID of the user associated with the push notification  given registration key.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
    /// returned.
    async fn fetch_user_id_by_push_key(
        &self,
        key: impl AsRef<str> + Send,
    ) -> crate::Result<Option<u64>> {
        let user_id = sqlx::query!(
            "SELECT user_id FROM push_registration_keys WHERE registration_key = $1",
            key.as_ref(),
        )
        .fetch_optional(self.executor())
        .await?
        .map(|r| r.user_id as u64);

        Ok(user_id)
    }

    /// Inserts a new push notification registration key for the given user ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with inserting the key.
    /// * If the user is a bot account.
    async fn insert_push_key(
        &mut self,
        user_id: u64,
        key: impl AsRef<str> + Send,
    ) -> crate::Result<()> {
        sqlx::query!(
            "INSERT INTO push_registration_keys (user_id, registration_key) VALUES ($1, $2)",
            user_id as i64,
            key.as_ref(),
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Deletes all push notification registration keys associated with the given user ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the keys.
    /// * If the user is a bot account.
    async fn delete_push_keys(&mut self, user_id: u64) -> crate::Result<()> {
        sqlx::query!(
            "DELETE FROM push_registration_keys WHERE user_id = $1",
            user_id as i64,
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Deletes a single push notification registration key.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the key.
    async fn delete_push_key(&mut self, key: impl AsRef<str> + Send) -> crate::Result<()> {
        sqlx::query!(
            "DELETE FROM push_registration_keys WHERE registration_key = $1",
            key.as_ref(),
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }
}

impl<'t, T> AuthDbExt<'t> for T where T: DbExt<'t> {}
