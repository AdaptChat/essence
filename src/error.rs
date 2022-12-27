use serde::Serialize;

/// A type alias for a [`Result`] with the error type [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// An error that occurs within Adapt.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Error {
    /// Received a malformed JSON or MsgPack body.
    MalformedBody,
    /// You are missing the request body in an endpoint that requires it. This is commonly JSON
    /// or MsgPack.
    MissingBody {
        /// The error message.
        message: &'static str,
    },
    /// Invalid field in the request body.
    InvalidField {
        /// The field that failed validation.
        field: &'static str,
        /// The error message.
        message: String,
    },
    /// You are missing a required field in the request body.
    MissingField {
        /// The name of the missing field.
        field: &'static str,
        /// The error message.
        message: &'static str,
    },
    /// Could not resolve a plausible IP address from the request.
    MalformedIp {
        /// The error message.
        message: &'static str,
    },
    /// The entity was not found.
    NotFound {
        /// The type of item that couldn't be found.
        entity: &'static str,
        /// The error message.
        message: String,
    },
    /// Tried authorizing a bot account with anything but an authentication token.
    UnsupportedAuthMethod {
        /// The error message.
        message: &'static str,
    },
    /// Invalid login credentials were provided, i.e. an invalid password.
    InvalidCredentials {
        /// Which credential was invalid.
        what: &'static str,
        /// The error message.
        message: &'static str,
    },
    /// Something was already taken, e.g. a username or email.
    AlreadyTaken {
        /// What was already taken.
        what: &'static str,
        /// The error message.
        message: String,
    },
    /// You are sending requests too quickly are you are being rate limited.
    Ratelimited {
        /// How long you should wait before sending another request, in whole seconds.
        retry_after: f32,
        /// The IP address that is being rate limited.
        ip: String,
        /// The ratelimited message.
        message: String,
    },
    /// Internal server error occured, this is likely a bug.
    InternalError {
        /// What caused the error. `None` if unknown.
        what: Option<&'static str>,
        /// The error message.
        message: String,
        /// A debug version of the error, or `None` if there is no debug version.
        debug: Option<String>,
    },
}

impl Error {
    /// The HTTP status code associated with this error. If this error is not sent over HTTP,
    /// this will be `None`.
    #[must_use]
    pub const fn http_status_code(&self) -> Option<u16> {
        Some(match self {
            Self::MalformedBody
            | Self::MissingBody { .. }
            | Self::InvalidField { .. }
            | Self::MissingField { .. }
            | Self::MalformedIp { .. }
            | Self::UnsupportedAuthMethod { .. } => 400,
            Self::InvalidCredentials { .. } => 401,
            Self::NotFound { .. } => 404,
            Self::AlreadyTaken { .. } => 409,
            Self::Ratelimited { .. } => 429,
            Self::InternalError { .. } => 500,
        })
    }
}

#[cfg(feature = "db")]
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::InternalError {
            what: Some("database"),
            message: e.to_string(),
            debug: Some(format!("{e:?}")),
        }
    }
}

#[cfg(feature = "auth")]
impl From<argon2_async::Error> for Error {
    fn from(e: argon2_async::Error) -> Self {
        Self::InternalError {
            what: Some("hasher"),
            message: e.to_string(),
            debug: Some(format!("{e:?}")),
        }
    }
}

/// An extension trait for [`Option`] that adds [`NotFoundExt::ok_or_not_found`].
pub trait NotFoundExt<T> {
    /// Converts an [`Option`] to a [`Result`] with [`Error::NotFound`] if it is [`None`].
    ///
    /// # Example
    /// ```no_run
    /// use essence::error::NotFoundExt;
    ///
    /// assert_eq!(Some(5).ok_or_not_found("user", "user not found"), Ok(5));
    /// ```
    fn ok_or_not_found(self, entity: &'static str, message: impl ToString) -> Result<T>;
}

impl<T> NotFoundExt<T> for Option<T> {
    #[inline]
    fn ok_or_not_found(self, entity: &'static str, message: impl ToString) -> Result<T> {
        self.ok_or_else(|| Error::NotFound {
            entity,
            message: message.to_string(),
        })
    }
}
