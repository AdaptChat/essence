use crate::Maybe;
use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// Payload sent to create a new user.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CreateUserPayload {
    /// The username of the user. Must be between 2 and 32 characters.
    pub username: String,
    /// The email of the user. Must be a valid email address.
    #[cfg_attr(feature = "utoipa", schema(format = "email"))]
    pub email: String,
    /// The password of the user. Must be between 8 and 32 characters.
    #[cfg_attr(feature = "utoipa", schema(format = "password"))]
    pub password: String,
}

/// Data returned when creating a new user.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CreateUserResponse {
    /// The ID of the user.
    pub id: u64,
    /// The token to use for authentication.
    pub token: String,
}

/// Payload sent when deleting a user.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DeleteUserPayload {
    /// The password of the user.
    #[cfg_attr(feature = "utoipa", schema(format = "password"))]
    pub password: String,
}

/// Payload sent when changing a user's password.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChangePasswordPayload {
    /// The current password of the user.
    pub current_password: String,
    /// The new password of the user.
    pub new_password: String,
}

/// Payload sent when changing a user's email.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ChangeEmailPayload {
    /// The current password of the user.
    pub password: String,
    /// The new email of the user.
    pub new_email: String,
}

/// Payload sent when editing a user.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EditUserPayload {
    /// The new username of the user. Leave empty to keep the current username.
    pub username: Option<String>,
    /// The new display name of the user. Leave empty to keep the current display name, and set to
    /// `null` to remove the display name.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>))]
    pub display_name: Maybe<String>,
    /// The new avatar of the user. Leave empty to keep the current avatar, and set to `null` to
    /// remove the avatar. If provided, the avatar should be represented as a
    /// [Data URI scheme](https://en.wikipedia.org/wiki/Data_URI_scheme).
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>, format = Byte))]
    pub avatar: Maybe<String>,
    /// The new banner URL of the user. Leave empty to keep the current banner, and set to `null` to
    /// remove the banner.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>))]
    pub banner: Maybe<String>,
    /// The new bio of the user. Leave empty to keep the current bio, and set to `null` to remove
    /// the bio.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>))]
    pub bio: Maybe<String>,
}

/// Payload sent when requesting to add a user as a friend.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SendFriendRequestPayload {
    /// The username of the user to add as a friend.
    pub username: String,
}
