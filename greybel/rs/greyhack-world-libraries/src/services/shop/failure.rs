//! Why the exploit shop did not list or sell an exploit.

use crate::{FileFailure, PurchaseFailure};

/// Why the exploit shop did not list or sell an exploit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExploitShopFailure {
    /// The version is not three integers (`NetworkHelperServer.GetExploitListServerRpc`).
    InvalidVersion,
    /// The library version has no exploits.
    NoResults,
    /// The destination folder is not a valid path (`BankHelperServer.BuyItem`).
    InvalidPath,
    /// The computer that receives the file no longer exists.
    UnknownComputer,
    /// The computer that receives the file has no network access.
    NoInternet,
    /// The destination folder is not there and cannot be made.
    MissingFolder(String),
    /// The user cannot create a file in the destination folder.
    NoCreatePermission { folder: String, name: String },
    /// The shop has no such exploit (`BankHelperServer.DownloadExploit`).
    UnknownExploit,
    /// The player has no bank account (`BankHelperServer.PagarCompra`).
    NoBankAccount,
    /// The player's bank account is not in the bank.
    UnknownBankAccount(String),
    /// The account cannot pay the price after the coupons.
    InsufficientFunds,
    /// The file was not created after the purchase was paid (`PlayerUtils.CrearArchivo`).
    File(FileFailure),
}

impl From<PurchaseFailure> for ExploitShopFailure {
    fn from(failure: PurchaseFailure) -> Self {
        match failure {
            PurchaseFailure::NoBankAccount => Self::NoBankAccount,
            PurchaseFailure::UnknownBankAccount(number) => Self::UnknownBankAccount(number),
            PurchaseFailure::InsufficientFunds => Self::InsufficientFunds,
        }
    }
}
