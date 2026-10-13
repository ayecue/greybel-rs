//! What an exploit attempt reports: its log, result, and reward.

use crate::{ComputerId, LanFailure, NodeId};

/// Overflow attack output and granted reward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploitReport {
    pub log: ExploitLog,
    /// Object or status returned by the attack.
    pub outcome: ExploitOutcome,
}

/// Overflow attack output (`MetaLibIntrinsics.overflow`), formatted by the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExploitLog {
    /// No output.
    Nothing,
    /// The library is outside `/lib`.
    OutsideLib,
    /// No weakness matches the address and value.
    MissingWeakness,
    /// The router is unreachable or firewalled; only its firewall-disabling weakness is allowed.
    ConnectionRefusal(LanFailure),
    /// The attack ran. Contains the prerequisite library filename and attack result.
    Attack {
        library: Option<String>,
        result: AttackResult,
    },
    /// A CrownShield protects the target network (`Database.IsCrownShieldProtected`).
    CrownShield,
}

/// Attack execution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttackResult {
    /// A weakness requirement failed.
    Unmet(UnmetRequirement),
    /// All requirements passed. Reports administrator interception forcing guest and the reward
    /// result.
    Reward {
        intercepted: bool,
        reward: AttackReward,
    },
}

/// Unsatisfied weakness requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnmetRequirement {
    /// The target no longer exists.
    TargetGone,
    /// The required library filename is not installed.
    MissingLibrary(String),
    /// The required library version is too old.
    OldLibrary {
        library: String,
        minimum: String,
        current: String,
    },
    /// The target has too few registered users.
    TooFewUsers,
    /// The router has too few forwards to the target.
    TooFewPortForwards,
    /// Too few computers are connected to the gateway.
    TooFewGatewayConnections,
    /// A file-library weakness requires a router.
    RouterOnly,
    /// No target user is active.
    NoActiveUser,
    /// The target root user is inactive.
    NoActiveRoot,
    /// The target has no active guest.
    NoGuest,
    /// No file exists at the weakness path.
    MissingPath(String),
    /// The exploit cannot run on the router subnet.
    OtherSubnet,
}

/// Result of a weakness reward after requirements passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttackReward {
    /// The computer has no non-root user.
    NoUser,
    /// Granted user privileges through a shell or folder.
    Privileges(String),
    /// The supplied weakness folder no longer exists.
    MissingResource(String),
    /// The router attack supplied an invalid LAN address.
    InvalidLanAddress,
    /// No computer exists at the LAN address.
    MissingComputer(String),
    /// Granted computer access with user credentials.
    Credentials(String),
    /// The target is not a smart appliance.
    NotAppliance,
    /// Appliance override settings were already enabled.
    OverrideAlreadyOn,
    /// Appliance override settings are now enabled.
    OverrideOn,
    /// The target is not on a police-station network.
    NotPolice,
    /// The traffic-lights override is ready.
    TrafficOverride,
    /// The target is not a router.
    NotRouter,
    /// The router firewall is disabled.
    FirewallOff,
    /// The guest password cannot be changed.
    GuestPassword,
    /// The new password must be alphanumeric.
    InvalidPassword(String),
    /// The new password exceeds 15 characters.
    LongPassword,
    /// The user password was changed.
    PasswordChange(String),
}

/// Object or status returned by an overflow attack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExploitOutcome {
    /// No object or boolean result.
    Nothing,
    /// The target network is disabled for nonpayment; checked before other outcomes
    /// (`MetaLibIntrinsics.overflow`).
    DisabledNetwork,
    /// Successful attack with no object.
    Success,
    /// Failed attack returning false.
    Failure,
    /// Shell granted on a computer as `user`.
    Shell {
        /// Computer accessed by the shell.
        computer: ComputerId,
        /// User privileges available to the shell.
        user: String,
    },
    /// Computer access granted as `user`.
    Computer {
        /// Computer accessed with obtained credentials.
        computer: ComputerId,
        /// User associated with the obtained credentials.
        user: String,
    },
    /// File or folder access on a computer as `user`.
    File {
        /// Computer that owns the node.
        computer: ComputerId,
        /// File or folder returned by the exploit.
        node: NodeId,
        /// Path of the returned node.
        path: String,
        /// User privileges used to access the node.
        user: String,
    },
}

/// Exploit report containing `log` and `outcome`.
pub(super) const fn report(log: ExploitLog, outcome: ExploitOutcome) -> ExploitReport {
    ExploitReport { log, outcome }
}

/// Attack report with prerequisite `library` and `result`.
pub(super) fn attack(
    library: Option<String>,
    result: AttackResult,
    outcome: ExploitOutcome,
) -> ExploitReport {
    report(ExploitLog::Attack { library, result }, outcome)
}

/// Attack report for satisfied requirements, administrator `intercepted` state, and `reward`
/// result.
pub(super) fn reward_report(
    library: Option<String>,
    intercepted: bool,
    reward: AttackReward,
    outcome: ExploitOutcome,
) -> ExploitReport {
    attack(
        library,
        AttackResult::Reward {
            intercepted,
            reward,
        },
        outcome,
    )
}
