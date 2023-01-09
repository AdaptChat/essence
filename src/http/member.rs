use crate::Maybe;
use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// The payload send to edit the authenticated user as a member.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EditClientMemberPayload {
    /// The new nickname of the member. Leave empty to keep the current nickname, and set to `null`
    /// to remove the nickname.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "openapi", schema(nullable, value_type = Option<String>))]
    pub nick: Maybe<String>,
}

/// The payload sent to edit a member.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EditMemberPayload {
    /// The new nickname of the member. Leave empty to keep the current nickname, and set to `null`
    /// to remove the nickname.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "openapi", schema(nullable, value_type = Option<String>))]
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
}
