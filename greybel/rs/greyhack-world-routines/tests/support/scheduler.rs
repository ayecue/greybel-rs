use greyhack_world_routines::Window;

use super::{Counter, Greeting, Host, TestScheduler};

pub fn window(now: u64) -> Window {
    Window {
        now,
        raise_after: 60,
    }
}

pub async fn run_all(scheduler: &TestScheduler, host: &mut Host, now: u64) -> Vec<String> {
    let conn = host.conn.clone();

    while scheduler
        .run_next_due(&conn, window(now))
        .await
        .unwrap()
        .is_some()
    {}

    host.state.borrow().output.clone()
}

pub async fn greet(scheduler: &TestScheduler, host: &Host, at: u64, text: &str) -> u64 {
    scheduler
        .schedule::<Greeting>(&host.conn, at, &text.to_owned())
        .await
        .unwrap()
}

pub async fn count(scheduler: &TestScheduler, host: &Host, at: u64, amount: i64) -> u64 {
    scheduler
        .schedule::<Counter>(&host.conn, at, &amount)
        .await
        .unwrap()
}
