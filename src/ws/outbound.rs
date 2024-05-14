#[cfg(feature = "client")]
use serde::Deserialize;
use serde::Serialize;

use crate::models::{
    Channel, ClientUser, DmChannel, Guild, Invite, Member, Message, PartialGuild, Presence,
    Relationship, Role, User,
};

/// Extra information about member removal.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemberRemoveInfo {
    /// The guild was deleted. Note that this is never sent in `member_remove` events.
    Delete,
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

/// An unacknowledged channel.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub struct UnackedChannel {
    /// The ID of the channel.
    pub channel_id: u64,
    /// The ID of the last message acknowledged in the channel. If this is `None`, then no messages
    /// have been acknowledged.
    pub last_message_id: Option<u64>,
    /// A list of message IDs that have mentioned you since the last time you acknowledged this
    /// channel.
    pub mentions: Vec<u64>,
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
        /// A list of DM channels that the session's user is a member of.
        dm_channels: Vec<DmChannel>,
        /// A list of channels or guilds favorited and shown in the user's home sidebar.
        favorites: Vec<u64>,
        /// An initial array of all presences observed by the user.
        presences: Vec<Presence>,
        /// A list of relationships associated with the user.
        relationships: Vec<Relationship>,
        /// A list of unacknowledged messages, organized by channel ID.
        unacked: Vec<UnackedChannel>,
        /// A list of resolved message data for unacknowledged messages where the client user is
        /// mentioned, limited to 100 messages.
        inbox: Vec<Message>,
    },
    /// Sent by harmony when an observable user is updated.
    UserUpdate {
        /// The user before it was updated.
        before: User,
        /// The user after it was updated.
        after: User,
    },
    /// Sent by harmony when an observable user is deleted.
    UserDelete {
        /// The ID of the user that was deleted.
        user_id: u64,
    },
    /// Sent by harmony when the client joins or creates a guild. Note that this does not include
    /// guilds received from the `Ready` event, those must be accounted for separately.
    GuildCreate {
        /// The guild that was joined or created.
        guild: Guild,
        /// A custom nonce for this guild. This is a random string that if used, a message with the
        /// same nonce will be dispatched by the websocket, indicating that the guild was created.
        ///
        /// This is only used once and it is not stored.
        nonce: Option<String>,
    },
    /// Sent by harmony when information about a guild is updated.
    GuildUpdate {
        /// The updated guild before modifications
        before: PartialGuild,
        /// The updated guild after modifications
        after: PartialGuild,
    },
    /// Sent by harmony when the client leaves or deletes a guild.
    GuildRemove {
        /// The ID of the guild that was left or deleted.
        guild_id: u64,
        /// Extra information about the guild deletion.
        #[serde(flatten)]
        info: MemberRemoveInfo,
    },
    /// Sent by harmony when a channel is acknowledged ("marked as read")
    ChannelAck {
        /// The ID of the channel that was acknowledged.
        channel_id: u64,
        /// New messages up to this ID can be considered acknowledged.
        last_message_id: u64,
    },
    /// Sent by harmony when a channel is created. This could be any type of channel, including
    /// guild channels, DM channels, and group DM channels.
    ChannelCreate {
        /// The channel that was created.
        channel: Channel,
        /// A custom nonce for this channel. This is a random string that if used, a message with
        /// the same nonce will be dispatched by the websocket, indicating that the channel was
        /// created.
        ///
        /// This is only used once and it is not stored.
        nonce: Option<String>,
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
        /// The ID of the guild that the channel was deleted in, if any.
        guild_id: Option<u64>,
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
    /// Sent by harmony when a message is sent.
    MessageCreate {
        /// The message that was sent by a user.
        message: Message,
        /// A custom nonce for this message. This is a random string that if used, a message with the
        /// same nonce will be dispatched by the websocket, indicating that the message was sent.
        ///
        /// This is only used once and it is not stored.
        nonce: Option<String>,
    },
    /// Sent by harmony when a message is updated.
    MessageUpdate {
        /// The message before it was modified.
        before: Message,
        /// The message after it was modified.
        after: Message,
    },
    /// Sent by harmony when a message is deleted.
    MessageDelete {
        /// The ID of the channel that the message was deleted in.
        channel_id: u64,
        /// The ID of the message that was deleted.
        message_id: u64,
    },
    /// Sent by harmony when a user starts typing.
    TypingStart {
        /// The ID of the channel that the user is typing in.
        channel_id: u64,
        /// The ID of the user that is typing.
        user_id: u64,
    },
    /// Send by harmony when a user stops typing. This is not always sent.
    TypingStop {
        /// The ID of the channel that the user stopped typing in.
        channel_id: u64,
        /// The ID of the user that stopped typing.
        user_id: u64,
    },
    /// Sent by harmony when a user updates their presence.
    PresenceUpdate {
        /// The presence after it was updated. The user ID can be retrieved from accessing
        /// `presence.user_id`.
        presence: Presence,
    },
    /// Sent by harmony when a relationship is created. If a relationship already exists, this
    /// should be treated as an update and replace it.
    RelationshipCreate {
        /// The relationship that was created.
        relationship: Relationship,
    },
    /// Sent by harmony when a relationship is removed.
    RelationshipRemove {
        /// The ID of the user that the relationship was removed with.
        user_id: u64,
    },
}
