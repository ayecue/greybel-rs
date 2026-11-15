//! Conversions of exploit, debug, and hack shop results.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
pub(crate) use greyhack_world_programs::protocol::*;

impl IntoProtocol for crate::ExploitLog {
    type Output = ExploitLog;

    fn into_protocol(self) -> Self::Output {
        let log = self;

        match log {
            crate::ExploitLog::Nothing => ExploitLog::Nothing,
            crate::ExploitLog::OutsideLib => ExploitLog::OutsideLib,
            crate::ExploitLog::MissingWeakness => ExploitLog::MissingWeakness,
            crate::ExploitLog::ConnectionRefusal(failure) => {
                ExploitLog::ConnectionRefusal(failure.into_protocol())
            }
            crate::ExploitLog::CrownShield => ExploitLog::CrownShield,
            crate::ExploitLog::Attack { library, result } => ExploitLog::Attack {
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

        match unmet {
            crate::UnmetRequirement::TargetMissing => UnmetRequirement::TargetMissing,
            crate::UnmetRequirement::MissingLibrary(library) => {
                UnmetRequirement::MissingLibrary(library)
            }
            crate::UnmetRequirement::OldLibrary {
                library,
                minimum,
                current,
            } => UnmetRequirement::OldLibrary {
                library,
                required: minimum,
                installed: current,
            },
            crate::UnmetRequirement::TooFewUsers => UnmetRequirement::TooFewUsers,
            crate::UnmetRequirement::TooFewPortForwards => UnmetRequirement::TooFewPortForwards,
            crate::UnmetRequirement::TooFewGatewayConnections => {
                UnmetRequirement::TooFewGatewayConnections
            }
            crate::UnmetRequirement::RouterOnly => UnmetRequirement::RouterOnly,
            crate::UnmetRequirement::NoActiveUser => UnmetRequirement::NoActiveUser,
            crate::UnmetRequirement::NoActiveRoot => UnmetRequirement::NoActiveRoot,
            crate::UnmetRequirement::NoGuest => UnmetRequirement::NoGuest,
            crate::UnmetRequirement::MissingPath(path) => UnmetRequirement::MissingPath(path),
            crate::UnmetRequirement::OtherSubnet => UnmetRequirement::OtherSubnet,
        }
    }
}

impl IntoProtocol for crate::AttackReward {
    type Output = AttackReward;

    fn into_protocol(self) -> Self::Output {
        let reward = self;

        match reward {
            crate::AttackReward::NoUser => AttackReward::NoUser,
            crate::AttackReward::Privileges(user) => AttackReward::Privileges(user),
            crate::AttackReward::MissingResource(folder) => AttackReward::MissingResource(folder),
            crate::AttackReward::InvalidLanAddress => AttackReward::InvalidLanAddress,
            crate::AttackReward::MissingComputer(address) => AttackReward::MissingComputer(address),
            crate::AttackReward::Credentials(user) => AttackReward::Credentials(user),
            crate::AttackReward::NotAppliance => AttackReward::NotAppliance,
            crate::AttackReward::OverrideAlreadyOn => AttackReward::OverrideAlreadyOn,
            crate::AttackReward::OverrideOn => AttackReward::OverrideOn,
            crate::AttackReward::NotPolice => AttackReward::NotPolice,
            crate::AttackReward::TrafficOverride => AttackReward::TrafficOverride,
            crate::AttackReward::NotRouter => AttackReward::NotRouter,
            crate::AttackReward::FirewallOff => AttackReward::FirewallOff,
            crate::AttackReward::GuestPassword => AttackReward::GuestPassword,
            crate::AttackReward::InvalidPassword(user) => AttackReward::InvalidPassword(user),
            crate::AttackReward::LongPassword => AttackReward::LongPassword,
            crate::AttackReward::PasswordChange(user) => AttackReward::PasswordChange(user),
        }
    }
}

impl IntoProtocol for crate::DebugFailure {
    type Output = DebugFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::DebugFailure::IncorrectPassword => DebugFailure::IncorrectPassword,
            crate::DebugFailure::LoginLockout => DebugFailure::LoginBlocked,
            crate::DebugFailure::MissingFile => DebugFailure::MissingFile,
            crate::DebugFailure::OutsideLib => DebugFailure::OutsideLib,
            crate::DebugFailure::PatchPresent => DebugFailure::Patched,
            crate::DebugFailure::NoVulnerability => DebugFailure::NoVulnerability,
            crate::DebugFailure::UnreachableResource(path) => {
                DebugFailure::UnreachableResource(path)
            }
            crate::DebugFailure::ScanPatchPresent => DebugFailure::ScanPatchPresent,
            crate::DebugFailure::TestPatchPresent => DebugFailure::TestPatchPresent,
            crate::DebugFailure::InvalidPatchFile => DebugFailure::InvalidPatchFile,
            crate::DebugFailure::IncorrectPatch => DebugFailure::IncorrectPatch,
            crate::DebugFailure::PatchAlreadyPresent => DebugFailure::PatchAlreadyPresent,
            crate::DebugFailure::UnitTestLockout => DebugFailure::UnitTestLockout,
            crate::DebugFailure::IncorrectErrorLines => DebugFailure::IncorrectErrorLines,
        }
    }
}

