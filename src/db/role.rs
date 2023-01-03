use crate::{
    db::DbExt,
    http::role::CreateRolePayload,
    models::{Role, RoleFlags},
};

macro_rules! construct_role {
    ($data:ident) => {{
        use $crate::models::{PermissionPair, Permissions, RoleFlags};

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

pub(crate) use construct_role;

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

    /// Creates a new role in the given guild ID with the given query. Payload must be validated
    /// before using this method.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with creatimg the role.
    async fn create_role(
        &mut self,
        guild_id: u64,
        role_id: u64,
        payload: CreateRolePayload,
    ) -> crate::Result<Role> {
        let mut flags = RoleFlags::default();
        if payload.hoisted {
            flags.insert(RoleFlags::HOISTED);
        }
        if payload.mentionable {
            flags.insert(RoleFlags::MENTIONABLE);
        }

        sqlx::query!("UPDATE roles SET position = position + 1 WHERE position > 0")
            .execute(self.transaction())
            .await?;

        sqlx::query!(
            r#"INSERT INTO roles
                (id, guild_id, name, color, allowed_permissions, denied_permissions, position, flags)
            VALUES
                ($1, $2, $3, $4, $5, $6, 1, $7)
            "#,
            role_id as i64,
            guild_id as i64,
            payload.name,
            payload.color.map(|color| color as i32),
            payload.permissions.allow.bits(),
            payload.permissions.deny.bits(),
            flags.bits() as i32,
        )
        .execute(self.transaction())
        .await?;

        Ok(Role {
            id: role_id,
            guild_id,
            name: payload.name,
            color: payload.color,
            permissions: payload.permissions,
            position: 1,
            flags,
        })
    }
}

impl<'t, T> RoleDbExt<'t> for T where T: DbExt<'t> {}
