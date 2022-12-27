use super::DbExt;

#[async_trait::async_trait]
pub trait AuthDbExt: for<'a> DbExt<'a> {
    /// Creates a new token for the given user ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with creating the token.
    #[inline]
    async fn create_token(
        &mut self,
        user_id: u64,
        token: impl AsRef<str> + Send,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            r#"INSERT INTO tokens (user_id, token) VALUES ($1, $2)"#,
            user_id as i64,
            token.as_ref(),
        )
        .execute(self.transaction())
        .await
        .map(|_| ())
    }
}

impl<T> AuthDbExt for T where T: for<'a> DbExt<'a> {}
