use crate::db::{get_pool, GuildDbExt};
#[allow(unused_imports)]
use crate::models::Embed;
use crate::{
    db::DbExt,
    http::message::{CreateMessagePayload, EditMessagePayload, MessageHistoryQuery},
    models::{Attachment, Guild, Message, MessageFlags, MessageInfo, Permissions},
    snowflake::extract_mentions,
    Error, NotFoundExt,
};
use itertools::Itertools;
use std::collections::HashMap;

macro_rules! construct_message {
    ($data:ident) => {{
        Message {
            id: $data.id as _,
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
            mentions: $data.mentions.into_iter().map(|id| id as _).collect(),
            edited_at: $data.edited_at,
        }
    }};
}

#[async_trait::async_trait]
pub trait MessageDbExt<'t>: DbExt<'t> {
    /// Fetches quick metadata about a message. Returns `author_id`.
    ///
    /// # Errors
    /// * If an error occurs inspecting the message
    async fn inspect_message(&self, message_id: u64) -> crate::Result<Option<Option<u64>>> {
        let data = sqlx::query!(
            "SELECT author_id FROM messages WHERE id = $1",
            message_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|data| data.author_id.map(|id| id as _));

        Ok(data)
    }

    /// Fetches the attachments for a message.
    ///
    /// # Errors
    /// * If an error occurs fetching the attachments.
    /// * If an error occurs fetching the message.
    /// * If the message is not found.
    async fn fetch_message_attachments(&self, message_id: u64) -> crate::Result<Vec<Attachment>> {
        Ok(sqlx::query!(
            r"SELECT * FROM attachments WHERE message_id = $1",
            message_id as i64
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|attachment| Attachment {
            id: attachment.id as _,
            alt: attachment.alt,
            filename: attachment.filename,
            size: attachment.size as _,
        })
        .collect())
    }

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
        let mut message = sqlx::query!(
            r#"SELECT
                messages.*,
                embeds AS "embeds_ser: sqlx::types::Json<Vec<Embed>>"
            FROM
                messages
            WHERE
                channel_id = $1
            AND
                id = $2
            "#,
            channel_id as i64,
            message_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|m| construct_message!(m));

