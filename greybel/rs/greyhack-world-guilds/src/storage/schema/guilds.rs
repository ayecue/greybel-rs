//! Guilds, companies, and the Crown.

use greyhack_world_storage::TableSet;

pub(crate) const GROUP: TableSet = TableSet::new(
    "-- a guild (`Guilds.Guild`): `bank_account` is the account its company shares are held under and `player_member`
-- whether the player is one of its members; `crown_shield_public_ip` is the address its CrownShield protects
CREATE TABLE IF NOT EXISTS guilds (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    name                   TEXT NOT NULL UNIQUE,
    bank_account           TEXT NOT NULL,
    total_exp              INTEGER NOT NULL DEFAULT 0,
    investment_perks       INTEGER NOT NULL DEFAULT 0,
    admin_restore_perks    INTEGER NOT NULL DEFAULT 0,
    has_crown              INTEGER NOT NULL DEFAULT 0,
    crown_shield_public_ip TEXT NOT NULL DEFAULT '',
    player_member          INTEGER NOT NULL DEFAULT 0
);

-- the companies a guild invested in (`Guild.investedCompanies`)
CREATE TABLE IF NOT EXISTS guild_companies (
    guild_id   INTEGER NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    network_id INTEGER NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    PRIMARY KEY (guild_id, network_id)
);

-- the companies a guild turned Admin Restore off for (`Guild.adminRestoreDisabledCompanies`)
CREATE TABLE IF NOT EXISTS guild_admin_restore_disabled (
    guild_id   INTEGER NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    network_id INTEGER NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    PRIMARY KEY (guild_id, network_id)
);

-- a guild's penalty for a company whose administrator is missing (`Guild.adminAbsencePenalties`); `recovery_day` is the
-- world day recovery started
CREATE TABLE IF NOT EXISTS guild_admin_absences (
    guild_id     INTEGER NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    network_id   INTEGER NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    percent      INTEGER NOT NULL,
    recovery_day INTEGER,
    recovering   INTEGER NOT NULL,
    PRIMARY KEY (guild_id, network_id)
);

-- the shares bank accounts hold of a company, in the order they first bought (`StocksCompany.stocks`)
CREATE TABLE IF NOT EXISTS company_shares (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    network_id   INTEGER NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    bank_account TEXT NOT NULL,
    shares       INTEGER NOT NULL,
    UNIQUE (network_id, bank_account)
);

-- a company's stock-market state: the seed its share price is drawn from (`NetworkLan.seed`; absent when the world
-- derives it) and the world second its DDoS ranking penalty ends (`StocksCompany.ddosRankingPenaltyUntil`)
CREATE TABLE IF NOT EXISTS company_market (
    network_id INTEGER PRIMARY KEY REFERENCES networks(id) ON DELETE CASCADE,
    seed       INTEGER,
    ddos_until INTEGER NOT NULL DEFAULT 0
);

-- the networks a guild's CrownShield protects (`Database.CrownShieldNetworks`)
CREATE TABLE IF NOT EXISTS crown_shield_networks (
    public_ip  TEXT PRIMARY KEY,
    guild_name TEXT NOT NULL UNIQUE
);

-- the guild leading the ranking and since when (`Guilds.crownCandidateGuildName`, `Guilds.crownCandidateSince`)
CREATE TABLE IF NOT EXISTS crown_state (
    id        INTEGER PRIMARY KEY CHECK (id = 0),
    candidate TEXT NOT NULL,
    since     INTEGER NOT NULL
);

",
    &[
    "guilds",
    "guild_companies",
    "guild_admin_restore_disabled",
    "guild_admin_absences",
    "company_shares",
    "company_market",
    "crown_shield_networks",
    "crown_state",
    ],
);
