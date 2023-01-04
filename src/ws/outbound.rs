#[cfg(feature = "client")]
use serde::Deserialize;
use serde::Serialize;

use crate::models::{ClientUser, Guild};

/// An outbound websocket message sent by harmony, received by the client.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[serde(tag = "op", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum OutboundMessage {
    /// Sent by harmony when a client first connects to it.
    Hello,
    /// Ping, sent by harmony to the client.
    Ping,
    /// Pong, sent by harmony to respond to client's ping event.
    Pong,
    /// Ready, sent by harmony when it is ready to send and receive events.
    Ready {
        /// The ID of the current session.
        session_id: String,
        /// The client user of the current session.
        user: ClientUser,
        /// A list of guilds that the session's user is a member of.
        guilds: Vec<Guild>,
    },
}
