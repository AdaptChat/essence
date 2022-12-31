use crate::{models::PermissionOverwrite, Maybe};
use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// The type and other information sent to create a new guild channel.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "type")]
pub enum CreateGuildChannelInfo {
    /// A text channel.
    Text {
        /// The topic of the text channel, if any.
        topic: Option<String>,
        /// The URL of the icon of the channel, if any.
        icon: Option<String>,
    },
    /// An announcement channel.
    Announcement {
        /// The topic of the text channel, if any.
        topic: Option<String>,
        /// The URL of the icon of the channel, if any.
        icon: Option<String>,
    },
    /// A voice channel.
    Voice {
        /// The user limit of the channel. This should be a value between `0` and `500`. A value
        /// of `0` is the default and indicates the absence of a user limit.
        #[serde(default)]
        user_limit: u32,
        /// The URL of the icon of the channel, if any.
        icon: Option<String>,
    },
    /// A category channel.
    Category,
}

/// The request body sent to create a new channel in a guild.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
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
    /// The position of the channel in the channel list. If one isn't provided, the position
    /// will be the last in its position scope.
    pub position: Option<u16>,
}

/// The request body sent to modify a channel.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EditChannelPayload {
    /// The new name of the channel. If left blank, the name will not be changed. Takes effect for
    /// all channels except for user DMs.
    pub name: Option<String>,
    /// The new topic or description of the channel. Explicitly setting this to `None` will clear
    /// the topic. Only takes effect for text-based channels in guilds, or group chats.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    pub topic: Maybe<String>,
    /// The new icon URL of the channel. Explicitly setting this to `None` will clear the icon.
    /// Takes effect for all channels except for user DMs.
    #[serde(default)]
    pub icon: Maybe<String>,
    /// The new user limit of the voice channel. Explicitly setting this to `None` will remove the
    /// current limit, if there is any. Only takes effect for guild voice channels.
    pub user_limit: Maybe<u16>,
}

/// The payload used per channel to specify its new position data.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EditChannelPositionPayload {
    /// The ID of the channel to modify.
    pub id: u64,
    /// The new position of the channel.
    pub position: u16,
    /// The new scope of the channel. If left blank, the scope will not be changed. If set to
    /// `Null`, the channel will be moved to the root of the channel list.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    pub scope: Maybe<u64>,
}

/// The request body sent to modify channel positions.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(transparent)]
pub struct EditChannelPositionsPayload {
    /// A list of channel positions to modify.
    pub positions: Vec<EditChannelPositionPayload>,
}
