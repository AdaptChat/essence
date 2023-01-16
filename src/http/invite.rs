use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// The payload sent to create a new invite in a guild. Note that invites are immutable, so after
/// creation, they cannot be modified.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateInvitePayload {
    /// The maximum number of uses for the invite. Must be at least 1, or leave empty for unlimited
    /// uses.
    #[serde(default)]
    pub max_uses: u32,
    /// The duration of the invite, in seconds. Must be between 0 and 604_800 (7 days), or leave
    /// empty for an invite that never expires.
    #[serde(default)]
    pub max_age: u32,
}
