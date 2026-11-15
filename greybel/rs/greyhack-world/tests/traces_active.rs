use greyhack_behaviour::{LogAction, TIME_SCALE, active_trace_seconds};
use greyhack_world_core::TraceKind;
use greyhack_world_guilds::GuildDraft;

use crate::support::traces_active::{BANK_SHARES, LEVEL_FIVE, bank};

#[tokio::test]
async fn should_start_a_trace_that_lasts_per_bounce_and_experience() {
    // Arrange
    let (world, _, _, exp, computer) = bank().await;
    let now = world.handle.time_seconds();

    // Act
    let started = world
        .modules
        .traces
        .traces
        .trace_action(computer, 2, LogAction::Remove, 7)
        .await
        .unwrap();
    let again = world
        .modules
        .traces
        .traces
        .trace_action(computer, 2, LogAction::Remove, 7)
        .await
        .unwrap();
    let other = world
        .modules
        .traces
        .traces
        .trace_action(computer, 1, LogAction::Remove, 8)
        .await
        .unwrap();

    // Assert
    let traces = world.modules.traces.traces.active_traces().await.unwrap();

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
    let (world, network, address, _, computer) = bank().await;
    let none = world
        .modules
        .traces
        .traces
        .trace_action(computer, 1, LogAction::Open, 1)
        .await
        .unwrap();
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

    // Act
    world
        .modules
        .npcs
        .jobs
        .fire_npc(&address, &user)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let fired = world
        .modules
        .traces
        .traces
        .trace_action(computer, 1, LogAction::Remove, 2)
        .await
        .unwrap();

    // Assert
    assert!(!none);
    assert!(!fired);
    assert!(
        world
            .modules
            .traces
            .traces
            .active_traces()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn should_not_trace_the_member_of_a_guild_that_owns_the_company() {
    // Arrange
    let (world, _, address, _, computer) = bank().await;

    world
        .modules
        .guilds
        .guilds
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
        .modules
        .traces
        .traces
        .trace_action(computer, 1, LogAction::Remove, 1)
        .await
        .unwrap();

    world
        .modules
        .guilds
        .guilds
        .set_company_shares(&address, "owls-bank", BANK_SHARES / 2)
        .await
        .unwrap();

    let half = world
        .modules
        .traces
        .traces
        .trace_action(computer, 1, LogAction::Remove, 2)
        .await
        .unwrap();

    world
        .modules
        .guilds
        .guilds
        .set_company_shares(&address, "owls-bank", BANK_SHARES)
        .await
        .unwrap();

    // Act
    let owned = world
        .modules
        .traces
        .traces
        .trace_action(computer, 1, LogAction::Remove, 3)
        .await
        .unwrap();
    let passive = world
        .modules
        .traces
        .traces
        .add_passive_trace(computer)
        .await
        .unwrap();

    // Assert
    assert!(before);
    assert!(half);
    assert!(!owned);
    assert!(!passive);
    assert_eq!(
        world
            .modules
            .traces
            .traces
            .active_traces()
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn should_give_a_strike_when_an_active_trace_runs_out() {
    // Arrange
    let (world, _, _, exp, computer) = bank().await;

    world
        .modules
        .traces
        .traces
        .trace_action(computer, 1, LogAction::Remove, 1)
        .await
        .unwrap();
    world.dispatch_events();

    // Act
    world
        .tick(active_trace_seconds(1, exp).ceil() as u64 * TIME_SCALE)
        .await
        .unwrap();

    // Assert
    let strikes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

    world.assembly.events.on({
        let strikes = strikes.clone();

        move |event: &greyhack_world_traces::StrikeEvent| strikes.borrow_mut().push(*event)
    });
    world.dispatch_events();
    assert!(
        strikes
            .borrow()
            .iter()
            .any(|event| event.trace == TraceKind::Active && event.strikes == 1)
    );
    assert_eq!(world.modules.traces.traces.strikes().await.unwrap(), 1);
    assert!(
        world
            .modules
            .traces
            .traces
            .active_traces()
            .await
            .unwrap()
            .is_empty()
    );
}

/// Only a deletion on the computer of the last shell of the program's terminal traces
/// (`HelperScript.AddActiveTraceFromAction`): the player's computer when the tree holds no terminal.
#[tokio::test]
async fn should_trace_a_deletion_only_on_the_computer_of_the_last_shell() {
    use greyhack_behaviour::{FileData, PermissionSet, ROOT_USER};
    use greyhack_world_core::{DeletionOrigin, NodeDraft};

    // Arrange
    let (world, _, _, _, company) = bank().await;
    let player = world.handle.player();
    let root = world
        .modules
        .core
        .files
        .lookup
        .root(company)
        .await
        .unwrap()
        .unwrap();
    let draft = |name: &str| {
        NodeDraft::file(
            name,
            ROOT_USER,
            PermissionSet::parse("-rwxrwxrwx").unwrap(),
            FileData::text("x"),
        )
    };
    let elsewhere = world
        .modules
        .libraries
        .generator
        .add_full_node(&root, &draft("elsewhere"))
        .await
        .unwrap();
    let held = world
        .modules
        .libraries
        .generator
        .add_full_node(&root, &draft("held"))
        .await
        .unwrap();
    let origin = |last_shell| DeletionOrigin {
        prev_ip: "1.1.1.1",
        path: "/file",
        bounces: 1,
        terminal: 1,
        last_shell,
    };

    // Act
    world
        .modules
        .core
        .files
        .scripts
        .delete_entry(&elsewhere, ROOT_USER, &origin(player))
        .await
        .unwrap()
        .unwrap();
    world.assembly.events.dispatch_async().await.unwrap();

    let after_other = world
        .modules
        .traces
        .traces
        .active_traces()
        .await
        .unwrap()
        .len();

    world
        .modules
        .core
        .files
        .scripts
        .delete_entry(&held, ROOT_USER, &origin(company))
        .await
        .unwrap()
        .unwrap();
    world.assembly.events.dispatch_async().await.unwrap();

    // Assert
    assert_eq!(after_other, 0);
    assert_eq!(
        world
            .modules
            .traces
            .traces
            .active_traces()
            .await
            .unwrap()
            .len(),
        1
    );
}
