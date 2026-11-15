use greyhack_behaviour::{LogAction, SYSTEM_LOG_PATH, SystemLog};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::{ComputerId, LogWrite};

pub const FIRST: &str = "03/Dec/2000 - 08:30";
pub const SECOND: &str = "04/Dec/2000 - 09:45";

pub async fn world_at(date: &str) -> World {
    let world = World::new(&PlayerOptions::default()).await.unwrap();

    assert!(world.modules.routines.set_date(date));

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
        .modules
        .core
        .files
        .lookup
        .content(computer, SYSTEM_LOG_PATH, false)
        .await
        .unwrap()
        .expect("the system log");

    SystemLog::parse(&content).expect("a system log")
}

pub async fn log_time(world: &World, computer: ComputerId) -> u64 {
    world
        .modules
        .core
        .files
        .lookup
        .file(computer, SYSTEM_LOG_PATH, false)
        .await
        .unwrap()
        .expect("the system log")
        .timestamp
}
