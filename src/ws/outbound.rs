use serde::Serialize;

/// An outbound websocket message sent by harmony, received by the client.
#[derive(Debug, Serialize)]
#[serde(tag = "t")]
pub enum OutboundMessage {
    /// Sent by harmony when a client first connected to it.
    Hello,
    /// Ping event, sent by harmony to the client.
    Ping,
    /// Pong event, sent by harmony to respond to client's ping event.
    Pong
}
