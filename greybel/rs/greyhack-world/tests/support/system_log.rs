use greyhack_behaviour::{LogAction, SYSTEM_LOG_PATH, SystemLog};
use greyhack_world::{ComputerId, LogWrite, PlayerOptions, World};

pub const FIRST: &str = "03/Dec/2000 - 08:30";
pub const SECOND: &str = "04/Dec/2000 - 09:45";

pub async fn world_at(date: &str) -> World {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();

    assert!(world.set_date(date));

    world
}

pub fn write(action: LogAction, ip: &str) -> LogWrite<'_> {
    LogWrite {
        action,
        ip,
        port: 22,
        file: "",
        bounce_ip: "",
    }
}

pub async fn log_of(world: &World, computer: ComputerId) -> SystemLog {
    let content = world
        .content(computer, SYSTEM_LOG_PATH, false)
        .await
        .unwrap()
        .expect("the system log");

    SystemLog::parse(&content).expect("a system log")
}

pub async fn log_time(world: &World, computer: ComputerId) -> u64 {
    world
        .file(computer, SYSTEM_LOG_PATH, false)
        .await
        .unwrap()
        .expect("the system log")
        .timestamp
}
