use crate::{models::PermissionPair, Error};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
#[cfg(feature = "utoipa")]
use utoipa::{
    openapi::{Array, ArrayBuilder, KnownFormat, ObjectBuilder, SchemaFormat, SchemaType},
    ToSchema,
};

/// Represents common information found in text-based guild channels.
#[derive(Clone, Debug, Default, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct TextBasedGuildChannelInfo {
    /// The topic of the channel, if any.
    pub topic: Option<String>,
    /// Whether the channel is NSFW.
    pub nsfw: bool,
    /// Whether the channel is locked. Only people with the `MANAGE_CHANNELS` permission can
    /// send messages in locked channels.
    pub locked: bool,
    /// The slowmode delay of the channel, in **milliseconds**. This should be a value between
    /// `0` and `86_400_000` (24 hours). `0` indicates the absence of slowmode.
    pub slowmode: u32,
    /// The ID of the last message sent in this channel. This is `None` if no messages have been
    /// sent in this channel, and is sometimes always none in partial contexts.
    pub last_message_id: Option<u64>,
}

/// An intermediate representation of a channel's type. This is never used directly, but is used
/// to help deserialization.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// A text channel.
    Text,
    /// An announcement channel.
    Announcement,
    /// A voice channel.
    Voice,
    /// A category of channels.
    Category,
    /// Two or more channels merged together.
    Merged,
    /// A standard DM channel.
    Dm,
    /// A group DM channel.
    Group,
}

impl FromStr for ChannelType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "announcement" => Ok(Self::Announcement),
            "voice" => Ok(Self::Voice),
            "category" => Ok(Self::Category),
            "merged" => Ok(Self::Merged),
            "dm" => Ok(Self::Dm),
            "group" => Ok(Self::Group),
            _ => {
                Err(Error::InternalError {
                    what: None,
                    message: "Database returned invalid channel type".to_string(),
                    debug: None,
                })
            }
        }
    }
}

impl ChannelType {
    /// Returns the channel type's name.
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Announcement => "announcement",
            Self::Voice => "voice",
            Self::Category => "category",
            Self::Merged => "merged",
            Self::Dm => "dm",
            Self::Group => "group",
        }
    }

    /// Returns whether the channel type is a text-based channel in a guild.
    #[inline]
    #[must_use]
    pub const fn is_guild_text_based(&self) -> bool {
        matches!(self, Self::Text | Self::Announcement)
    }

    /// Returns whether the channel type is a text-based channel.
    #[inline]
    #[must_use]
    pub const fn is_text_based(&self) -> bool {
        self.is_guild_text_based() || self.is_dm()
    }

    /// Returns whether the channel type is a guild channel.
    #[inline]
    #[must_use]
    pub const fn is_guild(&self) -> bool {
        matches!(
            self,
            Self::Text | Self::Announcement | Self::Voice | Self::Category
        )
    }

    /// Returns whether the channel type is a DM channel.
    #[inline]
    #[must_use]
    pub const fn is_dm(&self) -> bool {
        matches!(self, Self::Dm | Self::Group)
    }
}

/// Represents the type along with type-specific info of a guild channel.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum GuildChannelInfo {
    /// A normal text channel.
    Text(TextBasedGuildChannelInfo),
    /// A text channel that has an announcement feed that can be subscribed to.
    Announcement(TextBasedGuildChannelInfo),
    /// A voice channel.
    Voice {
        /// The user limit of the channel. This should be a value between `0` and `500`. A value
        /// of `0` indicates the absence of a user limit.
        user_limit: u16,
    },
    /// A category of channels. This isn't really a channel, but it shares many of the same
    /// properties of one.
    Category,
    /// Two or more channels merged together.
    Merged(TextBasedGuildChannelInfo),
}

impl GuildChannelInfo {
    /// Returns the [`ChannelType`] of the channel.
    #[inline]
    #[must_use]
    pub const fn channel_type(&self) -> ChannelType {
        match self {
            Self::Text { .. } => ChannelType::Text,
            Self::Announcement { .. } => ChannelType::Announcement,
            Self::Voice { .. } => ChannelType::Voice,
            Self::Category => ChannelType::Category,
            Self::Merged { .. } => ChannelType::Merged,
        }
    }
}

/// Represents a permission overwrite.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PermissionOverwrite {
    /// The ID of the role or user this overwrite applies to. The model type can be extracted from
    /// the ID.
    pub id: u64,
    /// The permissions this overwrite grants or denies.
    #[serde(flatten)]
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub permissions: PermissionPair,
}

/// Represents a channel in a guild.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct GuildChannel {
    /// The ID of the channel.
    pub id: u64,
    /// The ID of the guild that this channel is in.
    pub guild_id: u64,
    /// Information about the channel.
    #[serde(flatten)]
    pub info: GuildChannelInfo,
    /// The name of the channel.
    pub name: String,
    /// The position of the channel in the channel list. A lower value means appearing "higher" in
    /// the UI, basically think of this as a 0-indexed listing of the channels from top-to-bottom.
    ///
    /// Positions are scoped per category, and categories have their own positions. Channels that
    /// lack a category will be shown above all categories. This is because no channels can be
    /// displayed in between or after categories - in the UI all non-category channels are displayed
    /// above any other category channels.
    ///
    /// For example:
    ///
    /// ```text
    /// [0] text-channel
    /// [1] voice-channel
    /// [2] another-text-channel
    /// [0] Category
    ///   [0] another-text-channel
    ///   [1] another-voice-channel
    ///   [0] Another Category
    ///     [1] nested-voice-channel
    ///     [2] nested-voice-channel-2
    /// [1] Yet Another Category
    ///   [0] another-text-channel
    /// ```
    pub position: u16,
    /// The permission overwrites for this channel.
    pub overwrites: Vec<PermissionOverwrite>,
    /// The ID of the parent category of the channel. This is `None` if the channel is not in a
    /// category. This is also used for merged channels.
    pub parent_id: Option<u64>,
}

