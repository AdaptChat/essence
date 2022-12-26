use serde::Deserialize;

/// An inbound websocket message sent by the client, received by the server.
#[derive(Debug, Deserialize)]
#[serde(tag = "op")]
#[serde(rename_all = "snake_case")]
pub enum InboundMessage {
    /// Sent by the client to identify and authenticate itself to the websocket.
    Identify {
        /// The token to use for authentication.
        token: String,
    },
    /// Ping, sent by the client to harmony.
    Ping,
    /// Pong, used to respond to harmony's ping event.
    Pong,
}
