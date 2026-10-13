//! Guilds, company ownership and the Crown.

use greyhack_behaviour::AdminAbsence;

use super::network::NetworkId;

/// Guild row ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuildId(pub(crate) u32);

/// A guild (`Guilds.Guild`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guild {
    pub id: GuildId,
    pub name: String,
    /// Bank account its company shares are stored under (`Guild.bankAccount`).
    pub bank_account: String,
    /// Experience of the guild (`Guild.totalExp`).
    pub total_exp: u32,
    /// Investment licences it bought (`Guild.purchasedInvestmentLicensePerks`).
    pub investment_perks: u32,
    /// Admin Restore perks it bought (`Guild.purchasedAdminRestorePerks`).
    pub admin_restore_perks: u32,
    /// Whether it has the Crown (`Guild.hasCrown`).
    pub has_crown: bool,
    /// Public address its CrownShield protects, empty without one (`Guild.crownShieldPublicIp`).
    pub crown_shield_public_ip: String,
    /// Whether the player is a member (`Guild.GetMemberByPlayerID`).
    pub player_member: bool,
}

/// What a script defines for a guild (`Guilds.Guild` as the guild service stores it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildDraft {
    pub name: String,
    pub bank_account: String,
    pub total_exp: u32,
    pub investment_perks: u32,
    pub admin_restore_perks: u32,
    pub player_member: bool,
}

/// Why a guild could not be created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildFailure {
    /// A guild has the name.
    NameInUse(String),
    /// A guild has the bank account.
    AccountInUse(String),
}

/// Why CrownShield could not be hired for a guild.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrownShieldFailure {
    /// No guild has the name.
    UnknownGuild(String),
    /// The address is not a valid public address (`PanelMainGuild.OnHireCrownShield`).
    InvalidAddress,
    /// The guild does not have the Crown (`PanelMainGuild.ConfigCrownShield`).
    NoCrown,
    /// The guild already has a CrownShield.
    AlreadyActive,
    /// No network has the address.
    UnknownNetwork(String),
}

/// Why Admin Restore could not be switched for a company of a guild
/// (`NetworkHelperServer.AdminCompanySetAdminRestoreServerRpc`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminRestoreFailure {
    /// No guild has the name.
    UnknownGuild(String),
    /// The address is not a valid public address.
    InvalidCompany,
    /// No network has the address.
    UnknownCompany,
    /// The guild's bank account is not the company's majority owner.
    NotGuildCompany,
    /// The guild is at its limit of companies with Admin Restore off at its current perk level.
    OverLimit,
}

/// A company's shares of one bank account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stake {
    pub bank_account: String,
    pub shares: u32,
}

/// A guild's penalty record for a company.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildAbsence {
    pub network: NetworkId,
    pub record: AdminAbsence,
}

/// The guild leading the ranking and since when (world second; 0 for none).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CrownState {
    pub candidate: String,
    pub since: u64,
}

/// A guild as Grey Hack has it, copied into a mirrored world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildLayout {
    pub draft: GuildDraft,
    /// Public addresses of the companies it turned Admin Restore off for (`Guild.adminRestoreDisabledCompanies`).
    pub admin_restore_off: Vec<String>,
    pub has_crown: bool,
    pub crown_shield_public_ip: String,
}

/// A company's shareholders and market seed as Grey Hack has them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanyLayout {
    pub public_ip: String,
    pub seed: i32,
    /// Bank accounts and their shares, in the order they first bought.
    pub holders: Vec<(String, u32)>,
}

/// Guilds, companies and CrownShield networks as Grey Hack has them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuildsLayout {
    pub guilds: Vec<GuildLayout>,
    pub companies: Vec<CompanyLayout>,
    /// Protected addresses and the guilds that hired CrownShield for them.
    pub shields: Vec<(String, String)>,
}
