use crate::models::PermissionPair;
use crate::Maybe;
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

/// Payload sent to edit a role.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "client", derive(Serialize))]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EditRolePayload {
    /// The new name of the role, if any.
    pub name: Option<String>,
    /// The color of the role. Set to `null` for the default/inherited color, and leave empty
    /// to leave it alone
    #[serde(default)]
    #[cfg_attr(feature = "client", serde(skip_serializing_if = "Maybe::is_absent"))]
    #[cfg_attr(feature = "openapi", schema(nullable, value_type = Option<u32>))]
    pub color: Maybe<u32>,
    /// The permissions users with this role will have. Both `allow` and `deny` should be specified
    /// if this field is specified.
    pub permissions: Option<PermissionPair>,
    /// Whether the role should be hoisted.
    pub hoisted: Option<bool>,
    /// Whether the role should be mentionable by anyone.
    pub mentionable: Option<bool>,
}
