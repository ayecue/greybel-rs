#[path = "support/active_traces.rs"]
pub(in crate::systems) mod support;

use greyhack_behaviour::{LogAction, TIME_SCALE, active_trace_seconds};
use support::{BANK_SHARES, LEVEL_FIVE, bank};

use crate::{GuildDraft, TraceKind, WorldEvent};

#[tokio::test]
async fn should_start_a_trace_that_lasts_per_bounce_and_experience() {
    // Arrange
    let (mut world, _, _, exp, computer) = bank().await;
    let now = world.time_seconds();

    // Act
    let started = world
        .trace_action(computer, 2, LogAction::Remove, 7)
        .await
        .unwrap();
    let again = world
        .trace_action(computer, 2, LogAction::Remove, 7)
        .await
        .unwrap();
    let other = world
        .trace_action(computer, 1, LogAction::Remove, 8)
        .await
        .unwrap();

    // Assert
    let traces = world.active_traces().await.unwrap();

    assert!(started);
    assert!(!again);
    assert!(other);
    assert_eq!(traces.len(), 2);
    assert_eq!(
        traces[0].ends_at,
        now + active_trace_seconds(2, exp).ceil() as u64 * TIME_SCALE
    );
}

#[tokio::test]
async fn should_not_trace_a_dead_or_fired_administrator_or_an_action_that_does_not_trigger() {
    // Arrange
    let (mut world, network, address, _, computer) = bank().await;
    let none = world
        .trace_action(computer, 1, LogAction::Open, 1)
        .await
        .unwrap();
    let admin = world
        .db
        .network_admin_person(network)
        .await
        .unwrap()
        .unwrap();
    let user = world.db.npc(admin).await.unwrap().unwrap().user;

    // Act
    world
        .fire_npc(&address, &user)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let fired = world
        .trace_action(computer, 1, LogAction::Remove, 2)
        .await
        .unwrap();

    // Assert
    assert!(!none);
    assert!(!fired);
    assert!(world.active_traces().await.unwrap().is_empty());
}

#[tokio::test]
async fn should_not_trace_the_member_of_a_guild_that_owns_the_company() {
    // Arrange
    let (mut world, _, address, _, computer) = bank().await;

    world
        .create_guild(&GuildDraft {
            name: "Owls".to_owned(),
            bank_account: "owls-bank".to_owned(),
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            admin_restore_perks: 0,
            player_member: true,
        })
        .await
        .unwrap()
        .unwrap();

    let before = world
        .trace_action(computer, 1, LogAction::Remove, 1)
        .await
        .unwrap();

    world
        .set_company_shares(&address, "owls-bank", BANK_SHARES / 2)
        .await
        .unwrap();

    let half = world
        .trace_action(computer, 1, LogAction::Remove, 2)
        .await
        .unwrap();

    world
        .set_company_shares(&address, "owls-bank", BANK_SHARES)
        .await
        .unwrap();

    // Act
    let owned = world
        .trace_action(computer, 1, LogAction::Remove, 3)
        .await
        .unwrap();
    let passive = world.add_passive_trace(computer).await.unwrap();

    // Assert
    assert!(before);
    assert!(half);
    assert!(!owned);
    assert!(!passive);
    assert_eq!(world.active_traces().await.unwrap().len(), 2);
}

#[tokio::test]
async fn should_give_a_strike_when_an_active_trace_runs_out() {
    // Arrange
    let (mut world, _, _, exp, computer) = bank().await;

    world
        .trace_action(computer, 1, LogAction::Remove, 1)
        .await
        .unwrap();
    world.take_events();

    // Act
    world
        .tick(active_trace_seconds(1, exp).ceil() as u64 * TIME_SCALE)
        .await
        .unwrap();

    // Assert
    assert!(world.take_events().iter().any(|event| matches!(
        event,
        WorldEvent::GameOver {
            trace: TraceKind::Active,
            strikes: 1,
            ..
        }
    )));
    assert_eq!(world.strikes().await.unwrap(), 1);
    assert!(world.active_traces().await.unwrap().is_empty());
}
