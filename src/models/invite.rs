use crate::models::PartialGuild;
use chrono::{DateTime, Utc};
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "client")]
use serde::Deserialize;

/// A model representing an invite to a guild. All invites are **immutable**; they cannot be
/// modified once changed.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Invite {
    /// The code of the invite.
    pub code: String,
    /// The ID of the user that created this invite. This is `None` if this is a vanity invite.
    pub inviter_id: Option<u64>,
    /// Partial guild information about the guild this invite leads to. This is `None` when this is
    /// already fetched from a guild.
    pub guild: Option<PartialGuild>,
    /// The ID of the guild this invite leads to.
    pub guild_id: u64,
    /// A timestamp representing when this invite was created. This is `None` if this is a vanity
    /// invite.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub created_at: Option<DateTime<Utc>>,
    /// How many times this invite has been used.
    pub uses: u32,
    /// How many times this invite can be used. ``0`` if unlimited.
    pub max_uses: u32,
    /// How long this invite is valid for, in seconds. ``0`` if this invite never expires. This
    /// counts from the time the invite was created (see `created_at`).
    pub max_age: u32,
}
