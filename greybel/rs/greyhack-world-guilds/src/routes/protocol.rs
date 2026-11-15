//! Conversions of guild and admin restore failures.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
use greyhack_world_programs::protocol::AdminFailure;

impl IntoProtocol for crate::GuildFailure {
    type Output = AdminFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::GuildFailure as World;

        match failure {
            World::NameInUse(name) => AdminFailure::GuildNameTaken(name),
            World::AccountInUse(account) => AdminFailure::GuildAccountTaken(account),
        }
    }
}

impl IntoProtocol for crate::AdminRestoreFailure {
    type Output = AdminFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::AdminRestoreFailure as World;

        match failure {
            World::UnknownGuild(name) => AdminFailure::UnknownGuild(name),
            World::InvalidCompany => AdminFailure::InvalidCompany,
            World::UnknownCompany => AdminFailure::UnknownCompany,
            World::NotGuildCompany => AdminFailure::NotGuildCompany,
            World::OverLimit => AdminFailure::AdminRestoreLimit,
        }
    }
}

impl IntoProtocol for crate::CrownShieldFailure {
    type Output = AdminFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::CrownShieldFailure as World;

        match failure {
            World::UnknownGuild(name) => AdminFailure::UnknownGuild(name),
            World::InvalidAddress => AdminFailure::InvalidPublicAddress,
            World::NoCrown => AdminFailure::NoCrown,
            World::AlreadyActive => AdminFailure::CrownShieldActive,
            World::UnknownNetwork(address) => AdminFailure::UnknownNetwork(address),
        }
    }
}
