use crate::models::Permissions;
use crate::Maybe;
use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// Payload sent to create a new user.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CreateUserPayload {
    /// The unique username of the user. Must between 2 and 32 characters and only contain
    /// alphanumeric characters, periods (.), hyphens (-), and underscores (_).
    pub username: String,
    /// The global display name of the user. Must be between 2 and 32 characters.
    pub display_name: Option<String>,
    /// The email of the user. Must be a valid email address.
    #[cfg_attr(feature = "utoipa", schema(format = "email"))]
    pub email: String,
    /// The password of the user. Must be between 8 and 32 characters.
    #[cfg_attr(feature = "utoipa", schema(format = "password"))]
    pub password: String,
    /// Turnstile CAPTCHA response from Cloudflare.
    pub captcha_token: String,
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

/// Payload sent when creating a new bot account.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CreateBotPayload {
    /// The unique username of the bot. Must between 2 and 32 characters and only contain
    /// alphanumeric characters, periods (.), hyphens (-), and underscores (_).
    ///
    /// Note that unlike usernames, bot usernames are only unique to you. The full username of the
    /// bot will be ``owner_username/given_username``, for example ``user123/MyBot``.
    pub username: String,
    /// The global display name of the bot. Must be between 2 and 32 characters.
    pub display_name: Option<String>,
    /// Whether the bot is public. Public bots can be added by anyone, while private bots can only
    /// be added by the owner.
    #[serde(default)]
    pub public: bool,
}

/// Payload sent to edit details of a bot account.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EditBotPayload {
    /// The inner user payload to edit account details about the bot.
    #[serde(flatten)]
    pub user_payload: EditUserPayload,
    /// Whether the bot should be public. Leave empty to keep the current setting.
    pub public: Option<bool>,
    /// The new default permissions the bot should request for when being added to guilds.
    /// Leave empty to keep the current permissions.
    pub default_permissions: Option<Permissions>,
    /// Whether the bot should support being added to guilds.
    pub guild_enabled: Option<bool>,
    /// Whether the bot should support being added to group DMs.
    pub group_dm_enabled: Option<bool>,
    /// Whether the bot should support global access.
    pub global_enabled: Option<bool>,
}
