use greybel_test_support::future::block_on;
use greyhack_world::{PlayerOptions, World};

use crate::support::save_copy;
use crate::support::scripts::run_code_in;

#[test]
fn should_log_an_npc_in_and_out_on_due_ticks() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.player();
    let npc = block_on(world.add_npc(player, "test", "test npc", world.time_seconds() + 60))
        .unwrap()
        .unwrap();

    // Presence follows the NPC's work schedule: at every point the session exists exactly when the NPC is online.
    let mut online_seen = false;
    let mut offline_seen = false;

    for _ in 0..24 {
        block_on(world.tick(3_600)).unwrap();

        let online = block_on(world.npc(npc)).unwrap().unwrap().online;
        let sessions = block_on(world.sessions(player)).unwrap();

        assert_eq!(sessions.contains(&"test".to_owned()), online);
        online_seen |= online;
        offline_seen |= !online;
    }

    assert!(online_seen, "the NPC works at some point on a weekday");
    assert!(offline_seen, "the NPC rests at some point");
}

#[tokio::test]
async fn should_preserve_npcs_and_their_next_action_across_a_reload() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let npc = world
        .add_npc(player, "test", "test npc", world.time_seconds() + 60)
        .await
        .unwrap()
        .unwrap();

    world.tick(60).await.unwrap();

    let database = save_copy(&mut world).await;

    let mut restored = World::load(&database).await.unwrap();

    // After a reload the NPC keeps following its schedule: the next switch leaves its session matching its online
    // state.
    restored.tick(3_600).await.unwrap();

    let online = restored.npc(npc).await.unwrap().unwrap().online;

    assert_eq!(
        restored
            .sessions(player)
            .await
            .unwrap()
            .contains(&"test".to_owned()),
        online
    );
}

#[test]
fn should_control_an_npc_through_admin() {
    let code = r#"
admin = include_lib("admin.so")
admin.set_date("01/Jan/1970 - 00:00")
admin.network(get_router.public_ip).computer(get_shell.host_computer.local_ip).add_npc("test", 60)
print admin.tick(60)
print admin.tick(60)
"#;

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "60\n120");
}

#[test]
fn should_fire_and_hire_npcs_and_a_rented_servers_admin_through_admin() {
    let code = r#"
admin = include_lib("admin.so")
network = admin.network(get_router.public_ip)
network.computer(get_shell.host_computer.local_ip).add_npc("test", 60)
print network.fire_npc("nobody")
print network.hire_npc("Sales")
print network.hire_npc("Sergeant")
print network.fire_npc("test")
print network.hire_npc("None")
print network.fire_admin
print network.hire_admin
print network.hire_admin
print network.fire_admin
print admin.network("1.2.3.4").fire_npc("test")
"#;

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "Unknown error: Unable to find person to fire.\nThere are no open positions in the company for Sales\nError: invalid job option\n1\n1\nYou don't have any administrator hired on this network.\n1\nYou already have an administrator hired for this network.\n1\nnull"
    );
}
