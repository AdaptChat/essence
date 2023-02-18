use serde::{Deserialize, Serialize};
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// The method to use to retrieve an authentication token.
#[derive(Copy, Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum TokenRetrievalMethod {
    /// Generate a new token as a new session. This will keep old tokens but add an alternate token
    /// to the user's account.
    New,
    /// Generate a new token, revoking all other sessions. This will log out all other sessions.
    Revoke,
    /// Use an existing token if one exists, else create a new one. This is the default.
    #[default]
    Reuse,
}

/// The request body for POST /login
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct LoginRequest {
    /// The email to log in with.
    pub email: String,
    /// The password to log in with.
    #[cfg_attr(feature = "utoipa", schema(format = "password"))]
    pub password: String,
    /// The token retrieval method to use.
    #[serde(default)]
    pub method: TokenRetrievalMethod,
}

/// The response body for POST /login
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "client", derive(Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct LoginResponse {
    /// The user ID of the logged in user.
    pub user_id: u64,
    /// The authentication token to use for future requests.
    pub token: String,
}
