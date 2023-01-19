use crate::models::{PermissionOverwrite, Permissions, Role};

/// Calculates the permissions after applying all role permissions and channel overwrites.
/// This mutates `roles` by sorting it by position.
///
/// # Note
/// This does not account for guild owners (they should have all permissions), this should be
/// handled by the caller.
///
/// # Parameters
/// * `user_id` - The ID of the user to calculate permissions for.
/// * `roles` - The roles the user has.
/// * `overwrites` - The channel overwrites, or `None` to apply no overwrites.
#[must_use]
pub fn calculate_permissions(
    user_id: u64,
    mut roles: impl AsMut<[Role]>,
    overwrites: Option<&[PermissionOverwrite]>,
) -> Permissions {
    let mut roles = roles.as_mut();
    roles.sort_by_key(|r| r.position);

    calculate_permissions_sorted(user_id, roles, overwrites)
}

/// Calculates the permissions after applying all role permissions and channel overwrites.
/// This assumes `roles` is sorted by position.
///
/// # Note
/// This does not account for guild owners (they should have all permissions), this should be
/// handled by the caller.
///
/// # Parameters
/// * `user_id` - The ID of the user to calculate permissions for.
/// * `roles` - The roles the user has.
/// * `overwrites` - The channel overwrites, or `None` to apply no overwrites.
#[must_use]
pub fn calculate_permissions_sorted(
    user_id: u64,
    roles: impl AsRef<[Role]>,
    overwrites: Option<&[PermissionOverwrite]>,
) -> Permissions {
    let base = Permissions::empty();
    let roles = roles.as_ref();

    let mut perms = roles
        .iter()
        .fold(base, |acc, role| acc | role.permissions.allow);
    perms &= !roles
        .iter()
        .fold(base, |acc, role| acc | role.permissions.deny);

    // currently, administrator acts after denied perms, meaning administrator does *not* take
    // precedence when a higher role denies the administrator permission. this could change in the
    // future
    if perms.contains(Permissions::ADMINISTRATOR) {
        return Permissions::all();
    }

    if let Some(overwrites) = overwrites {
        let mut role_overwrites = overwrites
            .iter()
            .filter_map(|o| roles.iter().find(|r| r.id == o.id).map(|r| (o, r.position)))
            .collect::<Vec<_>>();

        role_overwrites.sort_by_key(|(_, pos)| *pos);

        for (overwrite, _) in role_overwrites {
            perms |= overwrite.permissions.allow;
            perms &= !overwrite.permissions.deny;
        }

        if let Some(o) = overwrites.iter().find(|o| o.id == user_id) {
            perms |= o.permissions.allow;
            perms &= !o.permissions.deny;
        }
    }

    perms
}
