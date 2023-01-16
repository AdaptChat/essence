use crate::serde_for_bitflags;
use serde::{Deserialize, Serialize};
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

bitflags::bitflags! {
    /// A bitmask of permission flags, representing what members of a guild
    /// are allowed to do.
    ///
    /// Some permissions are not available for specific channel types. The following labels are used
    /// to denote which channels the permission is available for:
    ///
    /// * `T` - Text channels
    /// * `A` - Announcement channels
    /// * `V` - Voice channels
    /// * `-` - No channels (role only)
    /// * `*` - All channels
    ///
    /// Any permission that falls in one of `T`, `A`, or `V` will be a valid permission for category
    /// channels.
    ///
    /// All permissions are available for roles.
    #[derive(Default)]
    pub struct Permissions: i64 {
        /// \*: People with this permission can view channels and receive events from them.
        const VIEW_CHANNEL = 1 << 0;
        /// TA: People with this permission can view the message history of channels. The
        /// `VIEW_CHANNEL` permission is not necessarily required to view the message history,
        /// however it means you cannot receive or send new messages in the channel.
        const VIEW_MESSAGE_HISTORY = 1 << 1;
        /// TA: People with this permission can send messages in channels. The `VIEW_CHANNEL`
        /// permission *is* required to send messages.
        const SEND_MESSAGES = 1 << 2;
        /// TA: People with this permission can manage messages sent by other people. This allows
        /// for the following:
        ///
        /// * Deleting messages sent by others
        /// * Deleting attachments or embeds sent by others
        /// * Removing reactions of others
        /// * Unpublishing messages sent by others (Announcement channels only)
        ///
        /// Note that anyone can still delete their own messages.
        const MANAGE_MESSAGES = 1 << 3;
        /// TA: People with this permission can attach files to messages.
        const ATTACH_FILES = 1 << 4;
        /// TA: People with this permission can send rich embeds or have embed links automatically
        /// appear.
        const SEND_EMBEDS = 1 << 5;
        /// TA: People with this permission can add new reactions to messages. Note that users
        /// without this permission can still react to already existing reactions.
        const ADD_REACTIONS = 1 << 6;
        /// TA: People with this permission can pin *and* unpin messages.
        const PIN_MESSAGES = 1 << 7;
        /// TA: People with this permission can star and unstar messages.
        const STAR_MESSAGES = 1 << 8;
        /// A: People with this permission can publish messages to the announcement feed.
        const PUBLISH_MESSAGES = 1 << 9;
        /// \*: People with this permission can manage settings of channels.
        ///
        /// This includes:
        ///
        /// * Changing the channel name
        /// * Changing the channel topic/description
        /// * Changing the channel icon
        /// * Changing the channel slowmode
        /// * Changing the channel's NSFW status
        /// * Locking or unlocking the channel
        ///
        /// This does **not** give them the ability to change permission overwrites, however,
        /// nor does it give them the ability to create or delete channels.
        const MODIFY_CHANNELS = 1 << 10;
        /// \*: People with this permission can manage channels.
        ///
        /// This includes:
        ///
        /// * Creating or deleting channels
        /// * Changing the channel position (Moving channels)
        /// * Placing channels in another category
        /// * Change permission overwrites for channels
        ///   * They cannot grant or deny and permission that they do not have themselves
        /// * Talking in locked channels
        const MANAGE_CHANNELS = 1 << 11;
        /// TA: People with this permission can create, edit, and delete webhooks.
        const MANAGE_WEBHOOKS = 1 << 12;
        /// \-: People with this permission can create, edit, and delete emojis.
        const MANAGE_EMOJIS = 1 << 13;
        /// \-: People with this permission can delete starboard posts, or disable the starboard
        /// completely.
        const MANAGE_STARBOARD = 1 << 14;
        /// \-: People with this permission can manage the guild's settings.
        ///
        /// This includes:
        /// * Changing guild settings, such as the guild's name and icon.
        /// * Changing the guild's visibility
        /// * Enabling or disabling the starboard
        const MANAGE_GUILD = 1 << 15;
        /// \-: People with this permission can manage the guild's roles. They will be able to
        /// change the permissions of any roles below their top role, and they will be forbidden to
        /// grant or deny any permissions they do not have themselves. They can also assign and
        /// remove any roles to other members, as long as the target role is below their top role.
        const MANAGE_ROLES = 1 << 16;
        /// \-: People with this permission can create invites to the guild.
        const CREATE_INVITES = 1 << 17;
        /// \-: People with this permission can revoke or pause invites of any channel in the guild.
        /// This does not take into account the `CREATE_INVITES` permission, meaning they can revoke
        /// invites even if they cannot create them.
        const MANAGE_INVITES = 1 << 18;
        /// TA: People with this permission can use emojis found in other servers.
        const USE_EXTERNAL_EMOJIS = 1 << 19;
        /// \-: People with this permission can change their own nickname.
        const CHANGE_NICKNAME = 1 << 20;
        /// \-: People with this permission can change the nickname of other people.
        const MANAGE_NICKNAMES = 1 << 21;
        /// \-: People with this permission can timeout and untimeout members that are lower than
        /// them in role hierarchy.
        const TIMEOUT_MEMBERS = 1 << 22;
        /// \-: People with this permission can kick members that are lower than them in role
        /// hierarchy.
        const KICK_MEMBERS = 1 << 23;
        /// \-: People with this permission can ban and unban members that are lower than them in
        /// role hierarchy.
        const BAN_MEMBERS = 1 << 24;
        /// TA: People with this permission can delete or purge messages in bulk.
        /// Unlike Discord, the API allows for up to any number of messages to be deleted at a time.
        const BULK_DELETE_MESSAGES = 1 << 25;
        /// \-: People with this permission can view an audit log of past moderation or other
        /// privileged actions.
        const VIEW_AUDIT_LOG = 1 << 26;
        /// TA: People with this permission can mention large groups of people. This means
        /// mentioning everyone under a non-mentionable role or mentioning everyone.
        const PRIVILEGED_MENTIONS = 1 << 27;
        /// V: People with this permission can connect to a voice channel.
        const CONNECT = 1 << 28;
        /// V: People with this permission can speak in a voice channel.
        const SPEAK = 1 << 29;
        /// V: People with this permission can mute other members in a voice channel.
        const MUTE_MEMBERS = 1 << 30;
        /// V: People with this permission can deafen other members in a voice channel.
        const DEAFEN_MEMBERS = 1 << 31;
        /// \-: People with this permission have the ability to override all permissions and any
        /// channel. This means that despite any overwrites, they will have all permissions
        /// throughout the entire guild.
        const ADMINISTRATOR = 1 << 32;
    }
}

serde_for_bitflags!(i64: Permissions);

/// Represents a pair of permissions, one representing allowed permissions and the other
/// representing denied permissions. This is so that any permission that is represented as
/// "neutral" where it is neither allowed or denied remains easily overwritten by lower
/// roles or members.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct PermissionPair {
    /// The allowed permissions.
    pub allow: Permissions,
    /// The denied permissions.
    pub deny: Permissions,
}
