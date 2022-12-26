use serde::Deserialize;

/// An inbound websocket message sent by the client, received by the server.
#[derive(Debug, Deserialize)]
pub enum InboundMessage {
    /// Sent by the client to identify and authenticate itself to the websocket.
    Identify {
        /// The token to use for authentication.
        token: String,
    },
}
