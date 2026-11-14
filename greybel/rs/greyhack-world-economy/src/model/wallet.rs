//! Wallets with per-coin balance rows.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WalletId(pub(crate) u32);

impl WalletId {
    /// Converts `number` to a typed entity ID.
    #[must_use]
    pub const fn new(number: u32) -> Self {
        Self(number)
    }

    /// Converts a typed ID to its numeric representation.
    #[must_use]
    pub const fn number(self) -> u32 {
        self.0
    }
}

/// Wallet containing balances for multiple coins (`BlockchainSystem.Wallet`).
#[derive(Debug, Clone, PartialEq)]
pub struct Wallet {
    pub id: WalletId,
    pub user: String,
    pub password_hash: String,
    pub pin: String,
    /// Bank account used for trade settlement.
    pub bank: String,
    /// Next world second when password reset is allowed.
    pub next_password_reset_at: Option<u64>,
}
