use crate::models::PermissionPair;
use serde::Deserialize;
#[cfg(feature = "client")]
use serde::Serialize;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Payload sent to create a new role in a guild.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateRolePayload {
    /// The name of the role.
    pub name: String,
    /// The color of the role. Leave empty for the default/inherited color.
    pub color: Option<u32>,
    /// The permissions users with this role will have.
    #[serde(default)]
    pub permissions: PermissionPair,
    /// Whether the role should be hoisted.
    #[serde(default)]
    pub hoisted: bool,
    /// Whether the role should be mentionable by anyone.
    #[serde(default)]
    pub mentionable: bool,
}
