use serde::Serialize;

/// An outbound websocket message sent by harmony, received by the client.
#[derive(Debug, Serialize)]
#[serde(tag = "op")]
#[serde(rename_all = "snake_case")]
pub enum OutboundMessage {
    /// Sent by harmony when a client first connects to it.
    Hello,
    /// Ping, sent by harmony to the client.
    Ping,
    /// Pong, sent by harmony to respond to client's ping event.
    Pong,
}
