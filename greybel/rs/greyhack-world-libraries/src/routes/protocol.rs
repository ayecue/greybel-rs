//! Conversions of exploit, debug, and hack shop results.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
pub(crate) use greyhack_world_router::protocol::*;

impl IntoProtocol for crate::ExploitLog {
    type Output = ExploitLog;

    fn into_protocol(self) -> Self::Output {
        let log = self;

        use crate::ExploitLog as World;

        match log {
            World::Nothing => ExploitLog::Nothing,
            World::OutsideLib => ExploitLog::OutsideLib,
            World::MissingWeakness => ExploitLog::MissingWeakness,
            World::ConnectionRefusal(failure) => {
                ExploitLog::ConnectionRefusal(failure.into_protocol())
            }
            World::CrownShield => ExploitLog::CrownShield,
            World::Attack { library, result } => ExploitLog::Attack {
                library,
                result: match result {
                    crate::AttackResult::Unmet(unmet) => AttackResult::Unmet(unmet.into_protocol()),
                    crate::AttackResult::Reward {
                        intercepted,
                        reward,
                    } => AttackResult::Reward {
                        intercepted,
                        reward: reward.into_protocol(),
                    },
                },
            },
        }
    }
}

impl IntoProtocol for crate::UnmetRequirement {
    type Output = UnmetRequirement;

    fn into_protocol(self) -> Self::Output {
        let unmet = self;

        use crate::UnmetRequirement as World;

        match unmet {
            World::TargetMissing => UnmetRequirement::TargetMissing,
            World::MissingLibrary(library) => UnmetRequirement::MissingLibrary(library),
            World::OldLibrary {
                library,
                minimum,
                current,
            } => UnmetRequirement::OldLibrary {
                library,
                required: minimum,
                installed: current,
            },
            World::TooFewUsers => UnmetRequirement::TooFewUsers,
            World::TooFewPortForwards => UnmetRequirement::TooFewPortForwards,
            World::TooFewGatewayConnections => UnmetRequirement::TooFewGatewayConnections,
            World::RouterOnly => UnmetRequirement::RouterOnly,
            World::NoActiveUser => UnmetRequirement::NoActiveUser,
            World::NoActiveRoot => UnmetRequirement::NoActiveRoot,
            World::NoGuest => UnmetRequirement::NoGuest,
            World::MissingPath(path) => UnmetRequirement::MissingPath(path),
            World::OtherSubnet => UnmetRequirement::OtherSubnet,
        }
    }
}

impl IntoProtocol for crate::AttackReward {
    type Output = AttackReward;

    fn into_protocol(self) -> Self::Output {
        let reward = self;

        use crate::AttackReward as World;

        match reward {
            World::NoUser => AttackReward::NoUser,
            World::Privileges(user) => AttackReward::Privileges(user),
            World::MissingResource(folder) => AttackReward::MissingResource(folder),
            World::InvalidLanAddress => AttackReward::InvalidLanAddress,
            World::MissingComputer(address) => AttackReward::MissingComputer(address),
            World::Credentials(user) => AttackReward::Credentials(user),
            World::NotAppliance => AttackReward::NotAppliance,
            World::OverrideAlreadyOn => AttackReward::OverrideAlreadyOn,
            World::OverrideOn => AttackReward::OverrideOn,
            World::NotPolice => AttackReward::NotPolice,
            World::TrafficOverride => AttackReward::TrafficOverride,
            World::NotRouter => AttackReward::NotRouter,
            World::FirewallOff => AttackReward::FirewallOff,
            World::GuestPassword => AttackReward::GuestPassword,
            World::InvalidPassword(user) => AttackReward::InvalidPassword(user),
            World::LongPassword => AttackReward::LongPassword,
            World::PasswordChange(user) => AttackReward::PasswordChange(user),
        }
    }
}

impl IntoProtocol for crate::DebugFailure {
    type Output = DebugFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::DebugFailure as World;

        match failure {
            World::IncorrectPassword => DebugFailure::IncorrectPassword,
            World::LoginLockout => DebugFailure::LoginBlocked,
            World::MissingFile => DebugFailure::MissingFile,
            World::OutsideLib => DebugFailure::OutsideLib,
            World::PatchPresent => DebugFailure::Patched,
            World::NoVulnerability => DebugFailure::NoVulnerability,
            World::UnreachableResource(path) => DebugFailure::UnreachableResource(path),
            World::ScanPatchPresent => DebugFailure::ScanPatchPresent,
            World::TestPatchPresent => DebugFailure::TestPatchPresent,
            World::InvalidPatchFile => DebugFailure::InvalidPatchFile,
            World::IncorrectPatch => DebugFailure::IncorrectPatch,
            World::PatchAlreadyPresent => DebugFailure::PatchAlreadyPresent,
            World::UnitTestLockout => DebugFailure::UnitTestLockout,
            World::IncorrectErrorLines => DebugFailure::IncorrectErrorLines,
        }
    }
}

impl IntoProtocol for crate::ExploitShopFailure {
    type Output = ShopFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::ExploitShopFailure as World;

