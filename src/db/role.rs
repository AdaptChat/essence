use crate::{
    db::DbExt,
    http::role::CreateRolePayload,
    models::{Role, RoleFlags},
    Error, NotFoundExt,
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

use crate::db::{get_pool, GuildDbExt};
use crate::http::role::EditRolePayload;
pub(crate) use construct_role;

#[async_trait::async_trait]
pub trait RoleDbExt<'t>: DbExt<'t> {
    /// Asserts the role exists and returns the position of the role.
    async fn assert_role_exists(&self, guild_id: u64, role_id: u64) -> crate::Result<u16> {
        self.assert_guild_exists(guild_id).await?;

        let role = sqlx::query!(
            "SELECT position FROM roles WHERE guild_id = $1 AND id = $2",
            guild_id as i64,
            role_id as i64,
        )
        .fetch_optional(self.executor())
        .await?;

        role.map_or_else(
            || {
                Err(Error::NotFound {
                    entity: "role",
                    message: format!("Role with ID {role_id} does not exist"),
                })
            },
            |role| Ok(role.position as u16),
        )
    }

    /// Asserts the user's top role is higher than the given role in the given guild.
    async fn assert_top_role_higher_than(
        &self,
        guild_id: u64,
        user_id: u64,
        role_id: u64,
    ) -> crate::Result<()> {
        let role_position = self.assert_role_exists(guild_id, role_id).await?;
        let (top_role_id, top_position) = sqlx::query!(
            r#"SELECT
                id,
                position
            FROM
                roles
            WHERE
                id = (SELECT role_id FROM role_data WHERE user_id = $1 AND guild_id = $2)
            ORDER BY
                position DESC
            LIMIT 1
            "#,
            user_id as i64,
            guild_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map_or((None, 0), |row| (Some(row.id as u64), row.position as u16));

        if role_position >= top_position {
            return Err(Error::RoleTooLow {
                guild_id,
                top_role_id,
                top_role_position: top_position,
                desired_position: role_position,
                message:
                    "You can only perform the requested action on roles lower than your top role.",
            });
        }

        Ok(())
    }

    /// Asserts that the given role is not managed.
    async fn assert_role_is_not_managed(&self, guild_id: u64, role_id: u64) -> crate::Result<()> {
        let is_managed = sqlx::query!(
            "SELECT flags FROM roles WHERE guild_id = $1 AND id = $2",
            guild_id as i64,
            role_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map_or(false, |row| {
            RoleFlags::from_bits_truncate(row.flags as _).contains(RoleFlags::MANAGED)
        });

        if is_managed {
            return Err(Error::RoleIsManaged {
                guild_id,
                role_id,
                message: "You cannot delete a managed role.",
            });
        }

        Ok(())
    }

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

    /// Edits the role with the given ID in the given guild. Payload must be validated before using
    /// this method.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with editing the role.
    /// * If the role does not exist.
    async fn edit_role(
        &mut self,
        guild_id: u64,
        role_id: u64,
        payload: EditRolePayload,
    ) -> crate::Result<Role> {
        let mut role = get_pool()
            .fetch_role(guild_id, role_id)
            .await?
            .ok_or_not_found("role", "role not found")?;

        if let Some(name) = payload.name {
            role.name = name;
        }
        if let Some(permissions) = payload.permissions {
            role.permissions = permissions;
        }
        if let Some(mentionable) = payload.mentionable {
            role.flags.set(RoleFlags::MENTIONABLE, mentionable);
        }
        if let Some(hoisted) = payload.hoisted {
            role.flags.set(RoleFlags::HOISTED, hoisted);
        }
        role.color = payload.color.into_option_or_if_absent(role.color);

        sqlx::query!(
            r#"UPDATE
                roles
            SET
                name = $1,
                color = $2,
                allowed_permissions = $3,
                denied_permissions = $4,
                flags = $5
            WHERE
                guild_id = $6
            AND
                id = $7
            "#,
            role.name,
            role.color.map(|color| color as i32),
            role.permissions.allow.bits(),
            role.permissions.deny.bits(),
            role.flags.bits() as i32,
            guild_id as i64,
            role_id as i64,
        )
        .execute(self.transaction())
        .await?;

        Ok(role)
    }

    /// Deletes the role with the given ID in the given guild.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the role.
    /// * If the role does not exist.
    async fn delete_role(&mut self, guild_id: u64, role_id: u64) -> crate::Result<()> {
        let position = sqlx::query!(
            "DELETE FROM roles WHERE guild_id = $1 AND id = $2 RETURNING position",
            guild_id as i64,
            role_id as i64,
        )
        .fetch_one(self.transaction())
        .await?
        .position;

        sqlx::query!(
            "UPDATE roles SET position = position - 1 WHERE position > $1",
            position as i16,
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }
}

impl<'t, T> RoleDbExt<'t> for T where T: DbExt<'t> {}
