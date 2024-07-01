use chrono::{DateTime, Utc};
#[cfg(feature = "client")]
use serde::Deserialize;
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// Represents a custom emoji.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct CustomEmoji {
    /// The ID of the emoji.
    pub id: u64,
    /// The ID of the guild the emoji is in.
    pub guild_id: u64,
    /// The name of the emoji.
    pub name: String,
    /// The ID of the user that created the emoji. This is `None` if the user has been deleted.
    pub created_by: Option<u64>,
}

/// Represents partial information about a custom emoji or a unicode emoji.
#[derive(Clone, Debug, Serialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct PartialEmoji {
    /// The ID of the custom emoji. This is `None` if the emoji is a unicode emoji.
    pub id: Option<u64>,
    /// The name of the custom emoji, or the emoji itself if this is a unicode emoji.
    pub name: String,
}

/// Represents a reaction on a message.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Reaction {
    /// The ID of the message the reaction is on.
    pub message_id: u64,
    /// The emoji this reaction represents.
    pub emoji: PartialEmoji,
    /// A list of user IDs that have reacted with this emoji.
    pub user_ids: Vec<u64>,
    /// A list of timestamps representing when the users reacted with this emoji. The index of the
    /// timestamp corresponds to the index of the user ID in `user_ids`.
    ///
    /// This is **only** provided when explicitly fetching reactions for a message. Otherwise, this
    /// is `None`.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub created_at: Option<Vec<DateTime<Utc>>>,
}
