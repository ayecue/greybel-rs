//! Required startup files (`Computer.startupFiles`, `Computer.CheckInicio`). A computer with a missing or
//! corrupt required file cannot start, accept connections, or use its network.

/// Startup-file role determining the `Computer.CheckInicio` failure message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupRole {
    /// Boot file; failure reports "Kernel panic".
    Kernel,
    /// Graphics file; failure reports "Graphics error".
    Graphics,
    /// System file or library; failure reports "Fatal error".
    System,
}

/// Computer startup files in `Computer.startupFiles` check order.
pub const STARTUP_FILES: [(&str, StartupRole); 10] = [
    ("/boot/System.map", StartupRole::Kernel),
    ("/boot/initrd.img", StartupRole::Kernel),
    ("/boot/kernel.img", StartupRole::Kernel),
    ("/sys/xorg.sys", StartupRole::Graphics),
    ("/etc/xorg.conf", StartupRole::Graphics),
    ("/sys/config.sys", StartupRole::System),
    ("/sys/network.cfg", StartupRole::System),
    ("/lib/net.so", StartupRole::System),
    ("/lib/init.so", StartupRole::System),
    ("/lib/kernel_module.so", StartupRole::System),
];

/// Router and switch startup files: computer files with `kernel_router.so` replacing `kernel_module.so`,
/// followed by `libhttp.so` (`Computer.CheckInicio`).
pub const ROUTER_STARTUP_FILES: [(&str, StartupRole); 11] = [
    STARTUP_FILES[0],
    STARTUP_FILES[1],
    STARTUP_FILES[2],
    STARTUP_FILES[3],
    STARTUP_FILES[4],
    STARTUP_FILES[5],
    STARTUP_FILES[6],
    STARTUP_FILES[7],
    STARTUP_FILES[8],
    ("/lib/kernel_router.so", StartupRole::System),
    ("/lib/libhttp.so", StartupRole::System),
];

/// Required content type for startup `path` (`Computer.CheckInicio`): libraries under `/lib`, text at
/// `/etc/xorg.conf`, and binaries elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupContent {
    Library,
    Text,
    Binary,
}

impl StartupContent {
    /// Returns the required startup content type for `path`.
    #[must_use]
    pub fn of(path: &str) -> Self {
        if path.contains("/lib/") {
            Self::Library
        } else if path == "/etc/xorg.conf" {
            Self::Text
        } else {
            Self::Binary
        }
    }
}
