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
                    user_limit: $data.user_limit.unwrap_or_default() as u32,
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

#[allow(clippy::redundant_pub_crate)] // false positive
pub(crate) use {construct_guild_channel, query_guild_channels};

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
                    ChannelType::Text => GuildChannelInfo::Text(info),
                    ChannelType::Announcement => GuildChannelInfo::Announcement(info),
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
}

impl<'t, T> ChannelDbExt<'t> for T where T: DbExt<'t> {}
