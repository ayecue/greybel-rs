//! Conversions of guild and admin restore failures.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
use greyhack_world_programs::protocol::AdminFailure;

impl IntoProtocol for crate::GuildFailure {
    type Output = AdminFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::GuildFailure::NameInUse(name) => AdminFailure::GuildNameTaken(name),
            crate::GuildFailure::AccountInUse(account) => AdminFailure::GuildAccountTaken(account),
        }
    }
}

impl IntoProtocol for crate::AdminRestoreFailure {
    type Output = AdminFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::AdminRestoreFailure::UnknownGuild(name) => AdminFailure::UnknownGuild(name),
            crate::AdminRestoreFailure::InvalidCompany => AdminFailure::InvalidCompany,
            crate::AdminRestoreFailure::UnknownCompany => AdminFailure::UnknownCompany,
            crate::AdminRestoreFailure::NotGuildCompany => AdminFailure::NotGuildCompany,
            crate::AdminRestoreFailure::OverLimit => AdminFailure::AdminRestoreLimit,
        }
    }
}

impl IntoProtocol for crate::CrownShieldFailure {
    type Output = AdminFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::CrownShieldFailure::UnknownGuild(name) => AdminFailure::UnknownGuild(name),
            crate::CrownShieldFailure::InvalidAddress => AdminFailure::InvalidPublicAddress,
            crate::CrownShieldFailure::NoCrown => AdminFailure::NoCrown,
            crate::CrownShieldFailure::AlreadyActive => AdminFailure::CrownShieldActive,
            crate::CrownShieldFailure::UnknownNetwork(address) => {
                AdminFailure::UnknownNetwork(address)
            }
        }
    }
}
