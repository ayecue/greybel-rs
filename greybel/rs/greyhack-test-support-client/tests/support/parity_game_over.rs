//! What the game-over, last-shell and payment-timing comparisons share: the traces and payments of the game and of the
//! copy of its network while a script waits, and what the world has of the player after the game ended.

use std::cell::RefCell;
use std::rc::Rc;

use greybel_interpreter::{InterpreterOptions, Stdout};
use greybel_test_support::future::block_on;
use greybel_test_support::game::Game;
use greybel_test_support::greyhack::run_in_at;
use greybel_test_support::interpreter::constant_clock;
use greybel_test_support::io::MarkedStdout;
use greyhack_test_support_client::protocol::test_support::{
    GameEndedReply, GuildsReply, LanNodeKind, NetworkSnapshot,
};
use greyhack_test_support_client::{ExecutionStep, GuildSetup, TestClient};
use greyhack_world::{DatabaseOpen, World};
use greyhack_world_core::{ComputerId, Database, TraceKind};
use greyhack_world_operations::WorldServer;
use tokio::sync::mpsc;

use super::game::{empty_network, join_fixture, join_network, setup_player, test_client};
use super::intrinsics::PROGRAM_NAME;
use super::parity_bank_payments::RENTAL_AMOUNT;
use super::parity_guilds::mirrored_world;
use super::parity_traces::{LEVEL_FIVE, TraceService, game_traces, world_traces};
use super::steps::SteppedStdin;

/// The traces of the game and of the copy of its network at the first input `script` waits for: the active traces
/// run for seconds of real time, so only a script that waits sees them.
pub async fn traces_while_waiting(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    guilds: &GuildsReply,
    script: &str,
) -> (TraceService, TraceService) {
    let expected = std::thread::scope(|scope| {
        scope
            .spawn(|| world_traces_while_waiting(script, user, snapshot, guilds))
            .join()
            .expect("the stepped intrinsics run")
    });
    let mut step = client
        .execute_until_input(script, "/project/test.src", "/project", PROGRAM_NAME)
        .await
        .expect("the script starts");
    let mut actual = None;

    while let ExecutionStep::Input = step {
        // The game starts a trace in a task of its own.
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        actual = Some(game_traces(
            &client.query_traces().await.expect("the traces are read"),
        ));
        step = client.type_input("").await.expect("the input is typed");
    }

    (actual.expect("the script waited for input"), expected)
}

fn world_traces_while_waiting(
    script: &str,
    user: &str,
    snapshot: &NetworkSnapshot,
    guilds: &GuildsReply,
) -> TraceService {
    let world = mirrored_world(user, snapshot, guilds);

    duplex_test_support::local::run_local(async move {
        let server = WorldServer::with_database(
            world,
            Database::in_memory().await.expect("an in-memory database"),
            constant_clock(),
        )
        .await
        .expect("the world is written to the database");
        let stdout = Rc::new(MarkedStdout::default());
        let (requests, mut pending) = mpsc::unbounded_channel();
        let options = InterpreterOptions {
            stdout: Rc::clone(&stdout) as Rc<dyn Stdout>,
            stdin: Rc::new(SteppedStdin { requests }),
            ..InterpreterOptions::new(Rc::new(scheduler::TokioScheduler::new()))
        };
        let path = format!("/home/{user}/{PROGRAM_NAME}");
        let observed = RefCell::new(None);
        let driver = async {
            while let Some(reply) = pending.recv().await {
                let traces = server.change_world(async |world| world_traces(world)).await;

                observed.replace(Some(traces));

                let _ = reply.send(String::new());
            }
        };
        let (_, ()) = tokio::join!(
            run_in_at(&server, script, user, true, &path, options),
            driver
        );

        observed
            .take()
            .expect("the script waited for input in the copy")
    })
}

