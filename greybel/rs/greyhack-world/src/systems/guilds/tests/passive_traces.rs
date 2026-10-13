use greyhack_behaviour::{LogAction, PASSIVE_NEXT_ADMIN_MS, TIME_SCALE};

use crate::systems::guilds::active_traces::tests::support::bank;
use crate::{LogWrite, TraceKind, WorldEvent};

#[tokio::test]
async fn should_start_a_passive_trace_instead_when_the_program_ends() {
    // Arrange
    let (mut world, _, _, _, computer) = bank().await;

    world
        .trace_action(computer, 1, LogAction::Remove, 1)
        .await
        .unwrap();

    // Act
    world.cancel_active_traces(&[1]).await.unwrap();

    // Assert
    assert!(world.active_traces().await.unwrap().is_empty());
    assert_eq!(world.passive_traces().await.unwrap(), [computer]);
    assert_eq!(world.strikes().await.unwrap(), 0);
}

#[tokio::test]
async fn should_forget_a_passive_trace_of_a_computer_without_a_log() {
    // Arrange
    let (mut world, _, _, _, computer) = bank().await;

    world.add_passive_trace(computer).await.unwrap();

    // Act
    world
        .tick(PASSIVE_NEXT_ADMIN_MS / 1000 * TIME_SCALE + 1)
        .await
        .unwrap();

    // Assert
    assert!(world.passive_traces().await.unwrap().is_empty());
    assert_eq!(world.strikes().await.unwrap(), 0);
}

#[tokio::test]
async fn should_follow_a_passive_trace_back_to_the_player_and_renew_their_token() {
    // Arrange
    let (mut world, _, address, _, computer) = bank().await;
    let player = world.player();
    let player_ip = world.public_ip(player).await.unwrap();
    let local_ip = world.computer(player).await.unwrap().unwrap().local_ip;
    let player_router = world.router_at(&player_ip).await.unwrap().unwrap();
    let before = world.player_token_trace().await.unwrap();

    world
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
    world.add_passive_trace(computer).await.unwrap();
    world.take_events();

    // Act
    for _ in 0..200 {
        world.tick(60).await.unwrap();
    }

    // Assert
    assert_eq!(world.strikes().await.unwrap(), 1);
    assert_ne!(world.player_token_trace().await.unwrap(), before);
    assert!(world.passive_traces().await.unwrap().is_empty());
    assert!(world.take_events().iter().any(|event| matches!(
        event,
        WorldEvent::GameOver {
            trace: TraceKind::Passive,
            ..
        }
    )));
}
