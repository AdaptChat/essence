#[cfg(feature = "client")]
use serde::Deserialize;
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Emoji {
    pub id: u64,
    pub guild_id: u64,
    pub name: String,
    pub created_by: u64,
}
