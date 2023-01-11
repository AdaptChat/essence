#[cfg(feature = "client")]
use serde::Deserialize;
use serde::Serialize;

use crate::models::{ClientUser, Guild, GuildChannel};

/// An outbound websocket message sent by harmony, received by the client.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
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
    /// Sent by harmony when the client joins or creates a guild. Note that this does not include
    /// guilds received from the `Ready` event, those must be accounted for separately.
    GuildCreate {
        /// The guild that was joined or created.
        guild: Guild,
    },
    /// Sent by harmony when a channel is created within a guild.
    GuildChannelCreate {
        /// The channel that was created.
        channel: GuildChannel,
    },
}
