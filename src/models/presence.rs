use crate::serde_for_bitflags;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// The status of a user's presence.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    /// The user is online and can receive notifications.
    #[default]
    Online,
    /// The user is connected but is not actively interacting with Adapt.
    Idle,
    /// The user is online or idle but will not receive any notifications.
    Dnd,
    /// The user is offline.
    Offline,
}

/// Represents the presence state (status and activity) of a user.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct Presence {
    /// The ID of the user whose presence this is.
    pub user_id: u64,
    /// The status of the user.
    pub status: PresenceStatus,
    /// The custom status of the user, if any.
    pub custom_status: Option<String>,
    /// The devices the user is present on.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub devices: Devices,
    /// User first online timestamp.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    pub online_since: DateTime<Utc>,
}

/// Represents a device a user could be present on. This is provided once during the `identify`
/// payload.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(rename_all = "snake_case")]
pub enum Device {
    /// The user is present on a desktop client.
    Desktop,
    /// The user is present on a mobile client.
    Mobile,
    /// The user is present on a web client.
    Web,
}

bitflags::bitflags! {
    /// Represents all of the devices a user is present on.
    #[derive(Default)]
    pub struct Devices: u32 {
        /// The user is present on a desktop client.
        const DESKTOP = 1 << 0;
        /// The user is present on a mobile client.
        const MOBILE = 1 << 1;
        /// The user is present on a web client.
        const WEB = 1 << 2;
    }
}

serde_for_bitflags!(u32: Devices);
