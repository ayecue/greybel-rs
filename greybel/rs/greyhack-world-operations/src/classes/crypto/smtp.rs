//! SMTP account listing (`CryptoIntrinsics.smtp_user_list`).

use greyhack_world::World;

use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::SmtpUserList;

/// Lists SMTP accounts at `address:port` ([`World::smtp_user_list`]).
pub(super) async fn smtp_user_list(
    world: &mut World,
    access: &Access,
    address: &str,
    port: u16,
) -> Result<SmtpUserList> {
    Ok(
        match world.smtp_users(access.computer, address, port).await? {
            Ok(users) => SmtpUserList::Users(users),
            Err(failure) => SmtpUserList::Failure(failure.into()),
        },
    )
}