        if let Some(message) = message.as_mut() {
            message.attachments = self.fetch_message_attachments(message_id).await?;
        }
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
            ($base:literal, $direction:literal) => {{
                sqlx::query!(
                    $base + r#" WHERE
                        m.channel_id = $1
                    AND
                        ($2::BIGINT IS NULL OR m.id < $2)
                    AND
                        ($3::BIGINT IS NULL OR m.id > $3)
                    AND
                        ($4::BIGINT IS NULL OR m.author_id = $4)
                    ORDER BY m.id "#
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
            }};
            (@messages $direction:literal) => {{
                query!(
                    r#"SELECT
                        m.*,
                        embeds AS "embeds_ser: sqlx::types::Json<Vec<Embed>>"
                    FROM
                        messages m"#,
                    $direction
                )
                .map(|m| construct_message!(m))
                .collect::<Vec<_>>()
            }};
            (@attachments $direction:literal) => {{
                query!(
                    r"SELECT a.*
                    FROM attachments a
                    INNER JOIN messages m ON a.message_id = m.id",
                    $direction
                )
                .map(|attachment| (
                    attachment.message_id as u64,
                    Attachment {
                        id: attachment.id as _,
                        alt: attachment.alt,
                        filename: attachment.filename,
                        size: attachment.size as _,
                    },
                ))
                .into_group_map()
            }};
            ($direction:literal) => {{
                let mut attachments: HashMap<u64, Vec<_>> = query!(@attachments $direction);
                let mut messages = query!(@messages $direction);
                for message in &mut messages {
                    if let Some(attachments) = attachments.remove(&message.id) {
                        message.attachments = attachments;
                    }
                }
                messages
            }};
        }
        Ok(if query.oldest_first {
            query!("ASC")
        } else {
            query!("DESC")
        })
    }

    /// Fetches a list of messages by ID from the database in bulk.
    ///
    /// # Note
    /// This returns messages from newest to oldest, despite the order of the IDs.
    ///
    /// # Errors
    /// * If an error occurs with fetching the messages.
    async fn bulk_fetch_messages(
        &self,
        channel_ids: Option<&[i64]>,
        message_ids: &[u64],
        limit: Option<u64>,
    ) -> crate::Result<Vec<Message>> {
        let message_ids = message_ids.iter().map(|id| *id as i64).collect_vec();
        let mut messages = sqlx::query!(
            r#"SELECT
                messages.*,
                embeds AS "embeds_ser: sqlx::types::Json<Vec<Embed>>"
            FROM
                messages
            WHERE
                id = ANY($1::BIGINT[])
            AND
                ($2::BIGINT[] IS NULL OR channel_id = ANY($2::BIGINT[]))
            ORDER BY id DESC
            LIMIT $3"#,
            &message_ids,
            channel_ids,
            limit.map(|limit| limit as i64),
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|m| construct_message!(m))
        .collect_vec();

        let mut attachments = sqlx::query!(
            r#"SELECT a.* FROM attachments a
            INNER JOIN
                messages m ON a.message_id = m.id
            WHERE
                m.id = ANY($1::BIGINT[])
            AND
                ($2::BIGINT[] IS NULL OR m.channel_id = ANY($2::BIGINT[]))"#,
            &message_ids,
            channel_ids,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|attachment| {
            (
                attachment.message_id as u64,
                Attachment {
                    id: attachment.id as _,
                    alt: attachment.alt,
                    filename: attachment.filename,
                    size: attachment.size as _,
                },
            )
        })
        .into_group_map();

        for message in &mut messages {
            if let Some(attachments) = attachments.remove(&message.id) {
                message.attachments = attachments;
            }
        }
        Ok(messages)
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

        let mut mentions = payload
            .content
            .as_deref()
            .map(extract_mentions)
            .unwrap_or_default();
        mentions.sort_unstable();
        mentions.dedup();

        let mentions_i64 = mentions.iter().map(|m| *m as i64).collect_vec();

        sqlx::query!(
            "INSERT INTO messages (id, channel_id, author_id, content, embeds, mentions)
             VALUES ($1, $2, $3, $4, $5::JSONB, $6::BIGINT[])",
            message_id as i64,
            channel_id as i64,
            user_id as i64,
            payload.content,
            embeds,
            &mentions_i64,
        )
        .execute(self.transaction())
        .await?;

        Ok(Message {
            id: message_id,
            channel_id,
            author_id: Some(user_id),
            author: None,
            kind: MessageInfo::Default,
            content: payload.content,
            embeds: payload.embeds,
            attachments: Vec::new(),
            flags: MessageFlags::empty(),
            stars: 0,
            mentions,
            edited_at: None,
        })
    }

    /// Create a new attachment.
    ///
    /// # Note
    /// This method uses transactions to ensure consistency with [`create_message`]
    async fn create_attachment(
        &mut self,
        message_id: u64,
        attachment: Attachment,
    ) -> crate::Result<()> {
        sqlx::query!(
            "INSERT INTO attachments VALUES ($1, $2, $3, $4, $5)",
            attachment.id,
            message_id as i64,
            attachment.filename,
            attachment.size as i64,
            attachment.alt,
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Sends a system message in the given channel.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs registering the message in the database.
    async fn send_system_message(
        &mut self,
        channel_id: u64,
        message_id: u64,
        info: MessageInfo,
    ) -> crate::Result<Message> {
        // SAFETY: mem::zeroed is Option::None
        let (mut md_target_id, mut md_pinned_by, mut md_pinned_message_id) =
            unsafe { std::mem::zeroed() };

        match info {
            MessageInfo::Default => {
                return Err(Error::custom_for(
                    "system message",
                    "Cannot send a default message as a system message",
                ));
            }
            MessageInfo::Join { user_id } | MessageInfo::Leave { user_id } => {
                md_target_id = Some(user_id as i64);
            }
            MessageInfo::Pin {
                pinned_by,
                pinned_message_id,
            } => {
                md_pinned_by = Some(pinned_by as i64);
                md_pinned_message_id = Some(pinned_message_id as i64);
            }
        };

        sqlx::query!(
            "INSERT INTO messages (
                id, channel_id,
                metadata_user_id, metadata_pinned_by, metadata_pinned_message_id
            )
            VALUES ($1, $2, $3, $4, $5)
            ",
            message_id as i64,
            channel_id as i64,
            md_target_id,
            md_pinned_by,
            md_pinned_message_id,
        )
        .execute(self.transaction())
        .await?;

        Ok(Message {
            id: message_id,
            channel_id,
            author_id: None,
            author: None,
            kind: info,
            content: None,
            embeds: Vec::new(),
            attachments: Vec::new(),
            flags: MessageFlags::empty(),
            stars: 0,
            mentions: Vec::new(),
            edited_at: None,
        })
    }

    /// Modifies the flags of a message in the given channel.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs fetching the message.
    async fn edit_message_flags(
        &mut self,
        channel_id: u64,
        message_id: u64,
        enable: MessageFlags,
        disable: MessageFlags,
    ) -> crate::Result<MessageFlags> {
        let message = sqlx::query!(
            r"UPDATE messages SET flags = flags | $1::INT & ~$2::INT
                WHERE id = $3 AND channel_id = $4
                RETURNING flags",
            enable.bits() as i32,
            disable.bits() as i32,
            message_id as i64,
            channel_id as i64,
        )
        .fetch_optional(self.transaction())
        .await?
        .ok_or_not_found("message", format!("Message with ID {message_id} not found"))?;

        Ok(MessageFlags::from_bits_truncate(message.flags as _))
    }

    /// Edits a message in the given channel. This turns the current message into a revision of the
    /// message, and creates a new message with the new data.
    ///
    /// If provided, ``user_id``` will be checked against the author of the message and throw a
    /// forbidden error if they do not match.
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
        user_id: Option<u64>,
        payload: EditMessagePayload,
    ) -> crate::Result<(Message, Message)> {
        let old = get_pool()
            .fetch_message(channel_id, message_id)
            .await?
            .ok_or_not_found("message", format!("Message with ID {message_id} not found"))?;

        if let Some(user_id) = user_id {
            if old.author_id != Some(user_id) {
                return Err(Error::NotMessageAuthor {
                    message_id,
                    message: "You must be the author of this message to edit it.".to_string(),
                });
            }
        }

        let content = payload
            .content
            .into_option_or_if_absent_then(|| old.content.clone());
        let embeds = payload
            .embeds
            .into_option_or_if_absent_then(|| Some(old.embeds.clone()))
            .unwrap_or_default();
        let embeds = serde_json::to_value(embeds).map_err(|err| Error::InternalError {
            what: Some("embed serialization".to_string()),
            message: err.to_string(),
            debug: Some(format!("{err:?}")),
        })?;

        let mentions = content.as_deref().map(extract_mentions).unwrap_or_default();
        let mentions_i64 = mentions.iter().map(|m| *m as i64).collect_vec();
        let new = sqlx::query!(
            r#"UPDATE messages
            SET 
                content = $1,
                embeds = $2::JSONB, 
                edited_at = NOW(), 
                mentions = $3::BIGINT[]
            WHERE 
                id = $4 AND channel_id = $5
            RETURNING *, embeds AS "embeds_ser: sqlx::types::Json<Vec<Embed>>""#,
            content,
            embeds,
            &mentions_i64,
            message_id as i64,
            channel_id as i64,
        )
        .fetch_one(self.transaction())
        .await?;

        Ok((old, construct_message!(new)))
    }

    /// Deletes a message with the given channel and message ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
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

    /// Bulk deletes messages with the given message IDs, optionally limiting to a channel.
    ///
    /// # Notes
    /// * This method uses transactions, on the event of an ``Err`` the transaction must be properly
    ///   rolled back, and the transaction must be committed to save the changes.
    /// * This method does not check if the message IDs are valid and will silently ignore invalid
    ///   IDs.
    ///
    /// # Errors
    /// * If an error occurs with deleting the messages.
    async fn bulk_delete_messages(
        &mut self,
        channel_id: Option<u64>,
        message_ids: &[u64],
    ) -> crate::Result<()> {
        let message_ids = message_ids.iter().map(|id| *id as i64).collect_vec();
        sqlx::query!(
            "DELETE FROM messages WHERE id = ANY($1::BIGINT[]) AND ($2::BIGINT IS NULL OR channel_id = $2)",
            &message_ids,
            channel_id.map(|id| id as i64),
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Fetches the IDs of all viewable channels by the user with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channels.
    async fn fetch_observable_channel_ids(
        &self,
        user_id: u64,
        guilds: &[Guild],
    ) -> crate::Result<Vec<u64>> {
        let mut channel_ids = Vec::new();
        for (guild, channels) in guilds
            .iter()
            .filter_map(|g| g.channels.as_ref().map(|c| (g, c)))
        {
            for channel in channels {
                if self
                    .fetch_member_permissions(guild.partial.id, user_id, Some(channel.id))
                    .await?
                    .contains(Permissions::VIEW_CHANNEL | Permissions::VIEW_MESSAGE_HISTORY)
                {
                    channel_ids.push(channel.id);
                }
            }
        }
        Ok(channel_ids)
    }

    /// Fetches the IDs of all unacked messages that mention the user with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the messages.
    async fn fetch_mentioned_messages(
        &self,
        user_id: u64,
        guilds: &[Guild],
    ) -> crate::Result<HashMap<u64, Vec<u64>>> {
        let channel_ids = self
            .fetch_observable_channel_ids(user_id, guilds)
            .await?
            .into_iter()
            .map(|id| id as i64)
            .collect_vec();

        let res = sqlx::query!(
            r"SELECT m.id, m.channel_id FROM messages m
            INNER JOIN channels c ON m.channel_id = c.id
            LEFT JOIN channel_acks a ON m.channel_id = a.channel_id AND a.user_id = $1
            WHERE
                m.channel_id = ANY($2::BIGINT[])
            AND (
                a.last_message_id IS NULL
                OR m.id > a.last_message_id
            )
            AND (
                $1 = ANY(m.mentions)
                OR c.guild_id = ANY(m.mentions)
                OR m.mentions && (
                    SELECT array_agg(role_id) FROM role_data 
                    WHERE guild_id = c.guild_id AND user_id = $1
                )
            )",
            user_id as i64,
            &channel_ids,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|row| (row.channel_id as u64, row.id as u64))
        .into_group_map();

        Ok(res)
    }
}

impl<'t, T> MessageDbExt<'t> for T where T: DbExt<'t> {}
