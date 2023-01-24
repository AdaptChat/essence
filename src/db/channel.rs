use crate::{
    cache,
    db::DbExt,
    models::{
        Channel, ChannelInfo, ChannelType, DmChannel, DmChannelInfo, GuildChannel,
        GuildChannelInfo, PermissionOverwrite, PermissionPair, Permissions,
        TextBasedGuildChannelInfo,
    },
    Error, NotFoundExt,
};
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
                user_limit
            FROM
                channels
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
use crate::db::get_pool;
use crate::http::channel::{CreateGuildChannelInfo, CreateGuildChannelPayload, EditChannelPayload};
#[allow(clippy::redundant_pub_crate)] // false positive
pub(crate) use {construct_guild_channel, query_guild_channels};

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

    /// Inspects basic information about a channel. Returns a tuple
    /// `(guild_id, owner_id, channel_type)`.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channel. If the channel is not found, `Ok(None)` is
    /// returned.
    async fn inspect_channel(&self, channel_id: u64) -> crate::Result<Option<ChannelInspection>> {
        if let Some(inspection) = cache::read().await.channels.get(&channel_id) {
            return Ok(Some(*inspection));
        }

        let channel = if let Some(r) = sqlx::query!(
            "SELECT guild_id, owner_id, type AS kind FROM channels WHERE id = $1",
            channel_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        {
            let inspection = (
                r.guild_id.map(|id| id as _),
                r.owner_id.map(|id| id as _),
                ChannelType::from_str(&r.kind)?,
            );

            cache::write().await.channels.insert(channel_id, inspection);
            inspection
        } else {
            return Ok(None);
        };

        Ok(Some(channel))
    }

    /// Fetches a channel from the database with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channel. If the channel is not found, `Ok(None)` is
    /// returned.
    #[allow(clippy::too_many_lines)]
    async fn fetch_channel(&self, channel_id: u64) -> crate::Result<Option<Channel>> {
        let channel = if let Some(c) =
            sqlx::query!("SELECT * FROM channels WHERE id = $1", channel_id as i64)
                .fetch_optional(self.executor())
                .await?
        {
            c
        } else {
            return Ok(None);
        };

        let kind = ChannelType::from_str(&channel.r#type)?;
        let info = match kind {
            _ if kind.is_guild_text_based() => {
                let info = TextBasedGuildChannelInfo {
                    topic: channel.topic,
                    nsfw: channel.nsfw.unwrap_or_default(),
                    locked: channel.locked.unwrap_or_default(),
                    slowmode: channel.slowmode.unwrap_or_default() as u32,
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
                    channel_id as i64,
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
            }),
        };

        Ok(Some(channel))
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
                r#"INSERT INTO
                    channel_overwrites
                SELECT
                    $1, $2, out.*
                FROM
                    UNNEST($3, $4, $5)
                AS
                    out(target_id, allow, deny)
                "#,
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

        cache::write().await.channels.remove(&channel_id);
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
        let (guild_id, _, kind) = get_pool()
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

        cache::write().await.channels.remove(&channel_id);
        Ok(())
    }
}

impl<'t, T> ChannelDbExt<'t> for T where T: DbExt<'t> {}
