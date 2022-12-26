//! Common object models consumed by Adapt's services.

mod user;

use std::fmt;
pub use user::*;

/// An enumeration for the type of a model, which takes up 5 bits in a snowflake.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ModelType {
    /// The model is a guild.
    Guild = 0,
    /// The model is a user account.
    User = 1,
    /// The model is a channel.
    Channel = 2,
    /// The model is a message.
    Message = 3,
    /// The model is a role.
    Role = 4,
    /// The model is used internally, e.g. a nonce.
    Internal = 5,
    /// Unknown model.
    Unknown = 31,
}

impl ModelType {
    /// Returns the corresponding model type for the given integer.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Guild,
            1 => Self::User,
            2 => Self::Channel,
            3 => Self::Message,
            4 => Self::Role,
            5 => Self::Internal,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for ModelType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Guild => "guild",
                Self::User => "user",
                Self::Channel => "channel",
                Self::Message => "message",
                Self::Role => "role",
                Self::Internal => "internal",
                Self::Unknown => "unknown",
            }
        )
    }
}
