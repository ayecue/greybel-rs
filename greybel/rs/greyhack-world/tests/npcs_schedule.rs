use greyhack_behaviour::{NetworkKind, SECONDS_PER_DAY, day_of_year};
use greyhack_world::{DatabaseOpen, World};
use greyhack_world_core::Database;

use crate::support::npcs_jobs::{public_ip, world_with};

#[tokio::test]
/// Lateness doesn't log an NPC out, as being logged in follows only the schedule (`Persona.IsLogged`); a completed one
/// counts as a strike, and the third fires the NPC (`Persona.NextCita`).
async fn should_count_late_npcs_as_logged_in_and_fire_on_the_third_strike() {
    let (world, network) = world_with(NetworkKind::Supermarket).await;
    let address = public_ip(&world, network).await;
    let admin = world
        .modules
        .npcs
        .npcs
        .network_admin_person(network)
        .await
        .unwrap()
        .unwrap();
    let user = world
        .modules
        .npcs
        .npcs
        .npc(admin)
        .await
        .unwrap()
        .unwrap()
        .user;

    for strike in 1..=3 {
        world
            .modules
            .npcs
            .schedules
            .set_phone_alarm(&address, &user, 0, 0, true)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let npc = world.modules.npcs.npcs.npc(admin).await.unwrap().unwrap();
        let work_start = npc.schedule.work1_start();
        let midnight = world.handle.time_seconds() / SECONDS_PER_DAY * SECONDS_PER_DAY;
        let mut work = midnight + work_start;

        while work <= world.handle.time_seconds() || !npc.schedule.is_working(work) {
            work += SECONDS_PER_DAY;
        }

        world
            .tick(work - world.handle.time_seconds())
            .await
            .unwrap();

        let late = world.modules.npcs.npcs.npc(admin).await.unwrap().unwrap();

        assert!(late.is_late_to_work(world.handle.time_seconds()).late);
        assert!(late.online, "a late administrator is logged in");

        let wake = late.wake_up_time.unwrap();
        let mut alarm_day = world.handle.time_seconds() / SECONDS_PER_DAY * SECONDS_PER_DAY;

        if late.day_alarm != day_of_year(world.handle.time_seconds()) {
            alarm_day += SECONDS_PER_DAY;
        }

        world
            .tick(alarm_day + wake - world.handle.time_seconds())
            .await
            .unwrap();

        let after = world.modules.npcs.npcs.npc(admin).await.unwrap().unwrap();

        assert_eq!(
            after.late_strikes,
            strike,
            "{after:?} at {}",
            world.handle.time_seconds()
        );
        assert_eq!(after.fired, strike == 3);
        assert_eq!(after.wake_up_time, None);
        assert!(!after.alarm.is_modified);
    }

    let database = Database::in_memory().await.unwrap();

    world.save_into(&database).await.unwrap();

    let restored = World::load(&database).await.unwrap();
    let npc = restored
        .modules
        .npcs
        .npcs
        .npc(admin)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(npc.late_strikes, 3);
    assert!(npc.fired);
}
