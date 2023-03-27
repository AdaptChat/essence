use crate::{
    models::{ChannelType, PermissionOverwrite},
    Maybe,
};
use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// The type and other information sent to create a new guild channel.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CreateGuildChannelInfo {
    /// A text channel.
    Text {
        /// The topic of the text channel, if any.
        topic: Option<String>,
        /// The icon of the channel represented as a
        /// [Data URI scheme](https://en.wikipedia.org/wiki/Data_URI_scheme), if any.
        icon: Option<String>,
    },
    /// An announcement channel.
    Announcement {
        /// The topic of the text channel, if any.
        topic: Option<String>,
        /// The icon of the channel represented as a
        /// [Data URI scheme](https://en.wikipedia.org/wiki/Data_URI_scheme), if any.
        icon: Option<String>,
    },
    /// A voice channel.
    Voice {
        /// The user limit of the channel. This should be a value between `0` and `500`. A value
        /// of `0` is the default and indicates the absence of a user limit.
        #[serde(default)]
        user_limit: u16,
        /// The icon of the channel represented as a
        /// [Data URI scheme](https://en.wikipedia.org/wiki/Data_URI_scheme), if any.
        icon: Option<String>,
    },
    /// A category channel.
    Category,
}

impl CreateGuildChannelInfo {
    /// Returns the [`ChannelType`] of the requested channel.
    #[inline]
    #[must_use]
    pub const fn channel_type(&self) -> ChannelType {
        match self {
            Self::Text { .. } => ChannelType::Text,
            Self::Announcement { .. } => ChannelType::Announcement,
            Self::Voice { .. } => ChannelType::Voice,
            Self::Category => ChannelType::Category,
        }
    }
}

/// The request body sent to create a new channel in a guild.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CreateGuildChannelPayload {
    /// The name of the text channel.
    pub name: String,
    /// The type of the channel and information specific to it.
    #[serde(flatten)]
    pub info: CreateGuildChannelInfo,
    /// The icon of the text channel, if any.
    pub icon: Option<String>,
    /// The ID of the category to create the channel in, if any.
    pub parent_id: Option<u64>,
    /// A list of permission overwrites to apply to the channel, if any.
    pub overwrites: Option<Vec<PermissionOverwrite>>,
}

/// The request body sent to create a new DM or group channel.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CreateDmChannelPayload {
    /// A standard DM channel with a single recipient.
    Dm {
        /// The ID of the recipient to add to the DM with.
        recipient_id: u64,
    },
    /// A group DM channel with multiple recipients.
    Group {
        /// The name of the group DM.
        name: String,
        /// A list of recipient IDs to initially add to the group DM.
        recipient_ids: Vec<u64>,
    },
}

impl CreateDmChannelPayload {
    /// Returns the [`ChannelType`] of the requested channel.
    #[inline]
    #[must_use]
    pub const fn channel_type(&self) -> ChannelType {
        match self {
            Self::Dm { .. } => ChannelType::Dm,
            Self::Group { .. } => ChannelType::Group,
        }
    }
}

/// The request body sent to modify a channel.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EditChannelPayload {
    /// The new name of the channel. If left blank, the name will not be changed. Takes effect for
    /// all channels except for user DMs.
    pub name: Option<String>,
    /// The new topic or description of the channel. Explicitly setting this to `None` will clear
    /// the topic. Only takes effect for text-based channels in guilds, or group chats.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>))]
    pub topic: Maybe<String>,
    /// The new icon of the channel. Explicitly setting this to `None` will clear the icon.
    /// Takes effect for all channels except for user DMs.
    ///
    /// This should be a [Data URI scheme](https://en.wikipedia.org/wiki/Data_URI_scheme).
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>, format = "byte"))]
    pub icon: Maybe<String>,
    /// The new user limit of the voice channel. Explicitly setting this to `0` will remove the
    /// current limit, if there is any. Only takes effect for guild voice channels.
    pub user_limit: Option<u16>,
}

/// The payload used per channel to specify its new position data.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EditChannelPositionPayload {
    /// The ID of the channel to modify.
    pub id: u64,
    /// The new position of the channel.
    pub position: u16,
    /// The new scope of the channel. If left blank, the scope will not be changed. If set to
    /// `Null`, the channel will be moved to the root of the channel list.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<u64>))]
    pub scope: Maybe<u64>,
}

/// The request body sent to modify channel positions.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(transparent)]
pub struct EditChannelPositionsPayload {
    /// A list of channel positions to modify.
    pub positions: Vec<EditChannelPositionPayload>,
}
