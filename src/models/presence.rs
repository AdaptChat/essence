use chrono::{DateTime, Utc};

/// User status model
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Online,
    Idle,
    Dnd,
    Offline,
}

/// User's presence
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub enum UserPresence {
    /// The status of the user.
    status: PresenceStatus,
    /// User first online timestamp.
    #[cfg_attr(feature = "bincode", bincode(with_serde))]
    online_since: DateTime<Utc>,
}
