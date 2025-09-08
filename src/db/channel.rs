#[allow(unused_imports)]
use crate::models::Embed;
use crate::{
    Error, Maybe, NotFoundExt,
    cache::{self, ChannelInspection},
    db::{DbExt, GuildDbExt, MessageDbExt, get_pool, message::construct_message},
    http::channel::{
        CreateDmChannelPayload, CreateGuildChannelInfo, CreateGuildChannelPayload,
        EditChannelPayload, EditChannelPositionsPayload,
    },
    models::{
        Channel, ChannelType, DbGradient, DmChannel, DmChannelInfo, ExtendedColor, Guild,
        GuildChannel, GuildChannelInfo, Message, PermissionOverwrite, PermissionPair, Permissions,
        TextBasedGuildChannelInfo,
    },
    ws::UnackedChannel,
};
use futures_util::future::TryJoinAll;
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

macro_rules! query_channels {
    ($where:literal $(, $($args:expr_2021),*)?) => {{
        sqlx::query_as!(
            crate::db::channel::ChannelRecord,
            r#"SELECT
                c.id,
                guild_id,
                c.type AS kind,
                name,
                position,
                parent_id,
                topic,
                icon,
                color,
                gradient AS "gradient: crate::models::DbGradient",
                slowmode,
                nsfw,
                locked,
                user_limit,
                owner_id
            FROM
                channels c
            WHERE
            "# + $where,
            $($($args),*)?
        )
    }};
}

pub(crate) use query_channels;

macro_rules! query_guild_channel_next_position {
    ($(@clause $clause:literal,)? $($args:expr_2021),*) => {{
        sqlx::query!(
            r#"SELECT
                COALESCE(MAX(position) + 1, 0) AS "position!"
            FROM
                channels
            WHERE
                guild_id = $1
            AND
                (parent_id = $2 OR parent_id IS NULL AND $2 IS NULL)
            "# $(+ "AND " + $clause)?,
            $($args),*
        )
    }}
}

pub(crate) struct ChannelRecord {
    pub id: i64,
    pub guild_id: Option<i64>,
    pub kind: String,
    pub name: Option<String>,
    pub position: Option<i16>,
    pub parent_id: Option<i64>,
    pub topic: Option<String>,
    pub icon: Option<String>,
    pub color: Option<i32>,
    pub gradient: Option<DbGradient>,
    pub slowmode: Option<i32>,
    pub nsfw: Option<bool>,
    pub locked: Option<bool>,
    pub user_limit: Option<i16>,
    pub owner_id: Option<i64>,
}

impl ChannelRecord {
    fn extended_color(&self) -> Option<ExtendedColor> {
        ExtendedColor::from_db(self.color, self.gradient.as_ref())
    }

    pub(crate) fn into_guild_channel(
        mut self,
        overwrites: Vec<PermissionOverwrite>,
        last_message: Option<Message>,
    ) -> crate::Result<GuildChannel> {
        let channel_id = self.id as u64;
        let kind = ChannelType::from_str(&self.kind)?;
        let info = match kind {
            _ if kind.is_guild_text_based() => {
                let info = TextBasedGuildChannelInfo {
                    topic: self.topic.take(),
                    nsfw: self.nsfw.unwrap_or_default(),
                    locked: self.locked.unwrap_or_default(),
                    slowmode: self.slowmode.unwrap_or_default() as u32,
                    last_message,
                };

                match kind {
                    ChannelType::Text => GuildChannelInfo::Text(info),
                    ChannelType::Announcement => GuildChannelInfo::Announcement(info),
                    _ => unreachable!(),
                }
            }
            ChannelType::Voice => GuildChannelInfo::Voice {
                user_limit: self.user_limit.unwrap_or_default() as u16,
            },
            ChannelType::Category => GuildChannelInfo::Category,
            _ if kind.is_dm() => unreachable!("This method should not be called for DM channels"),
            _ => unimplemented!(),
        };

        let guild_id = self.guild_id.ok_or_else(|| Error::InternalError {
            what: None,
            message: "Guild channel has no guild ID".to_string(),
            debug: None,
        })? as u64;

        Ok(GuildChannel {
            id: channel_id,
            guild_id,
            color: self.extended_color(),
            icon: self.icon.clone(),
            position: self.position.unwrap_or_default() as u16,
            parent_id: self.parent_id.map(|id| id as u64),
            name: self.name.unwrap_or_default(),
            info,
            overwrites,
        })
    }

