//! Conversions of mail, alarm, and job failures.

use super::{AlarmFailure, JobFailure, SmtpFailure};

impl From<greyhack_world::SmtpFailure> for SmtpFailure {
    fn from(failure: greyhack_world::SmtpFailure) -> Self {
        use greyhack_world::SmtpFailure as World;

        match failure {
            World::InvalidAddress => Self::InvalidAddress,
            World::HostDown => Self::HostDown,
            World::UnknownHost => Self::UnknownHost,
            World::UnknownAddress => Self::UnknownAddress,
            World::MissingPort(port) => Self::MissingPort(port),
            World::InvalidService => Self::InvalidService,
            World::MissingService => Self::MissingService,
        }
    }
}

impl From<greyhack_world::AlarmFailure> for AlarmFailure {
    fn from(failure: greyhack_world::AlarmFailure) -> Self {
        match failure {
            greyhack_world::AlarmFailure::IncorrectTime => Self::IncorrectTime,
            greyhack_world::AlarmFailure::IncompatibleDevice => Self::IncompatibleDevice,
        }
    }
}

impl From<greyhack_world::FireFailure> for JobFailure {
    fn from(failure: greyhack_world::FireFailure) -> Self {
        use greyhack_world::FireFailure as World;

        match failure {
            World::UnknownPerson => Self::UnknownPerson,
            World::Unavailable => Self::Unavailable,
            World::HiredRecently => Self::HiredRecently,
        }
    }
}

impl From<greyhack_world::HireFailure> for JobFailure {
    fn from(failure: greyhack_world::HireFailure) -> Self {
        use greyhack_world::HireFailure as World;

        match failure {
            World::InvalidJob => Self::InvalidJob,
            World::NoVacancy(role) => Self::NoVacancy(role.game_name().to_owned()),
        }
    }
}

impl From<greyhack_world::RentalAdminFailure> for JobFailure {
    fn from(failure: greyhack_world::RentalAdminFailure) -> Self {
        use greyhack_world::RentalAdminFailure as World;

        match failure {
            World::AdminExists => Self::AdminExists,
            World::NoAdmin => Self::NoAdmin,
            World::Purchase(failure) => match failure {
                greyhack_world::PurchaseFailure::NoBankAccount => Self::NoBankAccount,
                greyhack_world::PurchaseFailure::UnknownBankAccount(number) => {
                    Self::UnknownBankAccount(number)
                }
                greyhack_world::PurchaseFailure::InsufficientFunds => Self::InsufficientFunds,
            },
            World::NoAdminPayment => Self::NoAdminPayment,
        }
    }
}
