//! Computer user, group, and password management (`ComputerIntrinsics`).

use greyhack_world_router::protocol::{Outcome, UserGroups};

use crate::objects::Access;
use crate::routes::Result;
use crate::routes::conversions::IntoProtocol as _;

/// Changes `user`'s password with root permission.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn change_password(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    user: &str,
    password: &str,
) -> Result<Outcome> {
    Ok(user_outcome(
        world
            .module::<crate::CoreModule>()
            .computers
            .users
            .change_user_password(access.computer, &access.user, user, password)
            .await?,
    ))
}

/// Returns `name`'s group memberships or the lookup failure.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn groups(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    name: &str,
) -> Result<UserGroups> {
    Ok(
        match world
            .module::<crate::CoreModule>()
            .computers
            .users
            .user_groups(access.computer, name)
            .await?
        {
            None => UserGroups::Null,
            Some(Ok(groups)) => UserGroups::Groups(groups),
            Some(Err(failure)) => UserGroups::Failure(failure.into_protocol()),
        },
    )
}

/// Adds `user` to `group` with root permission.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn create_group(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    user: &str,
    group: &str,
) -> Result<Outcome> {
    Ok(user_outcome(
        world
            .module::<crate::CoreModule>()
            .computers
            .users
            .add_user_to_group(access.computer, &access.user, user, group)
            .await?,
    ))
}

/// Removes `user` from `group` with root permission.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete_group(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    user: &str,
    group: &str,
) -> Result<Outcome> {
    Ok(user_outcome(
        world
            .module::<crate::CoreModule>()
            .computers
            .users
            .remove_user_from_group(access.computer, &access.user, user, group)
            .await?,
    ))
}

/// Creates `user` with `password` using root permission.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn create_user(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    user: &str,
    password: &str,
) -> Result<Outcome> {
    Ok(user_outcome(
        world
            .module::<crate::CoreModule>()
            .computers
            .users
            .create_user_account(access.computer, &access.user, user, password)
            .await?,
    ))
}

/// Deletes `name` and optionally its home directory when `remove_home` is set.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete_user(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    name: &str,
    remove_home: bool,
) -> Result<Outcome> {
    Ok(user_outcome(
        world
            .module::<crate::CoreModule>()
            .computers
            .users
            .delete_user_account(access.computer, &access.user, name, remove_home)
            .await?,
    ))
}

/// Converts a user operation result, using `null` once the computer no longer exists.
fn user_outcome(result: Option<std::result::Result<(), crate::ComputerFailure>>) -> Outcome {
    match result {
        None => Outcome::Null,
        Some(Ok(())) => Outcome::True,
        Some(Err(failure)) => Outcome::Failure(failure.into_protocol()),
    }
}