    pub(crate) fn into_dm_channel(
        self,
        recipients: Vec<u64>,
        last_message: Option<Message>,
    ) -> crate::Result<DmChannel> {
        let channel_id = self.id as u64;
        let kind = ChannelType::from_str(&self.kind)?;
        let info = match kind {
            ChannelType::Dm => {
                if recipients.len() != 2 {
                    return Err(Error::InternalError {
                        what: None,
                        message: "DM channel has invalid number of recipients".to_string(),
                        debug: None,
                    });
                }
                DmChannelInfo::Dm {
                    recipient_ids: (recipients[0], recipients[1]),
                }
            }
            ChannelType::Group => DmChannelInfo::Group {
                name: self.name.clone().unwrap_or_default(),
                icon: self.icon,
                topic: self.topic,
                owner_id: self.owner_id.unwrap_or_default() as u64,
                recipient_ids: recipients,
            },
            _ if kind.is_guild() => {
                unreachable!("This method should not be called for guild channels")
            }
            _ => unimplemented!(),
        };

        Ok(DmChannel {
            id: channel_id,
            info,
            last_message,
        })
    }
}

#[async_trait::async_trait]
pub trait ChannelDbExt<'t>: DbExt<'t> {
    /// Asserts the given channel ID exists in the given guild.
    async fn assert_channel_in_guild(&self, guild_id: u64, channel_id: u64) -> crate::Result<()> {
        let exists = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1 AND guild_id = $2)",
            channel_id as i64,
            guild_id as i64
        )
        .fetch_one(self.executor())
        .await?
        .exists
        .unwrap_or_default();

        if exists {
            Ok(())
        } else {
            Err(Error::NotFound {
                entity: "channel".to_string(),
                message: format!("Channel with ID {channel_id} not found in this guild"),
            })
        }
    }

    /// Asserts the given channel ID exists in the given guild and is of the given channel type.
    async fn assert_channel_is_type(
        &self,
        guild_id: u64,
        channel_id: u64,
        kind: ChannelType,
    ) -> crate::Result<()> {
        let exists = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1 AND guild_id = $2 AND type = $3)",
            channel_id as i64,
            guild_id as i64,
            kind.name(),
        )
        .fetch_one(self.executor())
        .await?
        .exists
        .unwrap_or_default();

        if exists {
            Ok(())
        } else {
            Err(Error::NotFound {
                entity: "channel".to_string(),
                message: format!(
                    "No {} channel with ID {channel_id} found in this guild",
                    kind.name()
                ),
            })
        }
    }

    /// Asserts the user is a recipient of the given DM channel.
    async fn assert_user_is_recipient(&self, channel_id: u64, user_id: u64) -> crate::Result<()> {
        let exists = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM channel_recipients WHERE channel_id = $1 AND user_id = $2)",
            channel_id as i64,
            user_id as i64
        )
        .fetch_one(self.executor())
        .await?
        .exists
        .unwrap_or_default();

        if exists {
            Ok(())
        } else {
            Err(Error::NotFound {
                entity: "channel".to_string(),
                message: format!("You are not a recipient of any DM channels with ID {channel_id}"),
            })
        }
    }

    /// Asserts the user is the owner of the given group DM channel.
    async fn assert_user_is_group_owner(&self, channel_id: u64, user_id: u64) -> crate::Result<()> {
        let owner_id = sqlx::query!(
            "SELECT owner_id FROM channels WHERE id = $1",
            channel_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|row| row.owner_id.map(|owner_id| owner_id as u64))
        .ok_or_else(|| Error::NotFound {
            entity: "channel".to_string(),
            message: format!("No group DM channel with ID {channel_id} found"),
        })?;

        if owner_id.is_some_and(|owner_id| owner_id != user_id) {
            return Err(Error::NotOwner {
                // TODO: NotGroupDmOwner
                guild_id: 0,
                message: "You are not the owner of this group DM channel".to_string(),
            });
        }

        Ok(())
    }

    /// Inspects basic information about a channel.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channel. If the channel is not found, `Ok(None)` is
    /// returned.
    async fn inspect_channel(&self, channel_id: u64) -> crate::Result<Option<ChannelInspection>> {
        if let Some(inspection) = cache::inspection_for_channel(channel_id).await? {
            return Ok(Some(inspection));
        }

        let channel = if let Some(r) = sqlx::query!(
            "SELECT guild_id, owner_id, type AS kind FROM channels WHERE id = $1",
            channel_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        {
            let inspection = ChannelInspection {
                guild_id: r.guild_id.map(|id| id as _),
                owner_id: r.owner_id.map(|id| id as _),
                channel_type: ChannelType::from_str(&r.kind)?,
            };

            cache::update_channel(channel_id, inspection.clone()).await?;
            inspection
        } else {
            return Ok(None);
        };

        Ok(Some(channel))
    }

    /// Fetches a channel from the database.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channel. If the channel is not found, `Ok(None)` is
    /// returned.
    async fn fetch_channel(&self, channel_id: u64) -> crate::Result<Option<Channel>> {
        let Some(channel) = query_channels!("c.id = $1", channel_id as i64)
            .fetch_optional(self.executor())
            .await?
        else {
            return Ok(None);
        };

        self.construct_channel_with_record(channel).await.map(Some)
    }

    /// Fetches the last message sent in this channel, or `None` if no messages have been sent so
    /// far.
    ///
    /// # Errors
    /// * If an error occurs with fetching the last message.
    async fn fetch_last_message(&self, channel_id: u64) -> crate::Result<Option<Message>> {
        let mut message = sqlx::query!(
            r#"SELECT
                messages.*,
                embeds AS "embeds_ser: sqlx::types::Json<Vec<Embed>>"
            FROM
                messages
            WHERE
                channel_id = $1
            ORDER BY id DESC
            LIMIT 1
            "#,
            channel_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|m| construct_message!(m));

        if let Some(message) = message.as_mut() {
            message.attachments = self.fetch_message_attachments(message.id).await?;
        }
        Ok(message)
    }

    /// Fetches the IDs of all users that can view and receive messages from this channel.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user IDs.
    async fn fetch_channel_recipients(&self, channel_id: u64) -> crate::Result<Vec<u64>> {
        let inspection =
            self.inspect_channel(channel_id)
                .await?
                .ok_or_else(|| Error::NotFound {
                    entity: "channel".to_string(),
                    message: format!("Channel with ID {channel_id} not found"),
                })?;

        if inspection.channel_type.is_dm() {
            return sqlx::query!(
                "SELECT user_id FROM channel_recipients WHERE channel_id = $1",
                channel_id as i64,
            )
            .fetch_all(self.executor())
            .await
            .map(|r| r.into_iter().map(|r| r.user_id as u64).collect())
            .map_err(Into::into);
        }

        let guild_id = inspection.guild_id.unwrap_or(0); // silent-ish fail
        let user_ids = sqlx::query!(
            "SELECT id FROM members WHERE guild_id = $1",
            guild_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| r.id as u64)
        .map(|u| async move {
            self.fetch_member_permissions(guild_id, u, Some(channel_id))
                .await
                .map(|p| (u, p))
        })
        .collect::<TryJoinAll<_>>()
        .await?
        .into_iter()
        .filter_map(|(u, p): (_, Permissions)| p.contains(Permissions::VIEW_CHANNEL).then_some(u))
        .collect();

        Ok(user_ids)
    }

    /// Constructs a channel from the database with the given information.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channel. If the channel is not found, `Ok(None)` is
    /// returned.
    #[allow(clippy::too_many_lines, private_interfaces)]
    async fn construct_channel_with_record(
        &self,
        channel: ChannelRecord,
    ) -> crate::Result<Channel> {
        let channel_id = channel.id as u64;
        let kind = ChannelType::from_str(&channel.kind)?;

        let last_message = self.fetch_last_message(channel_id).await?;
        Ok(if kind.is_guild() {
            let overwrites = self.fetch_channel_overwrites(channel_id).await?;
            Channel::Guild(channel.into_guild_channel(overwrites, last_message)?)
        } else {
            let recipients: Vec<u64> = sqlx::query!(
                "SELECT user_id FROM channel_recipients WHERE channel_id = $1",
                channel.id,
            )
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(|r| r.user_id as u64)
            .collect();
            Channel::Dm(channel.into_dm_channel(recipients, last_message)?)
        })
    }

    /// Fetches channel overwrites in bulk with a custom WHERE clause.
    ///
    /// # Note
    /// All values will be `Some` for the convenience of using [`Option::take`] to own the values.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channel overwrites.
    async fn fetch_channel_overwrites_where(
        &self,
        clause: impl AsRef<str> + Send,
        binding_id: u64,
    ) -> crate::Result<HashMap<u64, Option<Vec<PermissionOverwrite>>>> {
        #[derive(sqlx::FromRow)]
        struct Query {
            channel_id: i64,
            target_id: i64,
            allow: i64,
            deny: i64,
        }

        let overwrites = sqlx::query_as::<_, Query>(&format!(
            "SELECT channel_id, target_id, allow, deny FROM channel_overwrites WHERE {}",
            clause.as_ref(),
        ))
        .bind(binding_id as i64)
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .into_group_map_by(|o| o.channel_id as u64);

        let overwrites = overwrites
            .into_iter()
            .map(|(c, o)| {
                (
                    c,
                    Some(
                        o.into_iter()
                            .map(|o| PermissionOverwrite {
                                id: o.target_id as _,
                                permissions: PermissionPair {
                                    allow: Permissions::from_bits_truncate(o.allow),
                                    deny: Permissions::from_bits_truncate(o.deny),
                                },
                            })
                            .collect(),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(overwrites)
    }

    /// Fetches the channel overwrites for a specific channel. This assumes that the channel is
    /// a guild channel.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channel overwrites.
    /// * If the channel is not a guild channel.
    async fn fetch_channel_overwrites(
        &self,
        channel_id: u64,
    ) -> crate::Result<Vec<PermissionOverwrite>> {
        Ok(sqlx::query!(
            "SELECT target_id, allow, deny FROM channel_overwrites WHERE channel_id = $1",
            channel_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|o| PermissionOverwrite {
            id: o.target_id as u64,
            permissions: PermissionPair {
                allow: Permissions::from_bits_truncate(o.allow),
                deny: Permissions::from_bits_truncate(o.deny),
            },
        })
        .collect())
    }

    /// Fetches a mapping of channel_ids to the last messages sent in those channels.
    ///
    /// # Note
    /// Channel IDs are passed as signed integers akin to how they are represented in the database.
    ///
    /// # Errors
    /// * If an error occurs with fetching the last messages.
    async fn fetch_last_message_map(
        &self,
        channel_ids: &[i64],
    ) -> crate::Result<HashMap<u64, Message>> {
        let message_ids: Vec<u64> = sqlx::query!(
            r#"SELECT id FROM messages
            WHERE channel_id = ANY($1::BIGINT[])
            AND id IN (
                SELECT MAX(id) FROM messages GROUP BY channel_id
            )"#,
            channel_ids,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|m| m.id as u64)
        .collect();

        let messages = self.bulk_fetch_messages(None, &message_ids, None).await?;
        Ok(messages.into_iter().map(|m| (m.channel_id, m)).collect())
    }

    /// Fetches all channels in a guild.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channels.
    async fn fetch_all_channels_in_guild(&self, guild_id: u64) -> crate::Result<Vec<GuildChannel>> {
        let channels: Vec<ChannelRecord> = query_channels!("guild_id = $1", guild_id as i64)
            .fetch_all(self.executor())
            .await?;

        let mut overwrites = self
            .fetch_channel_overwrites_where("guild_id = $1", guild_id)
            .await?;

        let channel_ids: Vec<_> = channels.iter().map(|c| c.id).collect();
        let mut last_messages = self.fetch_last_message_map(&channel_ids).await?;

        let channels = channels
            .into_iter()
            .map(|c| {
                let id = c.id as u64;
                c.into_guild_channel(
                    overwrites
                        .get_mut(&id)
                        .unwrap_or(&mut None)
                        .take()
                        .unwrap_or_default(),
                    last_messages.remove(&id),
                )
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(channels)
    }

    /// Fetches all DM and group channels for a user.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channels.
    async fn fetch_all_dm_channels_for_user(&self, user_id: u64) -> crate::Result<Vec<DmChannel>> {
        let channels = query_channels!(
            "(c.type = 'dm' OR c.type = 'group')
            AND c.id IN (
                SELECT channel_id FROM channel_recipients WHERE user_id = $1
            )",
            user_id as i64
        )
        .fetch_all(self.executor())
        .await?;

        let ids = channels.iter().map(|c| c.id).collect::<Vec<_>>();
        let recipients: HashMap<u64, Vec<_>> = sqlx::query!(
            "SELECT channel_id, user_id FROM channel_recipients WHERE channel_id = ANY($1::BIGINT[])",
            &ids,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .into_group_map_by(|r| r.channel_id as u64);
        let mut last_messages = self.fetch_last_message_map(&ids).await?;

        let mut resolved = Vec::with_capacity(channels.len());
        for channel in channels {
            let channel_id = channel.id as u64;
            resolved.push(
                match channel.into_dm_channel(
                    recipients
                        .get(&channel_id)
                        .map(|r| r.iter().map(|r| r.user_id as u64).collect())
                        .unwrap_or_default(),
                    last_messages.remove(&channel_id),
                ) {
                    Ok(channel) => channel,
                    Err(_) => continue,
                },
            );
        }

        Ok(resolved)
    }

    async fn bulk_register_overwrites(
        &mut self,
        guild_id: u64,
        channel_id: u64,
        overwrites: &[PermissionOverwrite],
    ) -> crate::Result<()> {
        let (targets, (allow, deny)) = overwrites
            .iter()
            .map(|o| {
                (
                    o.id as i64,
                    (o.permissions.allow.bits(), o.permissions.deny.bits()),
                )
            })
            .unzip::<_, _, Vec<_>, (Vec<_>, Vec<_>)>();

        sqlx::query!(
            "DELETE FROM channel_overwrites WHERE channel_id = $1",
            channel_id as i64
        )
        .execute(self.transaction())
        .await?;
        sqlx::query(
            r"INSERT INTO
                channel_overwrites (channel_id, guild_id, target_id, allow, deny)
            SELECT
                $1, $2, out.*
            FROM
                UNNEST($3, $4, $5)
            AS
                out(target_id, allow, deny)",
        )
        .bind(channel_id as i64)
        .bind(guild_id as i64)
        .bind(targets)
        .bind(allow)
        .bind(deny)
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Creates a new channel in a guild from a payload. Payload must be validated prior to creating
    /// the channel.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with creating the channel.
    #[allow(clippy::too_many_lines)]
    async fn create_guild_channel(
        &mut self,
        guild_id: u64,
        channel_id: u64,
        payload: CreateGuildChannelPayload,
    ) -> crate::Result<GuildChannel> {
        let (topic, user_limit) = match &payload.info {
            CreateGuildChannelInfo::Text { topic }
            | CreateGuildChannelInfo::Announcement { topic } => (topic.as_ref(), None),
            CreateGuildChannelInfo::Voice { user_limit } => (None, Some(user_limit)),
            CreateGuildChannelInfo::Category => (None, None),
        };

        let kind = payload.info.channel_type();
        let postgres_parent_id = payload.parent_id.map(|id| id as i64);

        // TODO: this could be integrated into the query
        let position = match kind {
            ChannelType::Category => {
                query_guild_channel_next_position!(
                    @clause "type = 'category'",
                    guild_id as i64,
                    postgres_parent_id
                )
                .fetch_one(get_pool())
                .await?
                .position as u16
            }
            _ => {
                query_guild_channel_next_position!(
                    @clause "type <> 'category'",
                    guild_id as i64,
                    postgres_parent_id
                )
                .fetch_one(get_pool())
                .await?
                .position as u16
            }
        };

        if let Some(ref color) = payload.color {
            color.validate()?;
        }
        let (color, gradient) = payload.color.as_ref().map(ExtendedColor::to_db).unzip();
        sqlx::query!(
            "INSERT INTO channels (
                id, guild_id, type, name, position, parent_id, topic,
                icon, color, gradient, user_limit
            )
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::gradient_type, $11)
            ",
            channel_id as i64,
            guild_id as i64,
            kind.name(),
            payload.name.trim(),
            position as i16,
            postgres_parent_id,
            topic,
            payload.icon,
            color.flatten(),
            gradient.flatten() as _,
            user_limit.map(|&limit| limit as i16),
        )
        .execute(self.transaction())
        .await?;

        if let Some(ref overwrites) = payload.overwrites {
            self.bulk_register_overwrites(guild_id, channel_id, overwrites)
                .await?;
        }

        let info = match payload.info {
            CreateGuildChannelInfo::Text { topic, .. }
            | CreateGuildChannelInfo::Announcement { topic, .. } => {
                let info = TextBasedGuildChannelInfo {
                    topic,
                    ..Default::default()
                };

                match kind {
                    ChannelType::Text => GuildChannelInfo::Text(info),
                    ChannelType::Announcement => GuildChannelInfo::Announcement(info),
                    _ => unreachable!(),
                }
            }
            CreateGuildChannelInfo::Voice { user_limit, .. } => {
                GuildChannelInfo::Voice { user_limit }
            }
            CreateGuildChannelInfo::Category => GuildChannelInfo::Category,
        };

        Ok(GuildChannel {
            id: channel_id,
            guild_id,
            info,
            name: payload.name,
            color: payload.color,
            icon: payload.icon,
            position,
            parent_id: payload.parent_id,
            overwrites: payload.overwrites.unwrap_or_default(),
        })
    }

    /// Creates a new DM-type channel from a payload. Payload must be validated prior to creating
    /// the channel.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with creating the channel.
    async fn create_dm_channel(
        &mut self,
        user_id: u64,
        channel_id: u64,
        payload: CreateDmChannelPayload,
    ) -> crate::Result<DmChannel> {
        let kind = payload.channel_type();
        let (name, owner_id, recipient_ids) = match payload.clone() {
            CreateDmChannelPayload::Dm { recipient_id } => {
                if recipient_id == user_id {
                    return Err(Error::InvalidField {
                        field: "recipient_id".to_string(),
                        message: "Recipient ID cannot be the same as the user ID".to_string(),
                    });
                }

                let db_immut = get_pool();
                if let Some(channel) = query_channels!(
                    "c.type = 'dm' AND c.id IN (
                        SELECT channel_id
                        FROM channel_recipients
                        WHERE user_id = $1
                        AND channel_id IN (
                            SELECT channel_id
                            FROM channel_recipients
                            WHERE user_id = $2
                        )
                    )",
                    user_id as i64,
                    recipient_id as i64
                )
                .fetch_optional(db_immut)
                .await?
                {
                    if let Channel::Dm(channel) =
                        db_immut.construct_channel_with_record(channel).await?
                    {
                        return Ok(channel);
                    }
                }

                (None, None, vec![user_id, recipient_id])
            }
            CreateDmChannelPayload::Group {
                name,
                mut recipient_ids,
            } => {
                if !recipient_ids.contains(&user_id) {
                    recipient_ids.push(user_id);
                }
                (Some(name), Some(user_id), recipient_ids)
            }
        };

        sqlx::query!(
            "INSERT INTO channels (id, name, type, owner_id) VALUES ($1, $2, $3, $4)",
            channel_id as i64,
            name,
            kind.name(),
            owner_id.map(|id| id as i64),
        )
        .execute(self.transaction())
        .await?;

        sqlx::query(
            "INSERT INTO channel_recipients
            SELECT $1, out.* FROM UNNEST($2) AS out(user_id)",
        )
        .bind(channel_id as i64)
        .bind(
            recipient_ids
                .iter()
                .map(|&id| id as i64)
                .collect::<Vec<_>>(),
        )
        .execute(self.transaction())
        .await?;

        Ok(DmChannel {
            id: channel_id,
            info: match payload {
                CreateDmChannelPayload::Dm { recipient_id } => DmChannelInfo::Dm {
                    recipient_ids: (user_id, recipient_id),
                },
                CreateDmChannelPayload::Group { name, .. } => DmChannelInfo::Group {
                    name,
                    topic: None,
                    icon: None,
                    owner_id: user_id,
                    recipient_ids,
                },
            },
            last_message: None,
        })
    }

    /// Edits a channel from a payload. Payload must be validated prior to updating the channel.
    /// Returns a tuple ``(old_channel, new_channel)``.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with updating the channel.
    /// * If the channel is not found.
    async fn edit_channel(
        &mut self,
        channel_id: u64,
        payload: EditChannelPayload,
    ) -> crate::Result<(Channel, Channel)> {
        let mut channel = get_pool()
            .fetch_channel(channel_id)
            .await?
            .ok_or_not_found(
                "channel",
                format!("Channel with ID {channel_id} not found."),
            )?;
        let old = channel.clone();

        if let Some(name) = payload.name {
            channel.set_name(name);
        }

        channel.set_topic(
            payload
                .topic
                .into_option_or_if_absent_then(|| channel.topic().map(ToOwned::to_owned)),
        );
        channel.set_icon(
            payload
                .icon
                .into_option_or_if_absent_then(|| channel.icon().map(ToOwned::to_owned)),
        );

        let limit = payload.user_limit.and_then(|limit| {
            if let Channel::Guild(GuildChannel {
                info:
                    GuildChannelInfo::Voice {
                        ref mut user_limit, ..
                    },
                ..
            }) = channel
            {
                *user_limit = limit;
                Some(limit as i16)
            } else {
                None
            }
        });

        if let Channel::Guild(ref mut channel) = channel {
            let guild_id = channel.guild_id;

            if let Some(ref overwrites) = payload.overwrites {
                self.bulk_register_overwrites(guild_id, channel_id, overwrites)
                    .await?;
                cache::delete_permissions_for_channel(guild_id, channel_id).await?;
                channel.overwrites.clone_from(overwrites);
            }

            channel.color = payload
                .color
                .clone()
                .into_option_or_if_absent(channel.color.clone());
        }

        if let Maybe::Value(ref color) = payload.color {
            color.validate()?;
        }
        let (color, gradient) = payload
            .color
            .into_option()
            .as_ref()
            .map(ExtendedColor::to_db)
            .unzip();

        sqlx::query!(
            r"UPDATE channels
            SET
                name = $1, topic = $2, icon = $3, user_limit = $4,
                color = $5, gradient = $6::gradient_type
            WHERE id = $7",
            channel.name().map(str::trim),
            channel.topic(),
            channel.icon(),
            limit,
            color.flatten(),
            gradient.flatten() as _,
            channel_id as i64,
        )
        .execute(self.transaction())
        .await?;

        cache::remove_channel(channel_id).await?;
        Ok((old, channel))
    }

    /// Edits the positions of all channels in a guild. The positions of only the given channels
    /// will be updated, and there will be no implicit "shifting" of channels to normalize position.
    /// This means that each payload must contain at least two channels.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with updating the channel positions.
    /// * If any of the channels are not found or do not belong to the given guild.
    /// * If two or more channels end up sharing the same position.
    /// * If each channel positioning scope does not begin at 0.
    /// * If there is a gap in the channel positioning scopes.
    async fn edit_guild_channel_positions(
        &mut self,
        guild_id: u64,
        payload: &EditChannelPositionsPayload,
    ) -> crate::Result<Vec<(u64, u16, Option<u64>)>> {
        #[inline]
        fn validate_positions(
            positions: &[(u64, (u16, Option<u64>, ChannelType))],
        ) -> crate::Result<()> {
            let mut seen = HashSet::new();
            let mut expected_position = 0u16;

            for &(_id, (position, _parent_id, _kind)) in positions {
                if position != expected_position {
                    return Err(Error::InvalidField {
                        field: "positions".to_string(),
                        message: "Positions must start at 0 and increment without gaps".to_string(),
                    });
                }
                if !seen.insert(position) {
                    return Err(Error::InvalidField {
                        field: "positions".to_string(),
                        message: format!("Duplicate position {position} found"),
                    });
                }
                expected_position += 1;
            }
            Ok(())
        }

        let mut positions: HashMap<u64, (u16, Option<u64>, ChannelType)> = sqlx::query!(
            r#"SELECT
                id,
                position AS "position!",
                parent_id,
                type AS kind
            FROM
                channels
            WHERE
                guild_id = $1
            "#,
            guild_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| try {
            (
                r.id as u64,
                (
                    r.position as u16,
                    r.parent_id.map(|id| id as u64),
                    ChannelType::from_str(&r.kind)?,
                ),
            )
        })
        .collect::<crate::Result<_>>()?;

        for entry in &payload.positions {
            let Some((position, parent_id, _)) = positions.get_mut(&entry.id) else {
                return Err(Error::NotFound {
                    entity: "channel".to_string(),
                    message: format!(
                        "Channel with ID {} not found in guild with ID {guild_id}",
                        entry.id
                    ),
                });
            };

            *position = entry.position;
            *parent_id = entry
                .parent_id
                .as_ref()
                .into_option_or_if_absent(parent_id.as_ref())
                .copied();
        }

        // Validate
        let (category_positions, channel_positions) = positions
            .iter()
            .partition::<HashMap<_, _>, _>(|(_, (_, _, kind))| *kind == ChannelType::Category);

        let mut category_positions = category_positions
            .into_iter()
            .into_group_map_by(|(_, (_, parent_id, _))| *parent_id);
        let mut channel_positions = channel_positions
            .into_iter()
            .into_group_map_by(|(_, (_, parent_id, _))| *parent_id);

        for scope in category_positions
            .values_mut()
            .chain(channel_positions.values_mut())
        {
            scope.sort_unstable_by_key(|&(_, (position, _, _))| position);
            validate_positions(&scope)?;
        }

        let out = payload.positions.iter().map(|entry| {
            (
                entry.id,
                entry.position,
                entry.parent_id.into_option_or_if_absent_then(|| {
                    positions
                        .get(&entry.id)
                        .and_then(|(_, parent_id, _)| parent_id.as_ref())
                        .map(|id| *id)
                }),
            )
        });
        let (ids, (positions, parent_ids)): (Vec<_>, (Vec<_>, Vec<_>)) = out
            .clone()
            .map(|(id, position, parent_id)| {
                (id as i64, (position as i16, parent_id.map(|id| id as i64)))
            })
            .unzip();

        // UPDATE
        sqlx::query!(
            r#"UPDATE channels
            SET
                position = data.position,
                parent_id = data.parent_id
            FROM
                UNNEST($1::BIGINT[], $2::SMALLINT[], $3::BIGINT[])
                AS data(id, position, parent_id)
            WHERE
                channels.id = data.id
            AND
                channels.guild_id = $4
            "#,
            &ids,
            &positions,
            &parent_ids as &[Option<i64>],
            guild_id as i64,
        )
        .execute(self.transaction())
        .await?;

        cache::clear_member_permissions(guild_id).await?;
        Ok(out.collect())
    }

    /// Deletes the channel with the given ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the channel.
    /// * If the channel is not found.
    async fn delete_channel(&mut self, channel_id: u64) -> crate::Result<()> {
        let ChannelInspection {
            guild_id,
            owner_id: _,
            channel_type: kind,
        } = get_pool()
            .inspect_channel(channel_id)
            .await?
            .ok_or_not_found(
                "channel",
                format!("Channel with ID {channel_id} not found."),
            )?;

        if kind.is_guild() {
            let guild_id = guild_id.ok_or_else(|| Error::InternalError {
                what: Some("internal".to_string()),
                message: "No guild ID found for guild channel, this is a bug".to_string(),
                debug: None,
            })?;

            sqlx::query!(
                r#"UPDATE
                    channels
                SET
                    position = position - 1
                WHERE
                    guild_id = $1
                AND
                    position > (SELECT position FROM channels WHERE id = $2)
                "#,
                guild_id as i64,
                channel_id as i64,
            )
            .execute(self.transaction())
            .await?;
        }

        sqlx::query!("DELETE FROM channels WHERE id = $1", channel_id as i64)
            .execute(self.transaction())
            .await?;

        cache::remove_channel(channel_id).await?;
        Ok(())
    }

    /// Marks a channel as read up to the given message ID for the user in the given channel.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with marking the channel as read.
    async fn ack(&mut self, user_id: u64, channel_id: u64, message_id: u64) -> crate::Result<()> {
        sqlx::query!(
            r"INSERT INTO channel_acks (
                channel_id, user_id, last_message_id
            )
            VALUES ($1, $2, $3)
            ON CONFLICT (channel_id, user_id)
            DO UPDATE SET last_message_id = $3",
            channel_id as i64,
            user_id as i64,
            message_id as i64,
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Fetches a mapping of channel IDs to the last message ID that the user has read up to.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channel acks.
    async fn fetch_last_message_ids(&self, user_id: u64) -> crate::Result<HashMap<u64, u64>> {
        Ok(sqlx::query!(
            r#"SELECT
                channel_id,
                last_message_id AS "last_message_id!"
            FROM
                channel_acks
            WHERE
                user_id = $1 AND last_message_id IS NOT NULL
            "#,
            user_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| (r.channel_id as u64, r.last_message_id as u64))
        .collect())
    }

    /// Fetches all unacknowledged messages, aggregating both last_message_ids and mentions.
    ///
    /// # Errors
    /// * If an error occurs while fetching unread messages.
    async fn fetch_unacked(
        &self,
        user_id: u64,
        guilds: &[Guild],
    ) -> crate::Result<Vec<UnackedChannel>> {
        let mut unacked = self
            .fetch_mentioned_messages(user_id, guilds)
            .await?
            .into_iter()
            .map(|(k, mentions)| {
                (
                    k,
                    UnackedChannel {
                        channel_id: k,
                        last_message_id: None,
                        mentions,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        for (k, last_message_id) in self.fetch_last_message_ids(user_id).await? {
            if let Some(unacked) = unacked.get_mut(&k) {
                unacked.last_message_id = Some(last_message_id);
            } else {
                unacked.insert(
                    k,
                    UnackedChannel {
                        channel_id: k,
                        last_message_id: Some(last_message_id),
                        mentions: Vec::new(),
                    },
                );
            }
        }
        Ok(unacked.into_values().collect())
    }
}

impl<'t, T> ChannelDbExt<'t> for T where T: DbExt<'t> {}
