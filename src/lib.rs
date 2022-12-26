#![allow(clippy::module_name_repetitions)]

pub mod models;
pub mod snowflake;
pub mod ws;

use serde::Serialize;

/// A type alias for a [`Result`] with the error type [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// An error that occurs within Adapt.
#[derive(Debug, Serialize)]
pub enum Error {
    /// Received a malformed JSON or Msgpack body.
    InvalidBody,
}

impl Error {
    /// The HTTP status code associated with this error. If this error is not sent over HTTP, this
    /// will be `None`.
    #[must_use]
    pub const fn http_status_code(&self) -> Option<u16> {
        Some(
            #[allow(unreachable_patterns)]
            match self {
                Self::InvalidBody => 400,
                _ => return None,
            },
        )
    }
}
