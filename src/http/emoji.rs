use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// The payload sent to create a new emoji.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CreateEmojiPayload {
    /// The name of the emoji.
    pub name: String,
    /// The emoji image.
    /// The image should be represented as a
    /// [Data URI scheme](https://en.wikipedia.org/wiki/Data_URI_scheme).
    pub image: String,
}

/// The payload sent to modify an emoji.
///
/// # Note
/// The image of an emoji is immutable. To change the image, create a new emoji instead.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct EditEmojiPayload {
    /// The new name of the emoji.
    pub name: String,
}
