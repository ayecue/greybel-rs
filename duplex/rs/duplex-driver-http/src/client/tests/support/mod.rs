use std::time::Duration;

use super::*;

pub const LIMIT: usize = 1024;

pub const SWEEP: Duration = Duration::from_secs(1);

pub const WAITED: Duration = Duration::from_millis(50);

pub fn poll_after(client: &Client, sequence: u64) -> u64 {
    poll_within(client, sequence, Duration::from_millis(1))
}

pub fn poll_within(client: &Client, sequence: u64, timeout: Duration) -> u64 {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("Tokio's runtime builds");

    runtime.block_on(client.poll_after(sequence, timeout))
}
