use crate::{
    db::DbExt,
    models::{PermissionPair, Permissions, Role, RoleFlags},
};

macro_rules! construct_role {
    ($data:ident) => {{
        Role {
            id: $data.id as _,
            guild_id: $data.guild_id as _,
            name: $data.name,
            color: $data.color.map(|color| color as _),
            position: $data.position as _,
            permissions: PermissionPair {
                allow: Permissions::from_bits_truncate($data.allowed_permissions),
                deny: Permissions::from_bits_truncate($data.denied_permissions),
            },
            flags: RoleFlags::from_bits_truncate($data.flags as _),
        }
    }};
}

#[async_trait::async_trait]
pub trait RoleDbExt<'t>: DbExt<'t> {
    /// Fetches a role from the database with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the role. If the role is not found, `Ok(None)` is
    /// returned.
    async fn fetch_role(&self, guild_id: u64, role_id: u64) -> sqlx::Result<Option<Role>> {
        let role = sqlx::query!(
            "SELECT * FROM roles WHERE guild_id = $1 AND id = $2",
            guild_id as i64,
            role_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|r| construct_role!(r));

        Ok(role)
    }

    /// Fetches all roles from the database in the given guild.
    ///
    /// # Errors
    /// * If an error occurs with fetching the roles.
    /// * If the guild does not exist.
    async fn fetch_all_roles_in_guild(&self, guild_id: u64) -> sqlx::Result<Vec<Role>> {
        let roles = sqlx::query!(
            "SELECT * FROM roles WHERE guild_id = $1 ORDER BY position ASC",
            guild_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| construct_role!(r))
        .collect();

        Ok(roles)
    }
}

impl<'t, T> RoleDbExt<'t> for T where T: DbExt<'t> {}
