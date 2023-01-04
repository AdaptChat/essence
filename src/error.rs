use crate::models::Permissions;
#[cfg(feature = "client")]
use serde::Deserialize;
use serde::Serialize;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// A type alias for a [`Result`] with the error type [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Copy, Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MalformedBodyErrorType {
    /// Invalid content type.
    InvalidContentType,
    /// Body was invalid UTF-8.
    InvalidUtf8,
    /// Received invalid JSON body.
    InvalidJson,
}

/// An error that occurs within Adapt.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Error {
    /// Received a malformed JSON or MsgPack body.
    MalformedBody {
        /// Extra information about the error.
        error_type: MalformedBodyErrorType,
        /// A generalized message about the error.
        message: String,
    },
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
    /// The request required a valid authentication token, but one of the following happened:
    ///
    /// * The token was not provided.
    /// * The token was malformed, i.e. a non-UTF-8 string.
    /// * The token does not exist or is invalid.
    InvalidToken {
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
    /// You must be a member of the guild to perform the requested action.
    NotMember {
        /// The ID of the guild you are not a member of.
        guild_id: u64,
        /// The error message.
        message: &'static str,
    },
    /// You must be the owner of the guild to perform the requested action.
    NotOwner {
        /// The ID of the guild you are not the owner of.
        guild_id: u64,
        /// The error message.
        message: &'static str,
    },
    /// You are too low in the role hierarchy to perform the requested action.
    RoleTooLow {
        /// The ID of the guild you are not the owner of.
        guild_id: u64,
        /// The ID of your top role. This is the role you possess with the highest position.
        /// This is `None` if you have no roles (the default role).
        top_role_id: Option<u64>,
        /// The position of your top role.
        top_role_position: u16,
        /// The desired position your top role should be in the role hierarchy.
        desired_position: u16,
        /// The error message.
        message: &'static str,
    },
    /// You are missing the required permissions to perform the requested action.
    MissingPermissions {
        /// The ID of the guild you are missing permissions in.
        guild_id: u64,
        /// The permissions required to perform the requested action.
        permissions: Permissions,
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
            Self::MalformedBody { .. }
            | Self::MissingBody { .. }
            | Self::InvalidField { .. }
            | Self::MissingField { .. }
            | Self::MalformedIp { .. }
            | Self::UnsupportedAuthMethod { .. } => 400,
            Self::InvalidToken { .. } | Self::InvalidCredentials { .. } => 401,
            Self::NotMember { .. }
            | Self::NotOwner { .. }
            | Self::MissingPermissions { .. }
            | Self::RoleTooLow { .. } => 403,
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
