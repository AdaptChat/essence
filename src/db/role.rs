use crate::{
    cache,
    db::DbExt,
    http::role::CreateRolePayload,
    models::{Role, RoleFlags},
    Error,
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
use crate::models::ModelType;
use crate::snowflake::with_model_type;
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
                    entity: "role".to_string(),
                    message: format!("Role with ID {role_id} does not exist"),
                })
            },
            |role| Ok(role.position as u16),
        )
    }
    /// Fetches the ID and position of the top role of the given user in the given guild.
    async fn fetch_top_role(&self, guild_id: u64, user_id: u64) -> crate::Result<(u64, u16)> {
        self.assert_guild_exists(guild_id).await?;

        let role = sqlx::query!(
            r#"SELECT
                r.id,
                r.position
            FROM roles r
            INNER JOIN
                role_data rd
            ON
                r.id = rd.role_id
            WHERE
                r.guild_id = $1 AND rd.user_id = $2
            ORDER BY
                r.position DESC
            LIMIT 1
            "#,
            guild_id as i64,
            user_id as i64,
        )
        .fetch_optional(self.executor())
        .await?;

        let info = role.map_or_else(
            || (with_model_type(guild_id, ModelType::Role), 0),
            |role| (role.id as u64, role.position as u16),
        );
        Ok(info)
    }

    /// Asserts the user's top role is higher than the given role in the given guild.
    async fn assert_top_role_higher_than(
        &self,
        guild_id: u64,
        user_id: u64,
        role_id: u64,
    ) -> crate::Result<()> {
        let role_position = self.assert_role_exists(guild_id, role_id).await?;
        let (top_role_id, top_position) = self.fetch_top_role(guild_id, user_id).await?;

        if role_position >= top_position && !self.is_guild_owner(guild_id, user_id).await? {
            return Err(Error::RoleTooLow {
                guild_id,
                top_role_id,
                top_role_position: top_position,
                desired_position: role_position,
                message: String::from(
                    "You can only perform the requested action on roles lower than your top role.",
                ),
            });
        }

        Ok(())
    }

    /// Asserts the invoker's top role is higher than the given target's top role in the given
    /// guild.
    async fn assert_top_role_higher_than_target(
        &self,
        guild_id: u64,
        invoker_id: u64,
        target_id: u64,
    ) -> crate::Result<()> {
        let (invoker_top_role_id, invoker_top_position) =
            self.fetch_top_role(guild_id, invoker_id).await?;
        let (_, target_top_position) = self.fetch_top_role(guild_id, target_id).await?;

        if invoker_top_position <= target_top_position
            && !self.is_guild_owner(guild_id, invoker_id).await?
        {
            return Err(Error::RoleTooLow {
                guild_id,
                top_role_id: invoker_top_role_id,
                top_role_position: invoker_top_position,
                desired_position: target_top_position,
                message: String::from(
                    "You can only perform the requested action on users with a lower top role than your own.",
                ),
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
                message: "You cannot delete a managed role.".to_string(),
            });
        }

        Ok(())
    }

    /// Returns the highest position of the given roles by their IDs.
    /// If no roles are given, returns 0.
    ///
    /// # Errors
    /// * If an error occurs within the database.
    async fn fetch_highest_position_in(
        &self,
        guild_id: u64,
        role_ids: &[u64],
    ) -> crate::Result<u16> {
        sqlx::query!(
            r#"SELECT
                position
            FROM
                roles
            WHERE
                guild_id = $1
            AND
                id = ANY($2)
            ORDER BY
                position DESC
            LIMIT 1
            "#,
            guild_id as i64,
            &role_ids.iter().map(|id| *id as i64).collect::<Vec<_>>(),
        )
        .fetch_optional(self.executor())
        .await?
        .map_or(Ok(0), |row| Ok(row.position as u16))
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

    /// Fetches all roles from the databased in the given guild assigned to the given member.
    ///
    /// # Errors
    /// * If an error occurs with fetching the roles.
    /// * If the guild does not exist.
    async fn fetch_all_roles_for_member(
        &self,
        guild_id: u64,
        member_id: u64,
    ) -> sqlx::Result<Vec<Role>> {
        let default_role_id = with_model_type(guild_id, ModelType::Role);
        let roles = sqlx::query!(
            r#"SELECT
                *
            FROM
                roles
            WHERE
                guild_id = $1
            AND (
                id = $3
                OR id IN (SELECT role_id FROM role_data WHERE guild_id = $1 AND user_id = $2)
            )
            ORDER BY position ASC
            "#,
            guild_id as i64,
            member_id as i64,
            default_role_id as i64,
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

        sqlx::query!(
            "UPDATE roles SET position = position + 1 WHERE guild_id = $1 AND position >= $2",
            guild_id as i64,
            payload.position as i16,
        )
        .execute(self.transaction())
        .await?;

        sqlx::query!(
            r#"INSERT INTO roles
                (id, guild_id, name, color, allowed_permissions, denied_permissions, position, flags)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            role_id as i64,
            guild_id as i64,
            payload.name,
            payload.color.map(|color| color as i32),
            payload.permissions.allow.bits(),
            payload.permissions.deny.bits(),
            payload.position as i16,
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
        mut role: Role,
        payload: EditRolePayload,
    ) -> crate::Result<(Role, Role)> {
        let old = role.clone();
        let role_id = role.id;

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

        cache::clear_member_permissions(guild_id).await?;
        Ok((old, role))
    }

    /// Edits the ordering of roles in the given guild in bulk. A slice of role IDs must be provided
    /// as ``role_ids`` from lowest to highest position. **All** roles **except the default role**
    /// must be provided in the slice. Although roles ``>=`` than the invoker's top role **must**
    /// be included in the slice, they **must** remain in their original positions (unless owner).
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with editing the roles.
    /// * If the guild does not exist.
    /// * If any of the roles with IDs in ``role_ids`` do not exist.
    /// * If the default role is included in the slice.
    /// * If the length of ``role_ids`` does not match the number of roles in the guild, i.e.
    ///  ``role_ids.len() != number of roles in the guild`` (excluding default role).
    /// * If a role in ``role_ids`` which is higher than or equal to the invoker's top role is not
    ///   in its original position, unless the invoker owns the guild.
    async fn edit_role_positions(
        &mut self,
        guild_id: u64,
        role_ids: &[u64],
        user_id: u64,
    ) -> crate::Result<()> {
        let pool = get_pool();
        let (top_role_id, top_role_position) = pool.fetch_top_role(guild_id, user_id).await?;
        let is_owner = pool.is_guild_owner(guild_id, user_id).await?;

        let default_role_id = with_model_type(guild_id, ModelType::Role);
        let roles = sqlx::query!(
            "SELECT id, position FROM roles WHERE guild_id = $1 AND id != $2",
            guild_id as i64,
            default_role_id as i64,
        )
        .fetch_all(pool)
        .await?;

        if roles.len() != role_ids.len() {
            return Err(Error::InvalidField {
                field: "role_ids".to_string(),
                message: format!(
                    "Expected to reorder {} roles, but {} were provided",
                    roles.len(),
                    role_ids.len(),
                ),
            });
        }

        let mut ids = Vec::with_capacity(roles.len());
        let mut positions = Vec::with_capacity(roles.len());

        for (i, &role_id) in role_ids.iter().enumerate() {
            let role = roles
                .iter()
                .find(|r| r.id as u64 == role_id)
                .ok_or_else(|| Error::NotFound {
                    entity: "role".to_string(),
                    message: format!(
                        "Role with ID {role_id} is the default role or does not exist"
                    ),
                })?;

            let position = (i + 1) as i16;
            if role.position != position {
                if !is_owner && role.position >= top_role_position as _ {
                    return Err(Error::RoleTooLow {
                        guild_id,
                        top_role_id,
                        top_role_position,
                        desired_position: role.position as _,
                        message: String::from(
                            "You can only change the position of roles lower than your top role.",
                        ),
                    });
                }
                ids.push(role_id as i64);
                positions.push(position);
            }
        }

        sqlx::query(
            r"UPDATE
                roles
            SET
                position = p.position
            FROM
                (SELECT UNNEST($1::BIGINT[]) AS id, UNNEST($2::SMALLINT[]) AS position) AS p
            WHERE
                roles.id = p.id
            ",
        )
        .bind(&ids)
        .bind(&positions)
        .execute(self.transaction())
        .await?;

        Ok(())
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
            "UPDATE roles SET position = position - 1 WHERE guild_id = $1 AND position > $2",
            guild_id as i64,
            position as i16,
        )
        .execute(self.transaction())
        .await?;

        cache::clear_member_permissions(guild_id).await?;
        Ok(())
    }
}

impl<'t, T> RoleDbExt<'t> for T where T: DbExt<'t> {}
