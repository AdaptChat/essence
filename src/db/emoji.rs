use super::DbExt;
use crate::models::{CustomEmoji, PartialEmoji, Reaction};

macro_rules! construct_emoji {
    ($data:expr) => {
        CustomEmoji {
            id: $data.id as u64,
            guild_id: $data.guild_id as u64,
            name: $data.name,
            created_by: $data.created_by.map(|id| id as u64),
        }
    };
}

macro_rules! construct_reaction {
    ($message_id:expr, $data:expr) => {
        Reaction {
            message_id: $message_id,
            emoji: PartialEmoji {
                id: $data.emoji_id.map(|id| id as u64),
                name: $data.emoji_name,
            },
            user_ids: $data
                .user_ids
                .map_or_else(Vec::new, |u| u.into_iter().map(|id| id as u64).collect()),
            created_at: $data.created_at,
        }
    };
    ($data:expr) => {
        construct_reaction!($data.message_id as u64, $data)
    };
}

pub(crate) use construct_reaction;

#[async_trait::async_trait]
pub trait EmojiDbExt<'t>: DbExt<'t> {
    /// Fetch all custom emojis that belongs to `guild_id`
    async fn fetch_all_emojis_in_guild(&self, guild_id: u64) -> crate::Result<Vec<CustomEmoji>> {
        Ok(
            sqlx::query!("SELECT * FROM emojis WHERE guild_id = $1", guild_id as i64)
                .fetch_all(self.executor())
                .await?
                .into_iter()
                .map(|r| construct_emoji!(r))
                .collect::<Vec<CustomEmoji>>(),
        )
    }

    /// Fetch emoji with id.
    ///
    /// Returns `None` if not found.
    async fn fetch_emoji(&self, id: u64) -> crate::Result<Option<CustomEmoji>> {
        Ok(
            sqlx::query!("SELECT * FROM emojis WHERE id = $1", id as i64)
                .fetch_optional(self.executor())
                .await?
                .map(|r| construct_emoji!(r)),
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
    ) -> crate::Result<CustomEmoji> {
        sqlx::query!(
            "INSERT INTO emojis VALUES ($1, $2, $3, $4)",
            id as i64,
            guild_id as i64,
            name.as_ref(),
            created_by as i64
        )
        .execute(self.transaction())
        .await?;

        Ok(CustomEmoji {
            id,
            guild_id,
            name: name.as_ref().to_string(),
            created_by: Some(created_by),
        })
    }

    /// Edit emoji with the given id.
    ///
    /// The only editable property is `name`.
    async fn edit_emoji(
        &mut self,
        id: u64,
        name: impl AsRef<str> + Send,
    ) -> crate::Result<CustomEmoji> {
        let r = sqlx::query!(
            "UPDATE emojis SET name = $1 WHERE id = $2 RETURNING *",
            name.as_ref(),
            id as i64
        )
        .fetch_one(self.transaction())
        .await?;

        Ok(construct_emoji!(r))
    }

    /// Deletes an emoji with the given id.
    async fn delete_emoji(&mut self, id: u64) -> crate::Result<()> {
        sqlx::query!("DELETE FROM emojis WHERE id = $1", id as i64)
            .execute(self.transaction())
            .await?;

        Ok(())
    }

    /// Returns whether the given emoji is already an existing reaction on the given message.
    async fn reaction_exists(&self, message_id: u64, emoji: &PartialEmoji) -> crate::Result<bool> {
        let exists = sqlx::query!(
            "SELECT EXISTS(
                SELECT 1 FROM reactions
                WHERE
                    message_id = $1
                    AND emoji_id IS NOT DISTINCT FROM $2
                    AND emoji_name = $3
            ) AS exists",
            message_id as i64,
            emoji.id.map(|id| id as i64),
            emoji.name,
        )
        .fetch_one(self.executor())
        .await?
        .exists
        .unwrap_or(false);

        Ok(exists)
    }

    /// Fetches all reactions from the message with the given ID.
    async fn fetch_reactions(&self, message_id: u64) -> crate::Result<Vec<Reaction>> {
        let reactions = sqlx::query!(
            r"SELECT
                emoji_id,
                emoji_name,
                array_agg(user_id) AS user_ids,
                array_agg(created_at) AS created_at
            FROM reactions
            WHERE message_id = $1
            GROUP BY (emoji_id, emoji_name)",
            message_id as i64
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| construct_reaction!(message_id, r))
        .collect();

        Ok(reactions)
    }

    /// Adds a reaction to the message with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with adding the reaction.
    /// * If the message is not found.
    async fn add_reaction(
        &mut self,
        message_id: u64,
        user_id: u64,
        emoji: &PartialEmoji,
    ) -> crate::Result<()> {
        sqlx::query!(
            "INSERT INTO reactions (message_id, user_id, emoji_id, emoji_name)
            VALUES ($1, $2, $3, $4)",
            message_id as i64,
            user_id as i64,
            emoji.id.map(|id| id as i64),
            emoji.name,
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Removes a reaction from the message with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with removing the reaction.
    /// * If the message is not found.
    async fn remove_reaction(
        &mut self,
        message_id: u64,
        user_id: u64,
        emoji: &PartialEmoji,
    ) -> crate::Result<()> {
        sqlx::query!(
            "DELETE FROM reactions
            WHERE
                message_id = $1 AND user_id = $2
                AND emoji_id IS NOT DISTINCT FROM $3 AND emoji_name = $4
            ",
            message_id as i64,
            user_id as i64,
            emoji.id.map(|id| id as i64),
            emoji.name,
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Removes all reactions from the message with the given ID, optionally filtering by emoji.
    async fn bulk_remove_reactions(
        &mut self,
        message_id: u64,
        emoji: Option<&PartialEmoji>,
    ) -> crate::Result<()> {
        match emoji {
            Some(emoji) => {
                sqlx::query!(
                    "DELETE FROM reactions
                    WHERE
                        message_id = $1
                        AND emoji_id IS NOT DISTINCT FROM $2
                        AND emoji_name = $3
                    ",
                    message_id as i64,
                    emoji.id.map(|id| id as i64),
                    emoji.name,
                )
                .execute(self.transaction())
                .await?;
            }
            None => {
                sqlx::query!(
                    "DELETE FROM reactions WHERE message_id = $1",
                    message_id as i64
                )
                .execute(self.transaction())
                .await?;
            }
        }

        Ok(())
    }
}

impl<'t, T> EmojiDbExt<'t> for T where T: DbExt<'t> {}
