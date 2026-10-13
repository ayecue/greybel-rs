use std::time::Duration;

use duplex_core::{Channel, ChannelOptions, Transport};
use duplex_driver_stdio::StreamDriver;
use duplex_test_support::conformance::calculator::CalculatorRouter;
use duplex_test_support::conformance::{Adder, calculator, user};

pub const STREAM_BYTES: usize = 1024;

pub const SETTLE: Duration = Duration::from_millis(20);

pub fn transports() -> (Transport<StreamDriver>, Transport<StreamDriver>) {
    let (near, far) = tokio::io::duplex(STREAM_BYTES);
    let (near_reader, near_writer) = tokio::io::split(near);
    let (far_reader, far_writer) = tokio::io::split(far);

    (
        StreamDriver::transport(near_reader, near_writer),
        StreamDriver::transport(far_reader, far_writer),
    )
}

pub fn pair() -> (
    Channel<user::User, StreamDriver>,
    Channel<calculator::Calculator, StreamDriver>,
) {
    let (near, far) = transports();
    let user = Channel::new(&near, ChannelOptions::without_health_checks());
    let calculator = Channel::new(&far, ChannelOptions::without_health_checks());

    Adder.attach(&calculator);

    (user, calculator)
}
