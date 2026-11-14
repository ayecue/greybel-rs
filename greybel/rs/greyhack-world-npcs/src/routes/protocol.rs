//! Conversions of mail, alarm, and job failures.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
pub(crate) use greyhack_world_router::protocol::*;

impl IntoProtocol for crate::SmtpFailure {
    type Output = SmtpFailure;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::InvalidAddress => SmtpFailure::InvalidAddress,
            Self::HostDown => SmtpFailure::HostDown,
            Self::UnknownHost => SmtpFailure::UnknownHost,
            Self::UnknownAddress => SmtpFailure::UnknownAddress,
            Self::MissingPort(port) => SmtpFailure::MissingPort(port),
            Self::InvalidService => SmtpFailure::InvalidService,
            Self::MissingService => SmtpFailure::MissingService,
        }
    }
}

impl IntoProtocol for crate::AlarmFailure {
    type Output = AlarmFailure;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::IncorrectTime => AlarmFailure::IncorrectTime,
            Self::IncompatibleDevice => AlarmFailure::IncompatibleDevice,
        }
    }
}

impl IntoProtocol for crate::FireFailure {
    type Output = JobFailure;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::UnknownPerson => JobFailure::UnknownPerson,
            Self::Unavailable => JobFailure::Unavailable,
            Self::HiredRecently => JobFailure::HiredRecently,
        }
    }
}

impl IntoProtocol for crate::HireFailure {
    type Output = JobFailure;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::InvalidJob => JobFailure::InvalidJob,
            Self::NoVacancy(role) => JobFailure::NoVacancy(role.game_name().to_owned()),
        }
    }
}

impl IntoProtocol for crate::RentalAdminFailure {
    type Output = JobFailure;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::AdminExists => JobFailure::AdminExists,
            Self::NoAdmin => JobFailure::NoAdmin,
            Self::Purchase(failure) => match failure {
                greyhack_world_economy::PurchaseFailure::NoBankAccount => JobFailure::NoBankAccount,
                greyhack_world_economy::PurchaseFailure::UnknownBankAccount(number) => {
                    JobFailure::UnknownBankAccount(number)
                }
                greyhack_world_economy::PurchaseFailure::InsufficientFunds => {
                    JobFailure::InsufficientFunds
                }
            },
            Self::NoAdminPayment => JobFailure::NoAdminPayment,
        }
    }
}
