use greyhack_behaviour::{LogAction, PASSIVE_NEXT_ADMIN_MS, TIME_SCALE};
use greyhack_world::{LogWrite, TraceKind};

use crate::support::guilds_active_traces::bank;

#[tokio::test]
async fn should_start_a_passive_trace_instead_when_the_program_ends() {
    // Arrange
    let (world, _, _, _, computer) = bank().await;

    world
        .modules
        .guilds
        .traces
        .trace_action(computer, 1, LogAction::Remove, 1)
        .await
        .unwrap();

    // Act
    world
        .modules
        .guilds
        .traces
        .cancel_active_traces(&[1])
        .await
        .unwrap();

    // Assert
    assert!(
        world
            .modules
            .guilds
            .traces
            .active_traces()
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        world.modules.guilds.traces.passive_traces().await.unwrap(),
        [computer]
    );
    assert_eq!(world.modules.guilds.traces.strikes().await.unwrap(), 0);
}

#[tokio::test]
async fn should_forget_a_passive_trace_of_a_computer_without_a_log() {
    // Arrange
    let (world, _, _, _, computer) = bank().await;

    world
        .modules
        .guilds
        .traces
        .add_passive_trace(computer)
        .await
        .unwrap();

    // Act
    world
        .tick(PASSIVE_NEXT_ADMIN_MS / 1000 * TIME_SCALE + 1)
        .await
        .unwrap();

    // Assert
    assert!(
        world
            .modules
            .guilds
            .traces
            .passive_traces()
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(world.modules.guilds.traces.strikes().await.unwrap(), 0);
}

#[tokio::test]
async fn should_follow_a_passive_trace_back_to_the_player_and_renew_their_token() {
    // Arrange
    let (world, _, address, _, computer) = bank().await;
    let player = world.handle.player();
    let player_ip = world
        .modules
        .core
        .networks
        .topology
        .public_ip(player)
        .await
        .unwrap();
    let local_ip = world
        .modules
        .core
        .networks
        .topology
        .computer(player)
        .await
        .unwrap()
        .unwrap()
        .local_ip;
    let player_router = world
        .modules
        .core
        .networks
        .routers
        .router_at(&player_ip)
        .await
        .unwrap()
        .unwrap();
    let before = world
        .modules
        .guilds
        .traces
        .player_token_trace()
        .await
        .unwrap();

    world
        .modules
        .core
        .computers
        .logs
        .write_log(
            computer,
            &LogWrite {
                action: LogAction::Remove,
                ip: &player_ip,
                port: 0,
                file: "/x",
                bounce_ip: "",
            },
        )
        .await
        .unwrap();
    world
        .modules
        .core
        .computers
        .logs
        .write_log(
            player_router.id.computer(),
            &LogWrite {
                action: LogAction::Bounce,
                ip: &local_ip,
                port: 0,
                file: "",
                bounce_ip: &address,
            },
        )
        .await
        .unwrap();
    world
        .modules
        .guilds
        .traces
        .add_passive_trace(computer)
        .await
        .unwrap();
    world.deliver_events();

    // Act
    for _ in 0..200 {
        world.tick(60).await.unwrap();
    }

    // Assert
    assert_eq!(world.modules.guilds.traces.strikes().await.unwrap(), 1);
    assert_ne!(
        world
            .modules
            .guilds
            .traces
            .player_token_trace()
            .await
            .unwrap(),
        before
    );
    assert!(
        world
            .modules
            .guilds
            .traces
            .passive_traces()
            .await
            .unwrap()
            .is_empty()
    );

    let strikes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

    world.assembly.events.on({
        let strikes = strikes.clone();

        move |event: &greyhack_world::StrikeEvent| strikes.borrow_mut().push(*event)
    });
    world.deliver_events();
    assert!(
        strikes
            .borrow()
            .iter()
            .any(|event| event.trace == TraceKind::Passive)
    );
}
