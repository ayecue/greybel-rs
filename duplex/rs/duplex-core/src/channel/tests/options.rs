#[path = "support/options.rs"]
mod support;

use std::time::Duration;

use support::FLAVOURS;

use super::{
    ChannelOptions, ChannelSettings, DEFAULT_MAX_MESSAGE, DEFAULT_STREAM_WINDOW, HealthCheck,
    HealthChecks, TransportFlavour,
};

#[test]
fn should_check_health_as_the_flavour_of_the_transport_says() {
    let checks = ["stdio", "port", "worker", "memory"].map(|transport| {
        ChannelSettings::of(
            &ChannelOptions::default(),
            FLAVOURS,
            transport,
            DEFAULT_STREAM_WINDOW,
        )
        .health_check
    });

    assert_eq!(
        checks,
        [
            Some(HealthCheck {
                interval: Duration::from_secs(1),
                ..HealthCheck::DEFAULT
            }),
            None,
            Some(HealthCheck::DEFAULT),
            Some(HealthCheck::DEFAULT),
        ]
    );
}

#[test]
fn should_check_health_as_the_options_say_before_the_flavour() {
    let every_minute = HealthCheck {
        interval: Duration::from_secs(60),
        ..HealthCheck::DEFAULT
    };
    let options = ChannelOptions {
        health_check: Some(HealthChecks::On(every_minute)),
        ..ChannelOptions::default()
    };
    let checks = [options, ChannelOptions::without_health_checks()].map(|options| {
        ChannelSettings::of(&options, FLAVOURS, "port", DEFAULT_STREAM_WINDOW).health_check
    });

    assert_eq!(checks, [Some(every_minute), None]);
}

#[test]
fn should_take_the_window_of_the_options_then_the_flavour_then_the_driver() {
    let options = ChannelOptions {
        stream_window: Some(8),
        ..ChannelOptions::default()
    };
    let windows = [
        ChannelSettings::of(&options, FLAVOURS, "worker", 1024).window,
        ChannelSettings::of(&ChannelOptions::default(), FLAVOURS, "worker", 1024).window,
        ChannelSettings::of(&ChannelOptions::default(), FLAVOURS, "port", 1024).window,
    ];

    assert_eq!(windows, [8, 64, 1024]);
}

#[test]
fn should_take_the_timeout_of_the_flavour() {
    let timeouts = ["worker", "port"].map(|transport| {
        ChannelSettings::of(&ChannelOptions::default(), FLAVOURS, transport, 1024).timeout
    });

    assert_eq!(timeouts, [Some(Duration::from_secs(2)), None]);
}

#[test]
fn should_take_the_most_a_message_takes_of_the_options_then_the_flavour_then_the_default() {
    let options = ChannelOptions {
        max_message: Some(16),
        ..ChannelOptions::default()
    };
    let most = [
        ChannelSettings::of(&options, FLAVOURS, "worker", 1024).max_message,
        ChannelSettings::of(&ChannelOptions::default(), FLAVOURS, "worker", 1024).max_message,
        ChannelSettings::of(&ChannelOptions::default(), FLAVOURS, "port", 1024).max_message,
    ];

    assert_eq!(most, [16, 1024, DEFAULT_MAX_MESSAGE as usize]);
}
