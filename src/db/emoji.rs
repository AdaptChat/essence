use crate::models::Emoji;

use super::DbExt;

#[async_trait::async_trait]
pub trait EmojiDbExt<'t>: DbExt<'t> {
    /// Fetch all custom emojis that belongs to `guild_id`
    async fn fetch_all_emojis_in_guild(&self, guild_id: u64) -> crate::Result<Vec<Emoji>> {
        Ok(
            sqlx::query!("SELECT * FROM emojis WHERE guild_id = $1", guild_id as i64)
                .fetch_all(self.executor())
                .await?
                .into_iter()
                .map(|r| Emoji {
                    id: r.id as u64,
                    guild_id,
                    name: r.name,
                    created_by: r.created_by as u64,
                })
                .collect::<Vec<Emoji>>(),
        )
    }

    /// Fetch emoji with id.
    ///
    /// Returns `None` if not found.
    async fn fetch_emoji(&self, id: u64) -> crate::Result<Option<Emoji>> {
        Ok(
            sqlx::query!("SELECT * FROM emojis WHERE id = $1", id as i64)
                .fetch_optional(self.executor())
                .await?
                .map(|r| Emoji {
                    id: r.id as u64,
                    guild_id: r.guild_id as u64,
                    name: r.name,
                    created_by: r.created_by as u64,
                }),
        )
    }

    /// Create a new emoji with the given parameters.
    ///
    /// Returns the new `Emoji`.
    async fn create_emoji(
        &mut self,
        id: u64,
        guild_id: u64,
        name: impl AsRef<str> + Send,
        created_by: u64,
    ) -> crate::Result<Emoji> {
        sqlx::query!(
            "INSERT INTO emojis VALUES ($1, $2, $3, $4)",
            id as i64,
            guild_id as i64,
            name.as_ref(),
            created_by as i64
        )
        .execute(self.transaction())
        .await?;

        Ok(Emoji {
            id,
            guild_id,
            name: name.as_ref().to_string(),
            created_by,
        })
    }

    /// Edit emoji with the given id.
    ///
    /// The only editable property is `name`.
    async fn edit_emoji(&mut self, id: u64, name: impl AsRef<str> + Send) -> crate::Result<Emoji> {
        let r = sqlx::query!(
            "UPDATE emojis SET name = $1 WHERE id = $2 RETURNING *",
            name.as_ref(),
            id as i64
        )
        .fetch_one(self.transaction())
        .await?;

        Ok(Emoji {
            id: r.id as u64,
            name: r.name,
            guild_id: r.guild_id as u64,
            created_by: r.created_by as u64,
        })
    }

    /// Deletes an emoji with the given id.
    async fn delete_emoji(&mut self, id: u64) -> crate::Result<()> {
        sqlx::query!("DELETE FROM emojis WHERE id = $1", id as i64)
            .execute(self.transaction())
            .await?;

        Ok(())
    }
}

impl<'t, T> EmojiDbExt<'t> for T where T: DbExt<'t> {}