/// What the game has of the player after the game ended: whether their computer exists, and their payments as
/// (kind, amount, listed networks) of the account `number`.
#[derive(Debug, PartialEq)]
pub struct Ended {
    pub computer_exists: bool,
    pub payments: Vec<(&'static str, i64, Vec<String>)>,
    pub network_disabled: bool,
    pub network_home: bool,
    pub rentals: Vec<(String, bool)>,
}

/// [`Ended`] as the game reports it for the account `number`.
pub fn ended_game(reply: &GameEndedReply, number: &str, network_address: &str) -> Ended {
    Ended {
        computer_exists: reply.computer_exists,
        payments: reply
            .subscriptions
            .iter()
            .filter(|payment| payment.bank == number)
            .map(|payment| {
                (
                    match payment.kind.as_ref() {
                        "BANK" => "Bank",
                        "RENTED" => "Rented",
                        "ADMIN" => "Admin",
                        other => panic!("the game's payment kind {other}"),
                    },
                    payment.amount,
                    payment.public_ips.iter().map(ToString::to_string).collect(),
                )
            })
            .collect(),
        network_disabled: reply.network_disabled,
        network_home: reply.network_home,
        rentals: reply
            .rentals
            .iter()
            .filter(|rental| rental.public_ip == network_address)
            .map(|rental| (rental.public_ip.clone(), rental.home))
            .collect(),
    }
}

/// [`Ended`] as the world has it for the account `number`, whose computer was `computer`.
pub fn ended_world(
    world: &World,
    computer: greyhack_world_core::ComputerId,
    number: &str,
    network_address: &str,
) -> Ended {
    let computer_row = block_on(world.modules.core.networks.topology.computer(computer))
        .expect("the computer is read");
    let rental_home = computer_row
        .as_ref()
        .map(|computer| computer.rental_home_ip.as_str())
        .unwrap_or_default();
    let payments: Vec<_> = block_on(world.modules.economy.bank.bank_subscriptions())
        .expect("the payments are read")
        .into_iter()
        .filter(|payment| payment.bank == number)
        .collect();
    let rentals = payments
        .iter()
        .filter(|payment| payment.kind == greyhack_behaviour::SubscriptionKind::Rented)
        .flat_map(|payment| {
            payment
                .public_ips
                .iter()
                .map(|address| (address.clone(), address == rental_home))
        })
        .collect();
    let router = block_on(
        world
            .modules
            .core
            .networks
            .routers
            .router_at(network_address),
    )
    .expect("the network is read");
    let network_disabled = router.as_ref().is_some_and(|router| {
        block_on(
            world
                .modules
                .core
                .networks
                .settings
                .is_disabled(router.id.computer()),
        )
        .expect("the disabled state is read")
    });
    let network_home = router.as_ref().is_some_and(|router| {
        block_on(
            world
                .modules
                .core
                .networks
                .settings
                .is_home_network(router.id.computer()),
        )
        .expect("the home state is read")
    });
    let payments = payments
        .into_iter()
        .map(|payment| {
            (
                match payment.kind {
                    greyhack_behaviour::SubscriptionKind::Bank => "Bank",
                    greyhack_behaviour::SubscriptionKind::Rented => "Rented",
                    greyhack_behaviour::SubscriptionKind::Admin => "Admin",
                },
                payment.amount,
                payment.public_ips,
            )
        })
        .collect();

    Ended {
        computer_exists: computer_row.is_some(),
        payments,
        network_disabled,
        network_home,
        rentals,
    }
}

/// The player has a guild that owns shares of the company they are on, and rents that network; strike `strikes` is
/// given for the company in both worlds, which are compared by what they have of the player afterwards.
pub async fn strike(strikes: u32) {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let home = client.network_state().await.unwrap().public_ip;
    let suffix = std::process::id() % 10000;
    let bank = client
        .setup_blockchain(&format!("Gu{suffix}"), &format!("u{suffix}"))
        .await
        .unwrap()
        .bank_number
        .clone();
    // A newly generated, listed company: only listed kinds issue shares and persist their return
    // (`StocksCompany.IsListedOnStockMarket`, `StocksCompany.SyncMarketData`), and the game copies keep the shares an
    // earlier run's guild bought.
    let network = empty_network(&client, "Supermercados").await;
    let snapshot = join_fixture(&client, &network).await;
    let address = snapshot.public_ip.clone();

    client
        .setup_guild(GuildSetup {
            name: format!("Owls{suffix}"),
            bank_account: bank.clone(),
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            join: true,
            company_ip: address.clone(),
            crown: false,
            shield_ip: String::new(),
            admin_restore_perks: 0,
            admin_restore_off: false,
        })
        .await
        .unwrap();
    client
        .rent_network(&address, RENTAL_AMOUNT, true)
        .await
        .unwrap();

    let guilds = client
        .query_guilds(std::slice::from_ref(&address))
        .await
        .expect("the guilds are read");
    let before = client.network_state().await.expect("the network is read");
    let mirrored_home = before
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if computer.player => Some(computer),
            _ => None,
        })
        .expect("the player is in the snapshot")
        .rental_home_ip
        .clone();
    assert_eq!(
        mirrored_home, address,
        "the home rental is visible in the snapshot before mirroring"
    );
    assert!(before.home, "the router's home marker is in the snapshot");
    let world = mirrored_world(&user, &before, &guilds);
    block_on(
        world
            .modules
            .core
            .networks
            .settings
            .set_home_network(&address, before.home),
    )
    .expect("the snapshot's home marker is mirrored");

    assert!(
        game_holders(&guilds)
            .iter()
            .any(|(account, _)| *account == bank),
        "the guild's shares are in the game before the strike: {:?}",
        game_holders(&guilds)
    );

    let player = world.handle.player();
    let router = block_on(world.modules.core.networks.routers.router_at(&address))
        .expect("the router is read")
        .expect("the company's router")
        .id
        .computer();

    block_on(
        world
            .modules
            .traces
            .traces
            .set_strikes(u8::try_from(strikes - 1).unwrap()),
    )
    .expect("the strikes are set");
    block_on(
        world
            .modules
            .traces
            .traces
            .strike_player(router, TraceKind::Active),
    )
    .expect("the strike is given");
    // The strike is an event the player domain ends the game on; the world server delivers events at the end of each
    // cycle, so the test does the same.
    block_on(world.registry.events().dispatch_async()).expect("the strike is dispatched");
    if strikes == 3 {
        // Leave the rented company before game over. RemovePayments disables rentals, then GameOver removes the player
        // from the network they currently occupy; keeping those as different networks avoids the same-network reset.
        join_network(&client, &home).await;
    }

    let ended = client
        .end_game(strikes, &address)
        .await
        .expect("the game gives the strike");
    let after = client
        .query_guilds(std::slice::from_ref(&address))
        .await
        .expect("the guilds are read");

    let observed = ended_game(&ended, &bank, &address);
    assert_eq!(ended_world(&world, player, &bank, &address), observed);
    assert_eq!(world_guild_exp(&world), game_guild_exp(&after));
    if strikes == 3 {
        assert!(
            ended.rentals.iter().all(|rental| !rental.home),
            "game over clears every rental's home flag"
        );
    }
    assert_eq!(world_holders(&world, router), game_holders(&after));
    assert_eq!(ended.computer_exists, strikes < 3);
    assert_eq!(ended.network_disabled, strikes == 3);
    assert_eq!(ended.network_home, strikes < 3);
    assert!(
        observed
            .payments
            .iter()
            .any(|(kind, _, ips)| { *kind == "Rented" && ips.contains(&address) }),
        "the RENTED payment contains the fixture network"
    );
    if strikes == 3 {
        assert!(
            observed.payments.iter().all(|(kind, _, _)| *kind != "Bank"),
            "RemovePayments stops and removes BANK"
        );
    }
    assert_eq!(
        block_on(world.modules.traces.traces.strikes()).unwrap(),
        if strikes < 3 {
            u8::try_from(strikes).expect("strike counts below three fit u8")
        } else {
            0
        }
    );

    client.shutdown();
}

