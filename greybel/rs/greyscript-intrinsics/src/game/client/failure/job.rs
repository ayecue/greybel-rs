//! Grey Hack failure text for hiring, firing, and admin rental payments.

use crate::protocol::JobFailure;

/// Formats `NetworkLan.FireNpc`, `BankHelperServer.HireNpcServerRpc`, `HireAdminRentalServerRpc`, `FireAdminRentalServerRpc`
/// and `BankHelperServer.PagarCompra` failures.
pub(super) fn job_text(failure: &JobFailure) -> String {
    match failure {
        JobFailure::UnknownPerson => "Unknown error: Unable to find person to fire.".to_owned(),
        JobFailure::Unavailable => "This person cannot be fired.".to_owned(),
        JobFailure::HiredRecently => {
            "You cannot fire this person until one month has passed since their hiring.".to_owned()
        }
        JobFailure::InvalidJob => "Error: invalid job option".to_owned(),
        JobFailure::NoVacancy(role) => format!(
            "There are no open positions in the company for {}",
            role.replace('_', " ")
        ),
        JobFailure::AdminExists => {
            "You already have an administrator hired for this network.".to_owned()
        }
        JobFailure::NoAdmin => "You don't have any administrator hired on this network.".to_owned(),
        JobFailure::NoBankAccount => "You must register a bank account in order to buy".to_owned(),
        JobFailure::UnknownBankAccount(user) => format!("{user} user not found"),
        JobFailure::InsufficientFunds => {
            "The purchase could not be completed. Insufficient funds".to_owned()
        }
        JobFailure::NoAdminPayment => {
            "Error: you do not have any monthly charge registered associated with an admin"
                .to_owned()
        }
    }
}
