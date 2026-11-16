//! Grey Hack failure text for mail accounts and `MetaMail`.

use super::computer::COMPUTER_UNAVAILABLE;
use super::connection::inactive_account;
use crate::protocol::MailFailure;

/// Formats `MetaMailIntrinsics` and `PlayerIntrinsics.mail_login` failures.
pub(super) fn mail_text(intrinsic: &str, failure: MailFailure) -> String {
    match failure {
        MailFailure::NoInternet => format!("{intrinsic}: No internet access."),
        MailFailure::ComputerUnavailable => COMPUTER_UNAVAILABLE.to_owned(),
        MailFailure::WrongCredentials => {
            "Unable to login into account. Wrong user or password.".to_owned()
        }
        MailFailure::AccountInactive => inactive_account(),
        MailFailure::RefreshFailed => "MetaMail: Unable to refresh mail account.".to_owned(),
        MailFailure::NoAccount => "send: Unable to access mail account.".to_owned(),
        MailFailure::NotDelivered => "Mail not delivered".to_owned(),
        MailFailure::RecipientInBlacklist => {
            "The email couldn't be sent. The recipient has blocked this email address.".to_owned()
        }
        MailFailure::BlockedPlayers => {
            "The email couldn't be sent. The recipient has blocked emails sent by other players.".to_owned()
        }
        MailFailure::BlockedNpcs => {
            "The email couldn't be sent. The recipient has blocked emails sent by other players using npcs accounts.".to_owned()
        }
    }
}
