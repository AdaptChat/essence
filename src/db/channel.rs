#[allow(unused_imports)]
use crate::models::Embed;
use crate::{
    cache,
    db::{message::construct_message, DbExt, MessageDbExt},
    models::{
        Channel, ChannelInfo, ChannelType, DmChannel, DmChannelInfo, Guild, GuildChannel,
        GuildChannelInfo, Message, PermissionOverwrite, PermissionPair, Permissions,
        TextBasedGuildChannelInfo,
    },
    ws::UnackedChannel,
    Error, NotFoundExt,
};
use futures_util::future::TryJoinAll;
use itertools::Itertools;
use std::{collections::HashMap, str::FromStr};

macro_rules! query_guild_channels {
    ($where:literal $(, $($args:expr),*)?) => {{
        sqlx::query!(
            r#"SELECT
                id,
                guild_id AS "guild_id!",
                name AS "name!",
                type AS kind,
                position AS "position!",
                parent_id,
                icon,
                topic,
                nsfw,
                locked,
                slowmode,
                user_limit,
                (
                    SELECT m.id FROM messages m
                    WHERE m.channel_id = c.id
                    ORDER BY id DESC LIMIT 1
                ) AS last_message_id
            FROM
                channels c
            WHERE
            "# + $where,
            $($($args),*)?
        )
    }};
}

