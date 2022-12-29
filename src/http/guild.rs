use crate::Maybe;
use serde::Deserialize;

/// The payload sent to create a new guild.
#[derive(Clone, Debug, Deserialize)]
pub struct CreateGuildPayload {
    /// The name of the guild. Must be between 2 and 100 characters.
    pub name: String,
    /// The description of the guild. Must be between 0 and 1000 characters, or `None` for
    /// no description.
    pub description: Option<String>,
    /// The icon URL for the guild. Must be a valid URL, or `None` to not set an icon.
    pub icon: Option<String>,
    /// The banner URL for the guild. Must be a valid URL, or `None` to not set a banner.
    pub banner: Option<String>,
    /// Whether the guild should be public or not. Defaults to `false`.
    #[serde(default)]
    pub public: bool,
}

/// The payload sent to edit a guild.
#[derive(Clone, Debug, Deserialize)]
pub struct EditGuildPayload {
    /// The new name of the guild. Leave empty to keep the current name.
    pub name: Option<String>,
    /// The new description of the guild. Leave empty to keep the current description, and set to
    /// `null` to remove the description.
    #[serde(default)]
    pub description: Maybe<String>,
    /// The new icon URL of the guild. Leave empty to keep the current icon, and set to `null` to
    /// remove the icon.
    #[serde(default)]
    pub icon: Maybe<String>,
    /// The new banner URL of the guild. Leave empty to keep the current banner, and set to `null`
    /// to remove the banner.
    #[serde(default)]
    pub banner: Maybe<String>,
    /// Whether the guild should be public or not. Leave empty to keep the current setting.
    pub public: Option<bool>,
}

/// The payload sent to delete a guild.
#[derive(Clone, Debug, Deserialize)]
pub struct DeleteGuildPayload {
    /// The password of the user. If this is a bot account, the password is not required and no
    /// body should be sent.
    pub password: String,
}

/// The query parameters used to specify what information to return when fetching a guild.
#[derive(Clone, Debug, Deserialize)]
pub struct GetGuildQuery {
    /// Whether to resolve the guild's channels in the response.
    #[serde(default)]
    pub channels: bool,
    /// Whether to resolve the guild's members in the response.
    #[serde(default)]
    pub members: bool,
    /// Whether to resolve the guild's roles in the response.
    #[serde(default)]
    pub roles: bool,
}
