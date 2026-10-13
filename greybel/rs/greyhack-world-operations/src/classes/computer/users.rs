//! Computer user, group, and password management (`ComputerIntrinsics`).

use greyhack_world::World;

use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::{Outcome, UserGroups};

/// Changes `user`'s password with root permission.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn change_password(
    world: &mut World,
    access: &Access,
    user: &str,
    password: &str,
) -> Result<Outcome> {
    Ok(world
        .change_user_password(access.computer, &access.user, user, password)
        .await?
        .into())
}

/// Returns `name`'s group memberships or the lookup failure.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn groups(world: &World, access: &Access, name: &str) -> Result<UserGroups> {
    Ok(match world.user_groups(access.computer, name).await? {
        None => UserGroups::Null,
        Some(Ok(groups)) => UserGroups::Groups(groups),
        Some(Err(failure)) => UserGroups::Failure(failure.into()),
    })
}

/// Adds `user` to `group` with root permission.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn create_group(
    world: &mut World,
    access: &Access,
    user: &str,
    group: &str,
) -> Result<Outcome> {
    Ok(world
        .add_user_to_group(access.computer, &access.user, user, group)
        .await?
        .into())
}

/// Removes `user` from `group` with root permission.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete_group(
    world: &mut World,
    access: &Access,
    user: &str,
    group: &str,
) -> Result<Outcome> {
    Ok(world
        .remove_user_from_group(access.computer, &access.user, user, group)
        .await?
        .into())
}

/// Creates `user` with `password` using root permission.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn create_user(
    world: &mut World,
    access: &Access,
    user: &str,
    password: &str,
) -> Result<Outcome> {
    Ok(world
        .create_user_account(access.computer, &access.user, user, password)
        .await?
        .into())
}

/// Deletes `name` and optionally its home directory when `remove_home` is set.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete_user(
    world: &mut World,
    access: &Access,
    name: &str,
    remove_home: bool,
) -> Result<Outcome> {
    Ok(world
        .delete_user_account(access.computer, &access.user, name, remove_home)
        .await?
        .into())
}