        match failure {
            World::InvalidVersion => ShopFailure::InvalidVersion,
            World::NoResults => ShopFailure::NoResults,
            World::InvalidPath => ShopFailure::InvalidPath,
            World::UnknownComputer => ShopFailure::UnknownComputer,
            World::NoInternet => ShopFailure::NoInternet,
            World::MissingFolder(folder) => ShopFailure::MissingFolder(folder),
            World::NoCreatePermission { folder, name } => {
                ShopFailure::NoCreatePermission { folder, name }
            }
            World::UnknownExploit => ShopFailure::UnknownExploit,
            World::NoBankAccount => ShopFailure::NoBankAccount,
            World::UnknownBankAccount(number) => ShopFailure::UnknownBankAccount(number),
            World::InsufficientFunds => ShopFailure::InsufficientFunds,
            World::File(failure) => ShopFailure::File(failure.into_protocol()),
        }
    }
}

impl IntoProtocol for &crate::Weakness {
    type Output = WeaknessInfo;

    fn into_protocol(self) -> Self::Output {
        let weakness = self;

        use greyhack_behaviour::Requirement as Rule;

        WeaknessInfo {
            details: weakness.details.clone(),
            unsec_value: weakness.unsec_value.clone(),
            requirements: weakness
                .requirements
                .iter()
                .map(|requirement| match requirement {
                    Rule::Library => WeaknessRequirement::Library {
                        name: weakness
                            .required_lib
                            .map(greyhack_behaviour::LibraryKind::file_name)
                            .unwrap_or_default(),
                        version: weakness.req_lib_version.clone(),
                    },
                    Rule::NumberUsersRegister => {
                        WeaknessRequirement::RegisteredUsers(weakness.num_register_users)
                    }
                    Rule::AnyActiveUser => WeaknessRequirement::AnyActiveUser,
                    Rule::RootActiveUser => WeaknessRequirement::RootActiveUser,
                    Rule::GuestActiveUser => WeaknessRequirement::GuestActiveUser,
                    Rule::PortForward => {
                        WeaknessRequirement::PortForward(weakness.num_port_forward)
                    }
                    Rule::ConnGateway => {
                        WeaknessRequirement::ConnGateway(weakness.num_conn_gateway)
                    }
                    Rule::LocalNetwork => WeaknessRequirement::LocalNetwork,
                    Rule::PathExist => WeaknessRequirement::PathExist(weakness.path_exist.clone()),
                })
                .collect(),
        }
    }
}

impl IntoProtocol for crate::DebugChallenge {
    type Output = DebugScan;

    fn into_protocol(self) -> Self::Output {
        let challenge = self;

        DebugScan {
            description: challenge.description,
            additional_descriptions: challenge.additional_descriptions,
            code: challenge.code,
        }
    }
}

impl IntoProtocol for crate::ZeroDayObjectKind {
    type Output = ZeroDayObjectKind;

    fn into_protocol(self) -> Self::Output {
        let kind = self;

        match kind {
            crate::ZeroDayObjectKind::Computer => ZeroDayObjectKind::Computer,
            crate::ZeroDayObjectKind::File => ZeroDayObjectKind::File,
        }
    }
}

impl IntoProtocol for crate::ZeroDay {
    type Output = UnitTestReport;

    fn into_protocol(self) -> Self::Output {
        let zero_day = self;

        UnitTestReport {
            address: zero_day.address,
            objects: zero_day
                .objects
                .into_iter()
                .map(|object| ZeroDayObjectInfo {
                    kind: object.kind.into_protocol(),
                    access: object.access,
                    methods: object.methods,
                })
                .collect(),
            vulnerability: (&zero_day.vulnerability).into_protocol(),
        }
    }
}

impl IntoProtocol for crate::NetUseFailure {
    type Output = NetUseFailure;

    fn into_protocol(self) -> Self::Output {
        use crate::NetUseFailure as World;

        match self {
            World::InvalidAddress => NetUseFailure::InvalidAddress,
            World::UnknownAddress => NetUseFailure::UnknownAddress,
            World::MainRouterDown => NetUseFailure::MainRouterDown,
            World::Lan(failure) => NetUseFailure::Lan(failure.into_protocol()),
            World::UnknownComputer => NetUseFailure::UnknownComputer,
            World::MissingPort => NetUseFailure::MissingPort,
            World::ClosedPort => NetUseFailure::ClosedPort,
            World::InvalidLibrary(service) => NetUseFailure::InvalidLibrary(service.library()),
            World::UnexpectedLibrary(name) => NetUseFailure::UnexpectedLibrary(name),
        }
    }
}

pub(crate) fn rshell_failure(
    failure: crate::RshellFailure,
    address: &str,
    port: u16,
) -> RshellFailure {
    use crate::RshellFailure as World;

    match failure {
        World::UnknownAddress => RshellFailure::UnknownAddress(address.to_owned()),
        World::MissingServer => RshellFailure::MissingServer(port),
        World::Service(failure) => RshellFailure::Service(failure.into_protocol()),
    }
}
