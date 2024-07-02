use crate::Maybe;
use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::{IntoParams, ToSchema};

/// The payload sent to create a new guild.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CreateGuildPayload {
    /// The name of the guild. Must be between 2 and 100 characters.
    pub name: String,
    /// The description of the guild. Must be between 0 and 1000 characters, or `None` for
    /// no description.
    pub description: Option<String>,
    /// The icon for the guild. Must be a valid URL, or `None` to not set an icon. This should be
    /// a [Data URI scheme](https://en.wikipedia.org/wiki/Data_URI_scheme) if provided.
    pub icon: Option<String>,
    /// The banner URL for the guild. Must be a valid URL, or `None` to not set a banner.
    pub banner: Option<String>,
    /// Whether the guild should be public or not. Defaults to `false`.
    #[serde(default)]
    pub public: bool,
    /// A nonce to include with the guild. This is not stored and can be used to identify the
    /// guild later on (it is relayed through the websocket).
    pub nonce: Option<String>,
}

/// The payload sent to edit a guild.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EditGuildPayload {
    /// The new name of the guild. Leave empty to keep the current name.
    pub name: Option<String>,
    /// The new description of the guild. Leave empty to keep the current description, and set to
    /// `null` to remove the description.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>))]
    pub description: Maybe<String>,
    /// The new icon of the guild. Leave empty to keep the current icon, and set to `null` to
    /// remove the icon. The icon should be represented as a
    /// [Data URI scheme](https://en.wikipedia.org/wiki/Data_URI_scheme).
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>, format = "byte"))]
    pub icon: Maybe<String>,
    /// The new banner URL of the guild. Leave empty to keep the current banner, and set to `null`
    /// to remove the banner.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "utoipa", schema(nullable, value_type = Option<String>))]
    pub banner: Maybe<String>,
    /// Whether the guild should be public or not. Leave empty to keep the current setting.
    pub public: Option<bool>,
}

/// The payload sent to delete a guild.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct DeleteGuildPayload {
    /// The password of the user. If this is a bot account, the password is not required and no
    /// body should be sent.
    pub password: String,
}

/// The query parameters used to specify what information to return when fetching a guild.
#[derive(Clone, Debug, Default, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(IntoParams))]
#[allow(clippy::struct_excessive_bools)]
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
    /// Whether to resolve the guild's emojis in the response.
    #[serde(default)]
    pub emojis: bool,
}

impl GetGuildQuery {
    /// Creates a new query with all fields set to `false`.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            channels: false,
            members: false,
            roles: false,
            emojis: false,
        }
    }

    /// Creates a new query with all fields set to `true`.
    #[inline]
    #[must_use]
    pub const fn all() -> Self {
        Self {
            channels: true,
            members: true,
            roles: true,
            emojis: true,
        }
    }
}
