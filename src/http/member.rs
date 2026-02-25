use crate::{Maybe, models::Permissions};
use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// The payload send to edit the authenticated user as a member.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EditClientMemberPayload {
    /// The new nickname of the member. Leave empty to keep the current nickname, and set to `null`
    /// to remove the nickname.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>))]
    pub nick: Maybe<String>,
}

/// The payload sent to edit a member.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EditMemberPayload {
    /// The new nickname of the member. Leave empty to keep the current nickname, and set to `null`
    /// to remove the nickname.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>))]
    pub nick: Maybe<String>,
    /// If provided, this is a bulk overwrite of the member's roles. Any roles not in this list
    /// will be removed from the member, and any roles in this list that are managable by the
    /// user will be added to the member (that is, if the role isn't managed and the user's top role
    /// is higher than the role).
    ///
    /// If any role is not found, a 404 will be returned.
    ///
    /// The default role will always be added to the member, regardless of whether it is in this
    /// list.
    pub roles: Option<Vec<u64>>,
    /// The new base permissions granted to the member. Leave empty to keep the current permissions.
    pub permissions: Option<Permissions>,
}

/// The payload sent to ban a member from a guild.
#[derive(Clone, Debug, Default, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct BanMemberPayload {
    /// The reason for the ban.
    pub reason: Option<String>,
    /// The number of seconds of message history to delete from the banned user. Capped at 7 days
    /// (604800 seconds). If not specified or `0`, no messages are deleted.
    #[serde(default)]
    pub delete_message_seconds: u32,
}

/// The payload sent to add a bot to a guild.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct AddBotPayload {
    /// The base permissions the bot should be granted in the guild. Leave empty to grant default
    /// configured permissions set by the bot owner.
    pub permissions: Option<Permissions>,
}