impl IntoProtocol for crate::ExploitShopFailure {
    type Output = ShopFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::ExploitShopFailure::InvalidVersion => ShopFailure::InvalidVersion,
            crate::ExploitShopFailure::NoResults => ShopFailure::NoResults,
            crate::ExploitShopFailure::InvalidPath => ShopFailure::InvalidPath,
            crate::ExploitShopFailure::UnknownComputer => ShopFailure::UnknownComputer,
            crate::ExploitShopFailure::NoInternet => ShopFailure::NoInternet,
            crate::ExploitShopFailure::MissingFolder(folder) => ShopFailure::MissingFolder(folder),
            crate::ExploitShopFailure::NoCreatePermission { folder, name } => {
                ShopFailure::NoCreatePermission { folder, name }
            }
            crate::ExploitShopFailure::UnknownExploit => ShopFailure::UnknownExploit,
            crate::ExploitShopFailure::NoBankAccount => ShopFailure::NoBankAccount,
            crate::ExploitShopFailure::UnknownBankAccount(number) => {
                ShopFailure::UnknownBankAccount(number)
            }
            crate::ExploitShopFailure::InsufficientFunds => ShopFailure::InsufficientFunds,
            crate::ExploitShopFailure::File(failure) => ShopFailure::File(failure.into_protocol()),
        }
    }
}

impl IntoProtocol for &crate::Weakness {
    type Output = WeaknessInfo;

    fn into_protocol(self) -> Self::Output {
        let weakness = self;

        WeaknessInfo {
            details: weakness.details.clone(),
            unsec_value: weakness.unsec_value.clone(),
            requirements: weakness
                .requirements
                .iter()
                .map(|requirement| match requirement {
                    greyhack_behaviour::Requirement::Library => WeaknessRequirement::Library {
                        name: weakness
                            .required_lib
                            .map(greyhack_behaviour::LibraryKind::file_name)
                            .unwrap_or_default(),
                        version: weakness.req_lib_version.clone(),
                    },
                    greyhack_behaviour::Requirement::NumberUsersRegister => {
                        WeaknessRequirement::RegisteredUsers(weakness.num_register_users)
                    }
                    greyhack_behaviour::Requirement::AnyActiveUser => {
                        WeaknessRequirement::AnyActiveUser
                    }
                    greyhack_behaviour::Requirement::RootActiveUser => {
                        WeaknessRequirement::RootActiveUser
                    }
                    greyhack_behaviour::Requirement::GuestActiveUser => {
                        WeaknessRequirement::GuestActiveUser
                    }
                    greyhack_behaviour::Requirement::PortForward => {
                        WeaknessRequirement::PortForward(weakness.num_port_forward)
                    }
                    greyhack_behaviour::Requirement::ConnGateway => {
                        WeaknessRequirement::ConnGateway(weakness.num_conn_gateway)
                    }
                    greyhack_behaviour::Requirement::LocalNetwork => {
                        WeaknessRequirement::LocalNetwork
                    }
                    greyhack_behaviour::Requirement::PathExist => {
                        WeaknessRequirement::PathExist(weakness.path_exist.clone())
                    }
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
        match self {
            crate::NetUseFailure::InvalidAddress => NetUseFailure::InvalidAddress,
            crate::NetUseFailure::UnknownAddress => NetUseFailure::UnknownAddress,
            crate::NetUseFailure::MainRouterDown => NetUseFailure::MainRouterDown,
            crate::NetUseFailure::Lan(failure) => NetUseFailure::Lan(failure.into_protocol()),
            crate::NetUseFailure::UnknownComputer => NetUseFailure::UnknownComputer,
            crate::NetUseFailure::MissingPort => NetUseFailure::MissingPort,
            crate::NetUseFailure::ClosedPort => NetUseFailure::ClosedPort,
            crate::NetUseFailure::InvalidLibrary(service) => {
                NetUseFailure::InvalidLibrary(service.library())
            }
            crate::NetUseFailure::UnexpectedLibrary(name) => NetUseFailure::UnexpectedLibrary(name),
        }
    }
}

impl IntoProtocol for crate::RshellRemoveFailure {
    type Output = RshellRemoveFailure;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::NoService => RshellRemoveFailure::NoService,
            Self::ForwardMissing => RshellRemoveFailure::ForwardMissing,
            Self::CtfNetwork => RshellRemoveFailure::CtfNetwork,
            Self::ExpectedList => RshellRemoveFailure::ExpectedList,
            Self::OnlyReverseShells => RshellRemoveFailure::OnlyReverseShells,
        }
    }
}

pub(crate) fn rshell_failure(
    failure: crate::RshellFailure,
    address: &str,
    port: u16,
) -> RshellFailure {
    match failure {
        crate::RshellFailure::UnknownAddress => RshellFailure::UnknownAddress(address.to_owned()),
        crate::RshellFailure::MissingServer => RshellFailure::MissingServer(port),
        crate::RshellFailure::Service(failure) => RshellFailure::Service(failure.into_protocol()),
        crate::RshellFailure::Limit => RshellFailure::Limit,
    }
}
