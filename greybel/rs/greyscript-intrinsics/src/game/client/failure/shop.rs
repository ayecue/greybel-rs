//! Grey Hack text for exploit shop failures.

use super::file::file_text;
use crate::protocol::ShopFailure;

/// Formats `failure` as the hack shop's window shows it (`NetworkHelperServer.GetExploitListServerRpc`,
/// `BankHelperServer.BuyItemServerRpc`, `BankHelperServer.DownloadExploit`, `BankHelperServer.PagarCompra`).
pub(super) fn shop_text(intrinsic: &str, failure: &ShopFailure) -> String {
    match failure {
        ShopFailure::InvalidVersion => "Invalid version format".to_owned(),
        ShopFailure::NoResults => "No results found".to_owned(),
        ShopFailure::InvalidPath => "Error: Invalid path".to_owned(),
        ShopFailure::UnknownComputer => "Can't download the file. Unknown error.".to_owned(),
        ShopFailure::NoInternet => "No internet access".to_owned(),
        ShopFailure::MissingFolder(folder) => format!("Error: {folder} not found."),
        ShopFailure::NoCreatePermission { folder, name } => {
            format!("Can't create file {folder}/{name}. Permission denied.")
        }
        ShopFailure::UnknownExploit => "Error: exploit not found in server.".to_owned(),
        ShopFailure::NoBankAccount => "You must register a bank account in order to buy".to_owned(),
        ShopFailure::UnknownBankAccount(user) => format!("{user} user not found"),
        ShopFailure::InsufficientFunds => {
            "The purchase could not be completed. Insufficient funds".to_owned()
        }
        ShopFailure::File(failure) => file_text(intrinsic, failure),
    }
}
