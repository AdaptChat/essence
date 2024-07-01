use crate::models::{Device, PresenceStatus};
use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;

/// An inbound websocket message sent by the client, received by the server.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum InboundMessage {
    /// Sent by the client to identify and authenticate itself to the websocket.
    Identify {
        /// The token to use for authentication.
        token: String,
        /// The initial status of the client. Defaults to `online`.
        #[serde(default)]
        status: PresenceStatus,
        /// Custom status of the client, if any.
        custom_status: Option<String>,
        /// The device that this client is connecting on.
        device: Device,
    },
    /// Ping, sent by the client to harmony.
    Ping,
    /// Pong, used to respond to harmony's ping event.
    Pong,
    /// Used to change the client's current presence status.
    UpdatePresence {
        /// The new status of the client, if any.
        status: Option<PresenceStatus>,
        /// The new custom status of the client, if any.
        custom_status: Option<String>,
    },
}
