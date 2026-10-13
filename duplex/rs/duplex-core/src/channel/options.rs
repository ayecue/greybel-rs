use std::time::Duration;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// Channel options. `Default` selects `Bundle::Common`, `Limits::DEFAULT`, and default health checks.
pub struct ChannelOptions {
    /// Overrides transport flavor health checks.
    pub health_check: Option<HealthChecks>,
    /// Controls outgoing batch sharing between channels.
    pub bundle: Bundle,
    /// Sets concurrent message limits by category.
    pub limits: Limits,
    /// Overrides the transport flavor or driver stream window.
    pub stream_window: Option<u32>,
    /// Overrides maximum message frame bytes. Larger sends return `Overload`.
    pub max_message: Option<u32>,
    /// Disables message validation on the client.
    pub unvalidated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Message limits by category. Exceeding a limit returns `Overload`.
pub struct Limits {
    /// Maximum number queued while no connection is available.
    pub queued_items: usize,
    /// Maximum number sent and awaiting a response.
    pub pending_items: usize,
    /// Maximum number received and actively handled.
    pub active_responses: usize,
    /// Maximum number received and waiting for a handler.
    pub incoming_items: usize,
}

impl Limits {
    /// Defaults to 2048 queued, 16384 pending, 8192 active responses, and 16384 incoming messages.
    pub const DEFAULT: Self = Self {
        queued_items: 2048,
        pending_items: 16_384,
        active_responses: 8192,
        incoming_items: 16_384,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
/// Transport message batch-sharing mode.
pub enum Bundle {
    #[default]
    /// Shares one batch per transport and task with all channels using `Common`.
    Common,
    /// Shares batches only among messages with the same key and transport.
    Key(u32),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// Per-message send options.
pub struct SendOptions {
    /// Overrides the channel bundle for this message.
    pub bundle: Option<Bundle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Channel health check options.
pub enum HealthChecks {
    /// Turns off health checks.
    Off,
    /// Enables health checks using the supplied timings.
    On(HealthCheck),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Protocol defaults for a transport.
pub struct TransportFlavour {
    /// Transport name.
    pub transport_name: &'static str,
    /// Transport health check options.
    pub health_check: Option<HealthChecks>,
    /// Transport stream window.
    pub stream_window: Option<u32>,
    /// Default timeout for messages without a declared timeout.
    pub timeout: Option<Duration>,
    /// Transport maximum message frame bytes.
    pub max_message: Option<u32>,
}

/// Default stream window in items. The 1 MiB buffer budget can reduce the effective window.
pub const DEFAULT_STREAM_WINDOW: u32 = 4096;

/// Uses 64 MiB when neither options nor transport flavor specify a maximum message frame size.
pub const DEFAULT_MAX_MESSAGE: u32 = 64 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Connection health check timings and thresholds.
pub struct HealthCheck {
    /// Health check interval.
    pub interval: Duration,
    /// Marks the connection unhealthy after more than this number of consecutive missed checks.
    pub unhealthy_threshold: usize,
    /// Closes the channel with `Timeout` after more than this number of consecutive missed checks.
    pub close_threshold: usize,
}

impl ChannelOptions {
    #[must_use]
    /// Returns defaults with health checks disabled.
    pub fn without_health_checks() -> Self {
        Self {
            health_check: Some(HealthChecks::Off),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChannelSettings {
    pub(crate) health_check: Option<HealthCheck>,
    pub(crate) window: u32,
    pub(crate) bundle: Bundle,
    pub(crate) limits: Limits,
    pub(crate) timeout: Option<Duration>,
    pub(crate) max_message: usize,
    pub(crate) validating: bool,
}

impl ChannelSettings {
    pub(crate) fn of(
        options: &ChannelOptions,
        flavours: &[TransportFlavour],
        transport: &str,
        window: u32,
    ) -> Self {
        let flavour = flavours
            .iter()
            .find(|flavour| flavour.transport_name == transport);
        let health_check = match options
            .health_check
            .or_else(|| flavour.and_then(|flavour| flavour.health_check))
        {
            Some(HealthChecks::Off) => None,
            Some(HealthChecks::On(health_check)) => Some(health_check),
            None => Some(HealthCheck::DEFAULT),
        };

        Self {
            health_check,
            window: options
                .stream_window
                .or_else(|| flavour.and_then(|flavour| flavour.stream_window))
                .unwrap_or(window),
            bundle: options.bundle,
            limits: options.limits,
            timeout: flavour.and_then(|flavour| flavour.timeout),
            max_message: options
                .max_message
                .or_else(|| flavour.and_then(|flavour| flavour.max_message))
                .unwrap_or(DEFAULT_MAX_MESSAGE) as usize,
            validating: !options.unvalidated,
        }
    }
}

impl HealthCheck {
    /// Uses a 5-second interval, unhealthy threshold 2, and close threshold 3.
    pub const DEFAULT: Self = Self {
        interval: Duration::from_secs(5),
        unhealthy_threshold: 2,
        close_threshold: 3,
    };
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
#[path = "tests/options.rs"]
mod tests;
