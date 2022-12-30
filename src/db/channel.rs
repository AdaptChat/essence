use crate::{
    db::DbExt,
    models::{
        Channel, ChannelInfo, ChannelType, DmChannel, DmChannelInfo, GuildChannel,
        GuildChannelInfo, PermissionOverwrite, PermissionPair, Permissions,
        TextBasedGuildChannelInfo,
    },
    Error,
};
use itertools::Itertools;
use std::{collections::HashMap, str::FromStr};

#[async_trait::async_trait]
pub trait ChannelDbExt<'t>: DbExt<'t> {
    /// Inspects basic information about a channel. Returns a tuple
    /// `(guild_id, owner_id, channel_type)`.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channel. If the channel is not found, `Ok(None)` is
    /// returned.
    async fn inspect_channel(
        &self,
        channel_id: u64,
    ) -> crate::Result<Option<(Option<u64>, Option<u64>, ChannelType)>> {
        let channel = if let Some(r) = sqlx::query!(
            "SELECT guild_id, owner_id, type AS kind FROM channels WHERE id = $1",
            channel_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        {
            (
                r.guild_id.map(|id| id as _),
                r.owner_id.map(|id| id as _),
                ChannelType::from_str(&r.kind)?,
            )
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
                    ChannelType::Text => GuildChannelInfo::Text { info },
                    ChannelType::Announcement => GuildChannelInfo::Announcement { info },
                    _ => unreachable!(),
                })
            }
            ChannelType::Voice => ChannelInfo::Guild(GuildChannelInfo::Voice {
                user_limit: channel.user_limit.unwrap_or_default() as u32,
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

                let overwrites = sqlx::query!(
                    r#"SELECT
                        target_id,
                        allow,
                        deny
                    FROM
                        channel_overwrites
                    WHERE
                        guild_id = $1
                    AND
                        channel_id = $2
                    "#,
                    guild_id as i64,
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
                .collect();

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

    /// Fetches all channels in a guild.
    ///
    /// # Errors
    /// * If an error occurs with fetching the channels.
    async fn fetch_all_channels_in_guild(&self, guild_id: u64) -> crate::Result<Vec<GuildChannel>> {
        let channels = sqlx::query!(
            r#"SELECT
                id,
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
                guild_id = $1
            "#,
            guild_id as i64,
        )
        .fetch_all(self.executor())
        .await?;

        let overwrites = sqlx::query!(
            r#"SELECT
                channel_id,
                target_id,
                allow,
                deny
            FROM
                channel_overwrites
            WHERE
                guild_id = $1
            "#,
            guild_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .into_group_map_by(|o| o.channel_id as u64);

        let mut overwrites = overwrites
            .into_iter()
            .map(|(c, o)| {
                (
                    c,
                    Some(
                        o.into_iter()
                            .map(|o| PermissionOverwrite {
                                id: o.target_id as u64,
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

        let channels = channels
            .into_iter()
            .map(|c| {
                let kind = ChannelType::from_str(&c.kind)?;

                Ok(GuildChannel {
                    id: c.id as u64,
                    guild_id,
                    info: match kind {
                        ChannelType::Text | ChannelType::Announcement => {
                            let info = TextBasedGuildChannelInfo {
                                topic: c.topic,
                                nsfw: c.nsfw.unwrap_or_default(),
                                locked: c.locked.unwrap_or_default(),
                                slowmode: c.slowmode.unwrap_or_default() as u32,
                            };

                            match kind {
                                ChannelType::Text => GuildChannelInfo::Text { info },
                                ChannelType::Announcement => {
                                    GuildChannelInfo::Announcement { info }
                                }
                                _ => unreachable!(),
                            }
                        }
                        ChannelType::Voice => GuildChannelInfo::Voice {
                            user_limit: c.user_limit.unwrap_or_default() as u32,
                        },
                        _ => GuildChannelInfo::Category,
                    },
                    name: c.name,
                    position: c.position as u16,
                    overwrites: overwrites
                        .get_mut(&(c.id as u64))
                        .unwrap_or(&mut None)
                        .take()
                        .unwrap_or_default(),
                    parent_id: c.parent_id.map(|id| id as u64),
                })
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(channels)
    }
}

impl<'t, T> ChannelDbExt<'t> for T where T: DbExt<'t> {}