fn world_guild_exp(world: &greyhack_world::World) -> Vec<(String, u32, bool)> {
    block_on(world.modules.guilds.guilds.guilds())
        .expect("the guilds are read")
        .into_iter()
        .map(|guild| (guild.name, guild.total_exp, guild.player_member))
        .collect()
}

fn game_guild_exp(
    reply: &greyhack_test_support_client::protocol::test_support::GuildsReply,
) -> Vec<(String, u32, bool)> {
    reply
        .guilds
        .iter()
        .map(|guild| (guild.name.clone(), guild.total_exp, guild.player_member))
        .collect()
}

/// The shareholders of the company `router` is the router of, with their shares.
fn world_holders(world: &greyhack_world::World, router: ComputerId) -> Vec<(String, u32)> {
    let network = block_on(world.modules.core.networks.topology.computer(router))
        .expect("the router is read")
        .expect("the company's router")
        .network;

    block_on(world.modules.guilds.guilds.company_stakes(network))
        .expect("the shares are read")
        .into_iter()
        .map(|stake| (stake.bank_account, stake.shares))
        .collect()
}

fn game_holders(
    reply: &greyhack_test_support_client::protocol::test_support::GuildsReply,
) -> Vec<(String, u32)> {
    reply
        .companies
        .iter()
        .flat_map(|company| company.holders.iter())
        .map(|holder| (holder.bank_account.clone(), holder.shares))
        .collect()
}