macro_rules! query_guild_channel_next_position {
    ($(@clause $clause:literal,)? $($args:expr),*) => {{
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

macro_rules! construct_guild_channel {
    ($data:ident, $overwrites:expr) => {{
        use std::str::FromStr;
        use $crate::models::channel::*;

        let kind = ChannelType::from_str(&$data.kind)?;

        Ok(GuildChannel {
            id: $data.id as _,
            guild_id: $data.guild_id as _,
            info: match kind {
                ChannelType::Text | ChannelType::Announcement => {
                    let info = TextBasedGuildChannelInfo {
                        topic: $data.topic,
                        nsfw: $data.nsfw.unwrap_or_default(),
                        locked: $data.locked.unwrap_or_default(),
                        slowmode: $data.slowmode.unwrap_or_default() as u32,
                        last_message: $data
                            .last_message_id
                            .map(|id| MaybePartialMessage::Id { id: id as _ }),
                    };

                    match kind {
                        ChannelType::Text => GuildChannelInfo::Text(info),
                        ChannelType::Announcement => GuildChannelInfo::Announcement(info),
                        _ => unreachable!(),
                    }
                }
                ChannelType::Voice => GuildChannelInfo::Voice {
                    user_limit: $data.user_limit.unwrap_or_default() as _,
                },
                _ => GuildChannelInfo::Category,
            },
            name: $data.name,
            position: $data.position as u16,
            overwrites: $overwrites,
            parent_id: $data.parent_id.map(|id| id as u64),
        })
    }};
}

use crate::cache::ChannelInspection;
use crate::db::{get_pool, GuildDbExt};
use crate::http::channel::{
    CreateDmChannelPayload, CreateGuildChannelInfo, CreateGuildChannelPayload, EditChannelPayload,
};
use crate::models::MaybePartialMessage;
#[allow(clippy::redundant_pub_crate)] // false positive
pub(crate) use {construct_guild_channel, query_guild_channels};

pub struct ChannelRecord {
    id: i64,
    guild_id: Option<i64>,
    r#type: String,
    name: Option<String>,
    position: Option<i16>,
    parent_id: Option<i64>,
    topic: Option<String>,
    icon: Option<String>,
    slowmode: Option<i32>,
    nsfw: Option<bool>,
    locked: Option<bool>,
    user_limit: Option<i16>,
    owner_id: Option<i64>,
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
        let Some(channel) = sqlx::query_as!(
            ChannelRecord,
            "SELECT * FROM channels WHERE id = $1",
            channel_id as i64
        )
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
    #[allow(clippy::too_many_lines)]
    async fn construct_channel_with_record(
        &self,
        channel: ChannelRecord,
    ) -> crate::Result<Channel> {
        let channel_id = channel.id as u64;
        let kind = ChannelType::from_str(&channel.r#type)?;
        let info = match kind {
            _ if kind.is_guild_text_based() => {
                let info = TextBasedGuildChannelInfo {
                    topic: channel.topic,
                    nsfw: channel.nsfw.unwrap_or_default(),
                    locked: channel.locked.unwrap_or_default(),
                    slowmode: channel.slowmode.unwrap_or_default() as u32,
                    last_message: self
                        .fetch_last_message(channel_id)
                        .await?
                        .map(MaybePartialMessage::Full),
                };

                ChannelInfo::Guild(match kind {
                    ChannelType::Text => GuildChannelInfo::Text(info),
                    ChannelType::Announcement => GuildChannelInfo::Announcement(info),
                    _ => unreachable!(),
                })
            }
            ChannelType::Voice => ChannelInfo::Guild(GuildChannelInfo::Voice {
                user_limit: channel.user_limit.unwrap_or_default() as u16,
            }),
            ChannelType::Category => ChannelInfo::Guild(GuildChannelInfo::Category),
            _ if kind.is_dm() => {
                let recipients: Vec<u64> = sqlx::query!(
                    "SELECT user_id FROM channel_recipients WHERE channel_id = $1",
                    channel.id,
                )
                .fetch_all(self.executor())
                .await?
                .into_iter()
                .map(|r| r.user_id as u64)
                .collect();

                ChannelInfo::Dm(match kind {
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
                        name: channel.name.clone().unwrap_or_default(),
                        icon: channel.icon,
                        topic: channel.topic,
                        owner_id: channel.owner_id.unwrap_or_default() as u64,
                        recipient_ids: recipients,
                    },
                    _ => unreachable!(),
                })
            }
            _ => unimplemented!(),
        };

        let channel = match info {
            ChannelInfo::Guild(info) => {
                let guild_id = channel.guild_id.ok_or_else(|| Error::InternalError {
                    what: None,
                    message: "Guild channel has no guild ID".to_string(),
                    debug: None,
                })? as u64;

                let overwrites = self.fetch_channel_overwrites(channel_id).await?;

                Channel::Guild(GuildChannel {
                    id: channel_id,
                    guild_id,
                    name: channel.name.unwrap_or_default(),
                    position: channel.position.unwrap_or_default() as u16,
                    parent_id: channel.parent_id.map(|id| id as u64),
                    info,
                    overwrites,
                })
            }
            ChannelInfo::Dm(info) => Channel::Dm(DmChannel {
                id: channel_id,
                info,
                last_message: self.fetch_last_message(channel_id).await?,
            }),
        };

        Ok(channel)
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
            r#"SELECT channel_id, target_id, allow, deny FROM channel_overwrites WHERE {}"#,
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

    /// Fetches all channels in a guild.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channels.
    async fn fetch_all_channels_in_guild(&self, guild_id: u64) -> crate::Result<Vec<GuildChannel>> {
        let channels = query_guild_channels!("guild_id = $1", guild_id as i64)
            .fetch_all(self.executor())
            .await?;

        let mut overwrites = self
            .fetch_channel_overwrites_where("guild_id = $1", guild_id)
            .await?;

        let channels = channels
            .into_iter()
            .map(|c| {
                construct_guild_channel!(
                    c,
                    overwrites
                        .get_mut(&(c.id as u64))
                        .unwrap_or(&mut None)
                        .take()
                        .unwrap_or_default()
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
        let channels = sqlx::query_as!(
            ChannelRecord,
            r#"SELECT * FROM channels
            WHERE (type = 'dm' OR type = 'group')
            AND id IN (
                SELECT channel_id FROM channel_recipients WHERE user_id = $1
            )"#,
            user_id as i64,
        )
        .fetch_all(self.executor())
        .await?;

        let mut resolved = Vec::with_capacity(channels.len());
        for channel in channels {
            resolved.push(
                match self.construct_channel_with_record(channel).await.ok() {
                    Some(Channel::Dm(dm)) => dm,
                    _ => continue,
                },
            );
        }

        Ok(resolved)
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
        let (topic, icon, user_limit) = match &payload.info {
            CreateGuildChannelInfo::Text { topic, icon }
            | CreateGuildChannelInfo::Announcement { topic, icon } => {
                (topic.as_ref(), icon.as_ref(), None)
            }
            CreateGuildChannelInfo::Voice { user_limit, icon } => {
                (None, icon.as_ref(), Some(user_limit))
            }
            CreateGuildChannelInfo::Category => (None, None, None),
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
                query_guild_channel_next_position!(guild_id as i64, postgres_parent_id)
                    .fetch_one(get_pool())
                    .await?
                    .position as u16
            }
        };

        sqlx::query!(
            "INSERT INTO channels
                (id, guild_id, type, name, position, parent_id, topic, icon, user_limit)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ",
            channel_id as i64,
            guild_id as i64,
            kind.name(),
            payload.name.trim(),
            position as i16,
            postgres_parent_id,
            topic,
            icon,
            user_limit.map(|&limit| limit as i16),
        )
        .execute(self.transaction())
        .await?;

        if let Some(ref overwrites) = payload.overwrites {
            let (targets, (allow, deny)) = overwrites
                .iter()
                .map(|o| {
                    (
                        o.id as i64,
                        (o.permissions.allow.bits(), o.permissions.deny.bits()),
                    )
                })
                .unzip::<_, _, Vec<_>, (Vec<_>, Vec<_>)>();

            sqlx::query(
                r"INSERT INTO
                    channel_overwrites
                SELECT
                    $1, $2, out.*
                FROM
                    UNNEST($3, $4, $5)
                AS
                    out(target_id, allow, deny)
                ",
            )
            .bind(channel_id as i64)
            .bind(guild_id as i64)
            .bind(targets)
            .bind(allow)
            .bind(deny)
            .execute(self.transaction())
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
                if let Some(channel) = sqlx::query_as!(
                    ChannelRecord,
                    r#"SELECT * FROM channels WHERE type = 'dm' AND id IN (
                        SELECT channel_id
                        FROM channel_recipients
                        WHERE user_id = $1
                        AND channel_id IN (
                            SELECT channel_id
                            FROM channel_recipients
                            WHERE user_id = $2
                        )
                    )"#,
                    user_id as i64,
                    recipient_id as i64,
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

        sqlx::query!(
            "UPDATE channels SET name = $1, topic = $2, icon = $3, user_limit = $4 WHERE id = $5",
            channel.name().map(str::trim),
            channel.topic(),
            channel.icon(),
            limit,
            channel_id as i64,
        )
        .execute(self.transaction())
        .await?;

        cache::remove_channel(channel_id).await?;
        Ok((old, channel))
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
