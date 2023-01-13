#[cfg(feature = "client")]
use serde::Deserialize;
use serde::Serialize;

use crate::models::invite::Invite;
use crate::models::{Channel, ClientUser, Guild, GuildChannel, Member, Role};

/// Extra information about member removal.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(tag = "type")]
pub enum MemberRemoveInfo {
    /// The member left on their own.
    Leave,
    /// The member was kicked.
    Kick {
        /// The ID of the moderator that kicked the member.
        moderator_id: u64,
    },
    // TODO: Ban should include ban info
    /// The member was banned.
    Ban {
        /// The ID of the moderator that banned the member.
        moderator_id: u64,
    },
}

/// An outbound websocket message sent by harmony, received by the client.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
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
    /// Sent by harmony when the client leaves or deletes a guild.
    GuildDelete {
        /// The ID of the guild that was left or deleted.
        guild_id: u64,
        /// Whether this event was sent due to the guild being deleted.
        ///
        /// If `true`, the guild was deleted by the guild's owner.
        /// Otherwise, it is likely that the user was kicked or left the guild.
        deleted: bool,
    },
    /// Sent by harmony when a channel is created within a guild.
    GuildChannelCreate {
        /// The channel that was created.
        channel: GuildChannel,
    },
    /// Sent by harmony when a channel is modified.
    ChannelUpdate {
        /// The channel before it was modified.
        before: Channel,
        /// The channel after modifications.
        after: Channel,
    },
    /// Sent by harmony when a channel is deleted.
    ChannelDelete {
        /// The ID of the channel that was deleted.
        channel_id: u64,
    },
    /// Sent by harmony when a role is created within a guild.
    RoleCreate {
        /// The role that was created.
        role: Role,
    },
    /// Sent by harmony when a role is updated.
    RoleUpdate {
        /// The role before it was modified.
        before: Role,
        /// The role after it was modified.
        after: Role,
    },
    /// Sent by harmny when a role is deleted.
    RoleDelete {
        /// The ID of the role that was deleted.
        role_id: u64,
    },
    /// Sent by harmony when a member joins a guild. The guild ID can be retrieved from
    /// accessing `member.guild_id`.
    MemberJoin {
        /// Information about the member that joined the guild.
        member: Member,
        /// The invite used to join the guild, if any.
        invite: Option<Invite>,
    },
    /// Sent by harmony when a member in a guild is updated. The guild ID can be retrieved from
    /// accessing `before.guild_id` or `after.guild_id`.
    MemberUpdate {
        /// The member before it was modified.
        before: Member,
        /// The member after it was modified.
        after: Member,
    },
    /// Sent by harmony when a member is removed from a guild. This can be due to a member leaving,
    /// being kicked, or being banned.
    MemberRemove {
        /// The ID of the guild that the member was removed from.
        guild_id: u64,
        /// The ID of the member that was removed.
        user_id: u64,
        /// Extra information about the removal.
        #[serde(flatten)]
        info: MemberRemoveInfo,
    },
}
