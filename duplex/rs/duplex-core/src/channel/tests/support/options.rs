use std::time::Duration;

use super::*;

pub const FLAVOURS: &[TransportFlavour] = &[
    TransportFlavour {
        transport_name: "stdio",
        health_check: Some(HealthChecks::On(HealthCheck {
            interval: Duration::from_secs(1),
            ..HealthCheck::DEFAULT
        })),
        stream_window: None,
        timeout: None,
        max_message: None,
    },
    TransportFlavour {
        transport_name: "port",
        health_check: Some(HealthChecks::Off),
        stream_window: None,
        timeout: None,
        max_message: None,
    },
    TransportFlavour {
        transport_name: "worker",
        health_check: None,
        stream_window: Some(64),
        timeout: Some(Duration::from_secs(2)),
        max_message: Some(1024),
    },
];
