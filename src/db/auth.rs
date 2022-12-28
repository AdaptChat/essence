use crate::db::DbExt;

pub trait AuthDbExt: for<'a> DbExt<'a> {
    /// Fetches a user token from the database with the given user ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user token. If the user token is not found,
    /// `Ok(None)` is returned.
    async fn fetch_token(&self, user_id: u64) -> sqlx::Result<Option<String>> {
        sqlx::query!(
            "SELECT token FROm tokens WHERE user_id = $1",
            user_id as i64
        )
        .fetch_optional(self.executor())
        .await
        .map(|r| r.map(|r| r.token))
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

impl<T> AuthDbExt for T where T: for<'a> DbExt<'a> {}
