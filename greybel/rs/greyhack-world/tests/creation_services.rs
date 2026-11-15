use greyhack_behaviour::SECONDS_PER_DAY;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_creation::NetworkOptions;

#[tokio::test]
async fn should_register_every_lazy_dependency() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();

    assert_eq!(
        world.registry.missing_services(),
        Vec::<&'static str>::new()
    );
    assert_eq!(world.assembly.routes.len(), 228);
}

#[tokio::test]
async fn should_hide_an_npcs_smartphone_while_the_npc_isnt_at_work() {
    // Arrange
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = world
        .modules
        .creation
        .network_generation
        .add_network(&NetworkOptions::default())
        .await
        .unwrap();
    let network = world
        .modules
        .core
        .networks
        .router(router)
        .await
        .unwrap()
        .unwrap()
        .network;
    let phone = world
        .modules
        .core
        .computers
        .computers_in(network)
        .await
        .unwrap()
        .into_iter()
        .find(|computer| computer.npc_owner.is_some())
        .expect("the network has an NPC's smartphone");
    let owner = world
        .modules
        .npcs
        .npcs
        .npc(phone.npc_owner.unwrap())
        .await
        .unwrap()
        .unwrap();
    let schedule = owner.schedule.clone();
    let start = world.handle.time_seconds();
    let mut times = (start..start + 7 * SECONDS_PER_DAY).step_by(900);
    let at_work = times.clone().find(|at| schedule.is_at_work(*at)).unwrap();
    let off_work = times.find(|at| !schedule.is_at_work(*at)).unwrap();

    // Act
    world.handle.world_mut().time_seconds = at_work;

    let lan = world
        .modules
        .core
        .networks
        .lan
        .lan(network)
        .await
        .unwrap()
        .unwrap();
    let visible = !lan.node(lan.find(&phone.local_ip).unwrap()).hidden;

    world.handle.world_mut().time_seconds = off_work;

    let lan = world
        .modules
        .core
        .networks
        .lan
        .lan(network)
        .await
        .unwrap()
        .unwrap();
    let hidden = lan.node(lan.find(&phone.local_ip).unwrap()).hidden;

    // Assert
    assert!(visible);
    assert!(hidden);
}

#[tokio::test]
async fn should_kill_the_npcs_who_walk_to_an_overridden_appliance_soon() {
    // Arrange
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = world
        .modules
        .creation
        .network_generation
        .add_network(&NetworkOptions::default())
        .await
        .unwrap();
    let network = world
        .modules
        .core
        .networks
        .router(router)
        .await
        .unwrap()
        .unwrap()
        .network;
    let computers = world
        .modules
        .core
        .computers
        .computers_in(network)
        .await
        .unwrap();
    let phone = computers
        .iter()
        .find(|computer| computer.npc_owner.is_some())
        .expect("the network has an NPC's smartphone")
        .clone();
    let npc = phone.npc_owner.unwrap();
    let schedule = world
        .modules
        .npcs
        .npcs
        .npc(npc)
        .await
        .unwrap()
        .unwrap()
        .schedule;
    let monday = 946_857_600 + 7 * SECONDS_PER_DAY;
    let walk = (monday..monday + SECONDS_PER_DAY)
        .step_by(60)
        .find(|at| schedule.walks_to_appliance_soon(*at) && schedule.is_at_work(*at + 6 * 60))
        .expect("the NPC walks to the appliance during the break");

    world.handle.world_mut().time_seconds = walk;

    // Act
    let killed = world
        .modules
        .npcs
        .schedules
        .kill_npcs_at_appliance(phone.id)
        .await
        .unwrap();
    let alive_meanwhile = !world
        .modules
        .npcs
        .npcs
        .npc(npc)
        .await
        .unwrap()
        .unwrap()
        .is_dead(walk + 5 * 60 - 1);

    world.handle.world_mut().time_seconds = walk + 6 * 60;

    let lan = world
        .modules
        .core
        .networks
        .lan
        .lan(network)
        .await
        .unwrap()
        .unwrap();
    let hidden = lan.node(lan.find(&phone.local_ip).unwrap()).hidden;
    let again = world
        .modules
        .npcs
        .schedules
        .kill_npcs_at_appliance(phone.id)
        .await
        .unwrap();

    // Assert
    assert!(killed.contains(&npc));
    assert!(alive_meanwhile);
    assert!(
        world
            .modules
            .npcs
            .npcs
            .npc(npc)
            .await
            .unwrap()
            .unwrap()
            .is_dead(world.handle.time_seconds())
    );
    assert!(hidden);
    assert!(!again.contains(&npc));
}
