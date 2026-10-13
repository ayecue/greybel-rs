//! Conversions of guild and admin restore failures.

use super::AdminFailure;

impl From<greyhack_world::GuildFailure> for AdminFailure {
    fn from(failure: greyhack_world::GuildFailure) -> Self {
        use greyhack_world::GuildFailure as World;

        match failure {
            World::NameInUse(name) => Self::GuildNameTaken(name),
            World::AccountInUse(account) => Self::GuildAccountTaken(account),
        }
    }
}

impl From<greyhack_world::AdminRestoreFailure> for AdminFailure {
    fn from(failure: greyhack_world::AdminRestoreFailure) -> Self {
        use greyhack_world::AdminRestoreFailure as World;

        match failure {
            World::UnknownGuild(name) => Self::UnknownGuild(name),
            World::InvalidCompany => Self::InvalidCompany,
            World::UnknownCompany => Self::UnknownCompany,
            World::NotGuildCompany => Self::NotGuildCompany,
            World::OverLimit => Self::AdminRestoreLimit,
        }
    }
}

impl From<greyhack_world::CrownShieldFailure> for AdminFailure {
    fn from(failure: greyhack_world::CrownShieldFailure) -> Self {
        use greyhack_world::CrownShieldFailure as World;

        match failure {
            World::UnknownGuild(name) => Self::UnknownGuild(name),
            World::InvalidAddress => Self::InvalidPublicAddress,
            World::NoCrown => Self::NoCrown,
            World::AlreadyActive => Self::CrownShieldActive,
            World::UnknownNetwork(address) => Self::UnknownNetwork(address),
        }
    }
}
