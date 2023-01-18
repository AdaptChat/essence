use crate::db::DbExt;
use crate::http::message::{CreateMessagePayload, EditMessagePayload, MessageHistoryQuery};
#[allow(unused_imports)]
use crate::models::{Embed, Message, MessageFlags, MessageInfo};
use crate::Error;

macro_rules! construct_message {
    ($data:ident) => {{
        Message {
            id: $data.id as _,
            revision_id: ($data.revision_id != 0).then_some($data.revision_id as _),
            channel_id: $data.channel_id as _,
            author_id: $data.author_id.map(|id| id as _),
            author: None,
            kind: match &*$data.r#type {
                "join" => MessageInfo::Join {
                    user_id: $data.metadata_user_id.unwrap_or_default() as _,
                },
                "leave" => MessageInfo::Leave {
                    user_id: $data.metadata_user_id.unwrap_or_default() as _,
                },
                "pin" => MessageInfo::Pin {
                    pinned_message_id: $data.metadata_pinned_message_id.unwrap_or_default() as _,
                    pinned_by: $data.metadata_pinned_by.unwrap_or_default() as _,
                },
                _ => MessageInfo::Default,
            },
            content: $data.content,
            embeds: $data.embeds_ser.0,
            attachments: Vec::with_capacity(0),
            flags: MessageFlags::from_bits_truncate($data.flags as _),
            stars: $data.stars as _,
            nonce: None,
        }
    }};
}

#[async_trait::async_trait]
pub trait MessageDbExt<'t>: DbExt<'t> {
    /// Fetches a message from the database with the given ID in the given channel.
    ///
    /// # Errors
    /// * If an error occurs with fetching the message. If the message is not found, `Ok(None)` is
    /// returned.
    /// * If an error occurs with fetching the reactions for the message.
    /// * If an error occurs with fetching the attachments for the message.
    async fn fetch_message(
        &self,
        channel_id: u64,
        message_id: u64,
    ) -> crate::Result<Option<Message>> {
        let message = sqlx::query!(
            r#"SELECT
                messages.*,
                embeds AS "embeds_ser: sqlx::types::Json<Vec<Embed>>"
            FROM
                messages
            WHERE
                channel_id = $1
            AND
                id = $2
            AND
                revision_id = 0
            "#,
            channel_id as i64,
            message_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|m| construct_message!(m));

        Ok(message)
    }

    /// Fetches message history from a channel with the given query.
    ///
    /// # Errors
    /// * If an error occurs with fetching the messages.
    async fn fetch_message_history(
        &self,
        channel_id: u64,
        query: MessageHistoryQuery,
    ) -> crate::Result<Vec<Message>> {
        macro_rules! query {
            ($direction:literal) => {{
                sqlx::query!(
                    r#"SELECT
                        messages.*,
                        embeds AS "embeds_ser: sqlx::types::Json<Vec<Embed>>"
                    FROM
                        messages
                    WHERE
                        channel_id = $1
                    AND
                        revision_id = 0
                    AND
                        ($2::BIGINT IS NULL OR id < $2)
                    AND
                        ($3::BIGINT IS NULL OR id > $3)
                    AND
                        ($4::BIGINT IS NULL OR author_id = $4)
                    ORDER BY id "#
                        + $direction
                        + " LIMIT $5",
                    channel_id as i64,
                    query.before.map(|id| id as i64),
                    query.after.map(|id| id as i64),
                    query.user_id.map(|id| id as i64),
                    query.limit as i64,
                )
                .fetch_all(self.executor())
                .await?
                .into_iter()
                .map(|m| construct_message!(m))
                .collect::<Vec<_>>()
            }};
        }
        Ok(if query.oldest_first {
            query!("ASC")
        } else {
            query!("DESC")
        })
    }

    /// Sends a message in the given channel.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs registering the message in the database.
    async fn create_message(
        &mut self,
        channel_id: u64,
        message_id: u64,
        user_id: u64,
        payload: CreateMessagePayload,
    ) -> crate::Result<Message> {
        let embeds =
            serde_json::to_value(payload.embeds.clone()).map_err(|err| Error::InternalError {
                what: Some("embed serialization".to_string()),
                message: err.to_string(),
                debug: Some(format!("{err:?}")),
            })?;

        sqlx::query!(
            "INSERT INTO messages (id, channel_id, author_id, content, embeds)
             VALUES ($1, $2, $3, $4, $5::JSONB)",
            message_id as i64,
            channel_id as i64,
            user_id as i64,
            payload.content,
            embeds,
        )
        .execute(self.transaction())
        .await?;

        Ok(Message {
            id: message_id,
            revision_id: None,
            channel_id,
            author_id: Some(user_id),
            author: None,
            kind: MessageInfo::Default,
            content: payload.content,
            embeds: payload.embeds,
            attachments: Vec::new(),
            flags: MessageFlags::empty(),
            stars: 0,
            nonce: None,
        })
    }

    /// Edits a message in the given channel. This turns the current message into a revision of the
    /// message, and creates a new message with the new data.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with fetching the message.
    async fn edit_message(
        &mut self,
        channel_id: u64,
        message_id: u64,
        revision_id: u64,
        payload: EditMessagePayload,
    ) -> crate::Result<Message> {
        let message = sqlx::query!(
            r#"
                UPDATE messages SET revision_id = $1
                WHERE id = $2 AND channel_id = $3 AND revision_id = 0
                RETURNING *, embeds AS "embeds_ser: sqlx::types::Json<Vec<Embed>>"
            "#,
            revision_id as i64,
            message_id as i64,
            channel_id as i64,
        )
        .fetch_one(self.transaction())
        .await?;

        let message = construct_message!(message);
        let payload = CreateMessagePayload {
            content: payload.content.into_option_or_if_absent(message.content),
            embeds: payload
                .embeds
                .into_option_or_if_absent(Some(message.embeds))
                .unwrap_or_default(),
            nonce: None,
        };

        self.create_message(channel_id, message_id, message.author_id.unwrap(), payload)
            .await
    }

    /// Deletes a message with the given channel and message ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    ///
    /// # Errors
    /// * If an error occurs with deleting the message.
    async fn delete_message(&self, channel_id: u64, message_id: u64) -> crate::Result<()> {
        sqlx::query!(
            "DELETE FROM messages WHERE id = $1 AND channel_id = $2",
            message_id as i64,
            channel_id as i64,
        )
        .execute(self.executor())
        .await?;

        Ok(())
    }
}

impl<'t, T> MessageDbExt<'t> for T where T: DbExt<'t> {}
