/// Startup failure and associated file path (`Computer.CheckInicio`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupFailure {
    /// A required boot file is absent.
    MissingKernelFile(&'static str),
    /// A required graphics file is absent.
    MissingGraphicsFile(&'static str),
    /// A required system file or library is absent.
    MissingSystemFile(&'static str),
    /// The file has the wrong type: same-name library in `/lib`, text for `/etc/xorg.conf`, or
    /// binary for other paths.
    InvalidFile(&'static str),
}
