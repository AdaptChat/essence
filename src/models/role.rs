use crate::models::PermissionPair;
use crate::serde_for_bitflags;
use serde::Serialize;

/// A role in a guild.
#[derive(Clone, Debug, Serialize)]
pub struct Role {
    /// The snowflake ID of the role.
    pub id: u64,
    /// The ID of the guild this role belongs to.
    pub guild_id: u64,
    /// The name of the role.
    pub name: String,
    /// The color of the role. This is an integer between 0 and 16777215, or ``None`` if the role
    /// has no color (in which case it inherits the color).
    pub color: Option<u32>,
    /// The permissions users with this role have.
    pub permissions: PermissionPair,
    /// The position of this role in the role hierarchy. The lower the number, the lower the role.
    /// The default role always has a position of 0.
    ///
    /// The backend will try its best to keep all role positions unique, but on the event two
    /// collide due to something such as a data race, then the true position of these roles will
    /// not be predictable, and will likely be in the order of model creation.
    pub position: u16,
    /// A bitmask of flags representing extra metadata about the role.
    pub flags: RoleFlags,
}

bitflags::bitflags! {
    #[derive(Default)]
    pub struct RoleFlags: u32 {
        /// Whether the role is hoisted, or shown separately, in member list.
        const HOISTED = 1 << 0;
        /// Whether the role is managed. Managed roles cannot be edited or deleted.
        const MANAGED = 1 << 1;
        /// Whether the role is mentionable.
        const MENTIONABLE = 1 << 2;
        /// Whether the role is the default role for everyone.
        const DEFAULT = 1 << 3;
    }
}

serde_for_bitflags!(u32: RoleFlags);
