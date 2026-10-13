use greyhack_behaviour::SECONDS_PER_DAY;

use crate::{NetworkOptions, PlayerOptions, World};

#[tokio::test]
async fn should_hide_an_npcs_smartphone_while_the_npc_isnt_at_work() {
    // Arrange
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = world.add_network(&NetworkOptions::default()).await.unwrap();
    let network = world.db.router(router).await.unwrap().unwrap().network;
    let phone = world
        .db
        .computers_in(network)
        .await
        .unwrap()
        .into_iter()
        .find(|computer| computer.npc_owner.is_some())
        .expect("the network has an NPC's smartphone");
    let owner = world
        .db
        .npc(phone.npc_owner.unwrap())
        .await
        .unwrap()
        .unwrap();
    let schedule = owner.schedule.clone();
    let start = world.time_seconds;
    let mut times = (start..start + 7 * SECONDS_PER_DAY).step_by(900);
    let at_work = times.clone().find(|at| schedule.is_at_work(*at)).unwrap();
    let off_work = times.find(|at| !schedule.is_at_work(*at)).unwrap();

    // Act
    world.time_seconds = at_work;

    let lan = world.lan(network).await.unwrap().unwrap();
    let visible = !lan.node(lan.find(&phone.local_ip).unwrap()).hidden;

    world.time_seconds = off_work;

    let lan = world.lan(network).await.unwrap().unwrap();
    let hidden = lan.node(lan.find(&phone.local_ip).unwrap()).hidden;

    // Assert
    assert!(visible);
    assert!(hidden);
}

#[tokio::test]
async fn should_kill_the_npcs_who_walk_to_an_overridden_appliance_soon() {
    // Arrange
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = world.add_network(&NetworkOptions::default()).await.unwrap();
    let network = world.db.router(router).await.unwrap().unwrap().network;
    let computers = world.db.computers_in(network).await.unwrap();
    let phone = computers
        .iter()
        .find(|computer| computer.npc_owner.is_some())
        .expect("the network has an NPC's smartphone")
        .clone();
    let npc = phone.npc_owner.unwrap();
    let schedule = world.db.npc(npc).await.unwrap().unwrap().schedule;
    let monday = 946_857_600 + 7 * SECONDS_PER_DAY;
    let walk = (monday..monday + SECONDS_PER_DAY)
        .step_by(60)
        .find(|at| schedule.walks_to_appliance_soon(*at) && schedule.is_at_work(*at + 6 * 60))
        .expect("the NPC walks to the appliance during the break");

    world.time_seconds = walk;

    // Act
    let killed = world.kill_npcs_at_appliance(phone.id).await.unwrap();
    let alive_meanwhile = !world
        .db
        .npc(npc)
        .await
        .unwrap()
        .unwrap()
        .is_dead(walk + 5 * 60 - 1);

    world.time_seconds = walk + 6 * 60;

    let lan = world.lan(network).await.unwrap().unwrap();
    let hidden = lan.node(lan.find(&phone.local_ip).unwrap()).hidden;
    let again = world.kill_npcs_at_appliance(phone.id).await.unwrap();

    // Assert
    assert!(killed.contains(&npc));
    assert!(alive_meanwhile);
    assert!(
        world
            .db
            .npc(npc)
            .await
            .unwrap()
            .unwrap()
            .is_dead(world.time_seconds)
    );
    assert!(hidden);
    assert!(!again.contains(&npc));
}
