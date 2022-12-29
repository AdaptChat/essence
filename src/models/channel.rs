use crate::models::PermissionPair;
use serde::{Deserialize, Serialize};

/// Represents common information found in text-based guild channels.
#[derive(Clone, Debug, Serialize)]
pub struct TextBasedGuildChannelInfo {
    /// The topic of the channel, if any.
    topic: Option<String>,
    /// Whether the channel is NSFW.
    nsfw: bool,
    /// Whether the channel is locked. Only people with the `MANAGE_CHANNELS` permission can
    /// send messages in locked channels.
    locked: bool,
    /// The slowmode delay of the channel, in **milliseconds**. This should be a value between
    /// `0` and `86_400_000` (24 hours). `0` indicates the absence of slowmode.
    slowmode: u32,
}

/// Represents the type along with type-specific info of a guild channel.
#[derive(Clone, Debug, Serialize)]
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
        user_limit: u32,
    },
    /// A category of channels. This isn't really a channel, but it shares many of the same
    /// properties of one.
    Category,
}

/// Represents a permission overwrite.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PermissionOverwrite {
    /// The ID of the role or user this overwrite applies to. The model type can be extracted from
    /// the ID.
    pub id: u64,
    /// The permissions this overwrite grants or denies.
    #[serde(flatten)]
    pub permissions: PermissionPair,
}

/// Represents a channel in a guild.
#[derive(Clone, Debug, Serialize)]
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
    /// category.
    pub parent_id: Option<u64>,
}

/// Represents extra information associated with DM channels.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum DmChannelInfo {
    /// A normal DM channel.
    Dm {
        /// The ID of the recipient of the DM.
        recipient_id: u64,
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

/// Represents a direct-message-like channel that does not belong in a guild.
#[derive(Clone, Debug, Serialize)]
pub struct DmChannel {
    /// The ID of the channel.
    pub id: u64,
    /// Information about the channel.
    #[serde(flatten)]
    pub info: DmChannelInfo,
}

/// Represents any channel.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Channel {
    /// A guild channel.
    Guild(GuildChannel),
    /// A DM channel.
    Dm(DmChannel),
}