impl Default for GuildChannel {
    fn default() -> Self {
        Self {
            id: 0,
            guild_id: 0,
            info: GuildChannelInfo::Text(TextBasedGuildChannelInfo::default()),
            name: "general".to_string(),
            position: 0,
            overwrites: Vec::new(),
            parent_id: None,
        }
    }
}

#[cfg(feature = "utoipa")]
fn tuple_u64_u64() -> Array {
    ArrayBuilder::new()
        .items(
            ObjectBuilder::new()
                .schema_type(SchemaType::Integer)
                .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int64))),
        )
        .min_items(Some(2))
        .max_items(Some(2))
        .build()
}

/// Represents extra information associated with DM channels.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum DmChannelInfo {
    /// A normal DM channel.
    Dm {
        /// The two IDs of the recipients of the DM.
        #[cfg_attr(feature = "utoipa", schema(schema_with = tuple_u64_u64))]
        recipient_ids: (u64, u64),
    },
    /// A group chat consisting of multiple users.
    Group {
        /// The name of the group chat.
        name: String,
        /// The topic of the group chat, if any.
        topic: Option<String>,
        /// The URL of the group's icon, if any.
        icon: Option<String>,
        /// The ID of the owner of the group chat.
        owner_id: u64,
        /// A list of recipients in the group chat by user ID.
        recipient_ids: Vec<u64>,
    },
}

impl DmChannelInfo {
    /// Returns the [`ChannelType`] of the DM channel.
    #[inline]
    #[must_use]
    pub const fn channel_type(&self) -> ChannelType {
        match self {
            Self::Dm { .. } => ChannelType::Dm,
            Self::Group { .. } => ChannelType::Group,
        }
    }
}

/// Represents a direct-message-like channel that does not belong in a guild.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct DmChannel {
    /// The ID of the channel.
    pub id: u64,
    /// Information about the channel.
    #[serde(flatten)]
    pub info: DmChannelInfo,
    /// The ID of the last message sent in this channel. This is `None` if no messages have been
    /// sent in this channel, and is sometimes always none in partial contexts.
    pub last_message_id: Option<u64>,
}

/// Represents any channel.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(untagged)]
pub enum Channel {
    /// A guild channel.
    Guild(GuildChannel),
    /// A DM channel.
    Dm(DmChannel),
}

impl Channel {
    /// Returns the name of the channel. Returns `None` if the channel is a standard DM channel.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Guild(channel) => Some(&channel.name),
            Self::Dm(channel) => {
                if let DmChannelInfo::Group { ref name, .. } = channel.info {
                    Some(name)
                } else {
                    None
                }
            }
        }
    }

    /// Sets the name of the channel to the given name. If the channel is a standard DM channel,
    /// nothing happens.
    pub fn set_name(&mut self, name: String) {
        match self {
            Self::Guild(channel) => channel.name = name,
            Self::Dm(channel) => {
                if let DmChannelInfo::Group {
                    name: ref mut group_name,
                    ..
                } = channel.info
                {
                    *group_name = name;
                }
            }
        }
    }

    /// Returns the topic of the channel.
    #[must_use]
    pub fn topic(&self) -> Option<&str> {
        match self {
            Self::Guild(channel) => {
                if let GuildChannelInfo::Text(ref info) | GuildChannelInfo::Announcement(ref info) =
                    channel.info
                {
                    info.topic.as_deref()
                } else {
                    None
                }
            }
            Self::Dm(channel) => {
                if let DmChannelInfo::Group { ref topic, .. } = channel.info {
                    topic.as_deref()
                } else {
                    None
                }
            }
        }
    }

    /// Sets the topic of the channel to the given topic.
    pub fn set_topic(&mut self, topic: Option<String>) {
        match self {
            Self::Guild(channel) => {
                if let GuildChannelInfo::Text(ref mut info)
                | GuildChannelInfo::Announcement(ref mut info) = channel.info
                {
                    info.topic = topic;
                }
            }
            Self::Dm(channel) => {
                if let DmChannelInfo::Group {
                    topic: ref mut group_topic,
                    ..
                } = channel.info
                {
                    *group_topic = topic;
                }
            }
        }
    }

    /// Returns the icon of the channel.
    #[must_use]
    pub fn icon(&self) -> Option<&str> {
        match self {
            Self::Guild(_) => None, // TODO: icons for guild channels
            Self::Dm(channel) => {
                if let DmChannelInfo::Group { ref icon, .. } = channel.info {
                    icon.as_deref()
                } else {
                    None
                }
            }
        }
    }

    /// Sets the icon of the channel to the given icon.
    pub fn set_icon(&mut self, icon: Option<String>) {
        match self {
            Self::Guild(_) => (), // TODO: icons for guild channels
            Self::Dm(channel) => {
                if let DmChannelInfo::Group {
                    icon: ref mut group_icon,
                    ..
                } = channel.info
                {
                    *group_icon = icon;
                }
            }
        }
    }
}

/// Represents any channel info.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[serde(untagged)]
pub enum ChannelInfo {
    /// A guild channel.
    Guild(GuildChannelInfo),
    /// A DM channel.
    Dm(DmChannelInfo),
}
