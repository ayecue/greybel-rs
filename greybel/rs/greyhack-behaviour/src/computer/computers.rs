//! Device types and process limits.

use crate::base::refusal::{Refusal, RuleResult};
use crate::computer::ssh::SSH_SCRIPT_PARAM;
use crate::path;

enum_def! {
    /// Device type (`NetworkLan.TypeDevice`).
    pub enum DeviceKind {
        Computer = 0,
        Router = 1,
        Switch = 2,
        Hub = 3,
        Camera = 4,
        Microwave = 5,
        Fridge = 6,
        Smartphone = 7,
    }
}

/// Root processes started at boot, with PID, name, and RAM use in megabytes.
pub const SYSTEM_PROCESSES: [(u32, &str, f32); 2] = [(1, "kernel_task", 64.0), (2, "Xorg", 128.0)];

/// Checks whether `name` is in [`SYSTEM_PROCESSES`] and therefore excluded from user process counts.
#[must_use]
pub fn is_system_process(name: &str) -> bool {
    SYSTEM_PROCESSES
        .iter()
        .any(|(_, system, _)| *system == name)
}

/// Path of the clock program every smartphone has (`NetworkLan.GenerateSmartPhone`).
pub const CLOCK_PROGRAM_PATH: &str = "/usr/bin/Clock.exe";
/// Name of the clock program (`NetworkLan.GenerateSmartPhone`).
pub const CLOCK_PROGRAM_NAME: &str = "Clock.exe";

/// Desktop-session process shown by `ps` for a logged-in user.
pub const SESSION_PROCESS: &str = "dsession";

/// Returns the `dsession` PID for `surname`: `new Random(surname.GetHashCode()).Next(12000, 20000)`
/// (`Persona.GetDSessionPID`).
#[must_use]
pub fn session_pid(surname: &str) -> u32 {
    let mut random = mono_random::Random::new(mono_string::hash_code(surname));

    12_000 + u32::try_from(random.next_below(8_000)).expect("below 8000")
}

/// Minutes a computer stays down after a reboot (`ConfigOS.Reboot`).
pub const REBOOT_MINUTES: u64 = 3;
/// Minutes a computer stays down after a reboot that follows floods (`ConfigOS.Reboot`).
pub const FLOOD_REBOOT_MINUTES: u64 = 20;

/// World seconds a computer stays down after a reboot, longer when `flooding` (`ConfigOS.Reboot`).
#[must_use]
pub const fn reboot_seconds(flooding: bool) -> u64 {
    let minutes = if flooding {
        FLOOD_REBOOT_MINUTES
    } else {
        REBOOT_MINUTES
    };

    minutes * crate::SECONDS_PER_MINUTE
}

/// Maximum number of user accounts per computer.
pub const MAX_USERS: usize = 16;
/// Initial PID for spawned user processes.
pub const FIRST_SPAWN_PID: u32 = 1000;
/// RAM allocated to each launched program.
pub const PROCESS_RAM_MB: f32 = 25.0;
/// Name of the protected terminal process a restart in safe mode starts
/// (`PlayerHelperServer.CheckIniErrorServerRpc`).
pub const SAFE_MODE_TERMINAL: &str = "Terminal-safemode";
/// RAM of the safe-mode terminal process.
pub const SAFE_MODE_TERMINAL_RAM_MB: f32 = 0.1;
/// Maximum number of concurrent terminal processes per computer.
pub const MAX_TERMINALS: usize = 15;
/// Grey Hack limit for nested program launches.
pub const MAX_LAUNCH_DEPTH: u32 = 32;
/// Start of the PID range reserved for protected game processes.
pub const PROTECTED_PIDS: i32 = 12_000;
/// Maximum process-name length accepted by `rshell_client`.
pub const MAX_PROCESS_NAME: usize = 24;
/// Process names rejected by `rshell_client`. Comparison uses the lowercased name, so the non-lowercase
/// `Xorg` entry is allowed, matching the game (`MetaExploitIntrinsics` `rshell_client`).
pub const RESERVED_PROCESS_NAMES: &[&str] =
    &[SESSION_PROCESS, "kernel_task", "Xorg", SSH_SCRIPT_PARAM];

/// Validates an `rshell_client` process name. The game reports a 28-character limit but enforces 24.
pub fn check_process_name(name: &str) -> RuleResult {
    if !path::is_valid_name(name) {
        Err(Refusal::InvalidProcessName)
    } else if name.chars().count() > MAX_PROCESS_NAME {
        Err(Refusal::ProcessNameTooLong)
    } else if RESERVED_PROCESS_NAMES.contains(&name.to_lowercase().trim()) {
        Err(Refusal::ReservedProcessName(name.to_owned()))
    } else {
        Ok(())
    }
}
