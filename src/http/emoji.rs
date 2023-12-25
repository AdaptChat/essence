use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// The payload sent to create a new emoji.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CreateEmojiPayload {
    /// The name of the emoji.
    pub name: String,
    /// The guild id the emoji belongs to.
    pub guild_id: u64,
    /// The user who created the emoji.
    pub created_by: u64,
}

/// The payload sent to modify an emoji.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct UpdateEmojiPayload {
    /// The id of the emoji.
    pub id: u64,
    /// The new name of the emoji.
    pub name: String,
}
