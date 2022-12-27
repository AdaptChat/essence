use crate::Maybe;
use serde::{Deserialize, Serialize};

/// Payload sent to create a new user.
#[derive(Clone, Debug, Deserialize)]
pub struct CreateUserPayload {
    /// The username of the user. Must be between 2 and 32 characters.
    pub username: String,
    /// The email of the user. Must be a valid email address.
    pub email: String,
    /// The password of the user. Must be between 8 and 32 characters.
    pub password: String,
}

/// Data returned when creating a new user.
#[derive(Clone, Debug, Serialize)]
pub struct CreateUserResponse {
    /// The ID of the user.
    pub id: u64,
    /// The token to use for authentication.
    pub token: String,
}

/// Payload sent when deleting a user.
#[derive(Clone, Debug, Deserialize)]
pub struct DeleteUserPayload {
    /// The password of the user.
    pub password: String,
}

/// Payload sent when changing a user's password.
#[derive(Clone, Debug, Deserialize)]
pub struct ChangePasswordPayload {
    /// The current password of the user.
    pub current_password: String,
    /// The new password of the user.
    pub new_password: String,
}

/// Payload sent when changing a user's email.
#[derive(Clone, Debug, Deserialize)]
pub struct ChangeEmailPayload {
    /// The current password of the user.
    pub password: String,
    /// The new email of the user.
    pub new_email: String,
}

/// Payload sent when editing a user.
#[derive(Clone, Debug, Deserialize)]
pub struct EditUserPayload {
    /// The new username of the user. Leave empty to keep the current username.
    pub username: Option<String>,
    /// The new avatar URL of the user. Leave empty to keep the current avatar, and set to `null` to
    /// remove the avatar.
    pub avatar: Maybe<String>,
    /// The new banner URL of the user. Leave empty to keep the current banner, and set to `null` to
    /// remove the banner.
    pub banner: Maybe<String>,
    /// The new bio of the user. Leave empty to keep the current bio, and set to `null` to remove
    /// the bio.
    pub bio: Maybe<String>,
}