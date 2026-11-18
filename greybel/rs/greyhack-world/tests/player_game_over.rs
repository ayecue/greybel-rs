use greyhack_behaviour::{SubscriptionKind, exp_after_member_exit};
use greyhack_world_core::TraceKind;
use greyhack_world_guilds::GuildDraft;

use crate::support::traces_active::{BANK_SHARES, LEVEL_FIVE, bank};

#[tokio::test]
async fn should_end_the_game_on_the_third_strike() {
    // Arrange
    let (world, network, address, _, company) = bank().await;
    let player = world.handle.player();

    world
        .modules
        .guilds
        .guilds
        .create_guild(&GuildDraft {
            name: "Owls".to_owned(),
            bank_account: "test".to_owned(),
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            admin_restore_perks: 0,
            player_member: true,
            player_owner: false,
        })
        .await
        .unwrap()
        .unwrap();
    world
        .modules
        .guilds
        .guilds
        .set_company_shares(&address, "test", BANK_SHARES / 2)
        .await
        .unwrap();
    world
        .modules
        .economy
        .bank
        .add_subscription("test", SubscriptionKind::Bank, 50, "")
        .await
        .unwrap();
    world
        .modules
        .economy
        .bank
        .add_subscription("test", SubscriptionKind::Rented, 40, &address)
        .await
        .unwrap();
    world
        .modules
        .core
        .networks
        .settings
        .set_home_network(&address, true)
        .await
        .unwrap();
    world.modules.traces.traces.set_strikes(2).await.unwrap();
    world.dispatch_events();
    world.take_changed_computers();

    // Act
    world
        .modules
        .traces
        .traces
        .strike_player(company, TraceKind::Active)
        .await
        .unwrap();

    // Assert

    let strikes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

    world.assembly.events.on({
        let strikes = strikes.clone();

        move |event: &greyhack_world_traces::StrikeEvent| strikes.borrow_mut().push(*event)
    });
    world.assembly.events.dispatch_async().await.unwrap();
    assert!(
        strikes
            .borrow()
            .iter()
            .any(|event| event.trace == TraceKind::Active && event.strikes == 3)
    );

    let guild = world
        .modules
        .guilds
        .guilds
        .guilds()
        .await
        .unwrap()
        .remove(0);
    let subscriptions = world
        .modules
        .economy
        .bank
        .bank_subscriptions()
        .await
        .unwrap();

    assert!(!world.has_player());
    assert!(
        world
            .modules
            .core
            .networks
            .topology
            .computer(player)
            .await
            .unwrap()
            .is_none()
    );
    assert!(world.take_changed_computers().contains(&player));
    assert_eq!(world.modules.traces.traces.strikes().await.unwrap(), 0);
    assert_eq!(guild.total_exp, exp_after_member_exit(LEVEL_FIVE));
    assert!(!guild.player_member);
    assert!(
        world
            .modules
            .guilds
            .guilds
            .company_shares(network)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(subscriptions.len(), 1);
    assert_eq!(subscriptions[0].kind, SubscriptionKind::Rented);
    assert!(
        world
            .modules
            .core
            .networks
            .settings
            .is_disabled(
                world
                    .modules
                    .core
                    .networks
                    .routers
                    .router_at(&address)
                    .await
                    .unwrap()
                    .unwrap()
                    .id
                    .computer()
            )
            .await
            .unwrap()
    );
    assert!(
        !world
            .modules
            .core
            .networks
            .settings
            .is_home_network(
                world
                    .modules
                    .core
                    .networks
                    .routers
                    .router_at(&address)
                    .await
                    .unwrap()
                    .unwrap()
                    .id
                    .computer()
            )
            .await
            .unwrap(),
        "Router.ResetHomeNetwork clears the router's home marker"
    );
}

#[tokio::test]
async fn should_keep_the_guild_owner_as_a_member_after_game_over() {
    let (world, _, _, _, _) = bank().await;

    world
        .modules
        .guilds
        .guilds
        .create_guild(&GuildDraft {
            name: "Owners".to_owned(),
            bank_account: "test".to_owned(),
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            admin_restore_perks: 0,
            player_member: true,
            player_owner: true,
        })
        .await
        .unwrap()
        .unwrap();

    world.modules.player.game_over.end_game().await.unwrap();

    let guilds = world.modules.guilds.guilds.guilds().await.unwrap();
    assert_eq!(guilds.len(), 1, "the guild itself is not deleted");
    assert!(guilds[0].player_member);
    assert!(guilds[0].player_owner);
    assert_eq!(guilds[0].total_exp, exp_after_member_exit(LEVEL_FIVE));
}

#[tokio::test]
async fn should_keep_the_player_for_the_first_two_strikes() {
    // Arrange
    let (world, _, _, _, company) = bank().await;
    let player = world.handle.player();

    // Act
    world
        .modules
        .traces
        .traces
        .strike_player(company, TraceKind::Active)
        .await
        .unwrap();
    world
        .modules
        .traces
        .traces
        .strike_player(company, TraceKind::Passive)
        .await
        .unwrap();

    // Assert
    assert_eq!(world.modules.traces.traces.strikes().await.unwrap(), 2);
    assert!(world.has_player());
    assert!(
        world
            .modules
            .core
            .networks
            .topology
            .computer(player)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn should_stop_every_trace_when_the_game_ends() {
    // Arrange
    let (world, _, _, _, company) = bank().await;

    world
        .modules
        .traces
        .traces
        .trace_action(company, 5, greyhack_behaviour::LogAction::Remove, 1)
        .await
        .unwrap();
    world
        .modules
        .traces
        .traces
        .add_passive_trace(company)
        .await
        .unwrap();
    world.modules.traces.traces.set_strikes(2).await.unwrap();

    // Act
    world
        .modules
        .traces
        .traces
        .strike_player(company, TraceKind::Passive)
        .await
        .unwrap();
    world.assembly.events.dispatch_async().await.unwrap();

    // Assert
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
    assert!(
        world
            .modules
            .traces
            .traces
            .passive_traces()
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        world
            .modules
            .routines
            .scheduled_routines()
            .await
            .unwrap()
            .iter()
            .all(|entry| entry.key != "traces.active_trace")
    );
}
