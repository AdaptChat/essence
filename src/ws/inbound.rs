use crate::models::{Devices, PresenceStatus};
use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;

/// Payload sent from the client to update presence.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
pub struct UpdatePresencePayload {
    /// The new status of the client, if any.
    pub status: Option<PresenceStatus>,
    /// The new devices of the client, if any.
    pub devices: Option<Devices>,
}

/// An inbound websocket message sent by the client, received by the server.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum InboundMessage {
    /// Sent by the client to identify and authenticate itself to the websocket.
    Identify {
        /// The token to use for authentication.
        token: String,
        /// The initial presence of the client.
        presence: Option<UpdatePresencePayload>,
    },
    /// Ping, sent by the client to harmony.
    Ping,
    /// Pong, used to respond to harmony's ping event.
    Pong,
    /// Used to change the client's current presence status.
    UpdatePresence {
        /// The status to change to.
        status: UpdatePresencePayload,
    },
}
