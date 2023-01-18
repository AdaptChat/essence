use crate::models::Embed;
use crate::Maybe;
use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "openapi")]
use utoipa::{IntoParams, ToSchema};

/// Payload sent to send a message.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateMessagePayload {
    /// The content of the message, if any. If specified, this should be a string with a size of at
    /// most 4 KB.
    pub content: Option<String>,
    /// A list of rich embeds to send with the message. Leave empty to send no embeds. If specified,
    /// this takes a maximum of 10 embeds.
    #[serde(default)]
    pub embeds: Vec<Embed>,
    /// A nonce to include with the message. This is not stored and can be used to identify the
    /// message later on (it is relayed through the websocket).
    pub nonce: Option<String>,
}

/// Payload sent to edit a message.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EditMessagePayload {
    /// The new content of the message, if any. Explicitly specify `null` to remove the content.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "openapi", schema(nullable, value_type = Option<String>))]
    pub content: Maybe<String>,
    /// A list of rich embeds to send with the message.
    ///
    /// This will overwrite any existing embeds if specified.
    /// This wlil remove all embeds if set to either an empty list or explicitly set to `null`.
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "openapi", schema(nullable, value_type = Vec<Embed>))]
    pub embeds: Maybe<Vec<Embed>>,
}

#[inline]
const fn default_limit() -> u8 {
    100
}

/// Query to fetch message history.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(IntoParams))]
pub struct MessageHistoryQuery {
    /// If specified, only messages before this message will be returned. If any messages exactly
    /// match this ID, they will **not** be returned.
    pub before: Option<u64>,
    /// If specified, only messages after this message will be returned. If any messages exactly
    /// match this ID, they will **not** be returned.
    pub after: Option<u64>,
    /// The limit of messages to return. If unspecified, this defaults to ``100``. Must be between
    /// ``0`` and ``200``.
    #[serde(default = "default_limit")]
    pub limit: u8,
    /// If specified, only messages sent by the given user will be returned.
    pub user_id: Option<u64>,
    /// Whether or not to query messages starting from the oldest message first. Defaults to
    /// ``false``.
    ///
    /// If ``true``, messages will be sorted from oldest to newest. If ``false``, messages will be
    /// sorted from newest to oldest.
    pub oldest_first: bool,
}
