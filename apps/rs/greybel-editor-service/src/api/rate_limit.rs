use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::Request;

use super::ClientAddress;

/// Allows each client `limit` saves per `window` (a fixed window per client IP), in memory.
#[derive(Clone)]
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    client: ClientAddress,
    windows: Arc<Mutex<HashMap<String, SaveWindow>>>,
}

#[derive(Clone, Copy)]
struct SaveWindow {
    start: Instant,
    count: u32,
}

impl RateLimiter {
    const PRUNE_SIZE: usize = 10_000;

    /// `client_ip_header` names the header a proxy sets to the client's IP, such as App Platform's
    /// `do-connecting-ip`; without it, or when a request lacks it, the connection's address counts.
    pub fn new(limit: u32, window: Duration, client_ip_header: Option<String>) -> Self {
        Self {
            limit,
            window,
            client: ClientAddress::new(client_ip_header),
            windows: Arc::default(),
        }
    }

    pub fn client_address(&self) -> &ClientAddress {
        &self.client
    }

    /// Counts a save of the client of `request`, or returns how long it has to wait.
    pub fn check(&self, request: &Request) -> Result<(), Duration> {
        self.check_client(&self.client.find(request), Instant::now())
    }

    fn check_client(&self, client: &str, now: Instant) -> Result<(), Duration> {
        let mut windows = self.windows.lock().unwrap_or_else(PoisonError::into_inner);

        if windows.len() >= Self::PRUNE_SIZE {
            windows.retain(|_, window| now.duration_since(window.start) < self.window);
        }

        let window = windows.entry(client.to_owned()).or_insert(SaveWindow {
            start: now,
            count: 0,
        });

        if now.duration_since(window.start) >= self.window {
            *window = SaveWindow {
                start: now,
                count: 0,
            };
        }

        if window.count >= self.limit {
            return Err(self.window.saturating_sub(now.duration_since(window.start)));
        }

        window.count += 1;

        Ok(())
    }
}
