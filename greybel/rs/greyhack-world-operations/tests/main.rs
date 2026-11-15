#![expect(
    clippy::too_many_lines,
    reason = "Integration tests read top to bottom"
)]

mod admin_computer;
mod clock;
mod coin;
mod router;
mod rules;
mod server;
mod support;
