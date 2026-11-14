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
    GameEndedReply, GuildsReply, NetworkSnapshot,
};
use greyhack_test_support_client::{ExecutionStep, GuildSetup, TestClient};
use greyhack_world::prelude::*;
use greyhack_world::{ComputerId, Database, TraceKind, World};
use greyhack_world_operations::WorldServer;
use tokio::sync::mpsc;

use super::game::{join_new_network_of_kind_where, setup_player, test_client};
use super::intrinsics::PROGRAM_NAME;
use super::parity_bank_payments::RENTAL_AMOUNT;
use super::parity_guilds::mirrored_world;
use super::parity_traces::{LEVEL_FIVE, Traces, game_traces, world_traces};
use super::steps::SteppedStdin;

/// The traces of the game and of the copy of its network at the first input `script` waits for: the active traces
/// run for seconds of real time, so only a script that waits sees them.
pub async fn traces_while_waiting(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    guilds: &GuildsReply,
    script: &str,
) -> (Traces, Traces) {
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
) -> Traces {
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
}

/// [`Ended`] as the game reports it for the account `number`.
pub fn ended_game(reply: &GameEndedReply, number: &str) -> Ended {
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
    }
}

/// [`Ended`] as the world has it for the account `number`, whose computer was `computer`.
pub fn ended_world(world: &World, computer: greyhack_world::ComputerId, number: &str) -> Ended {
    Ended {
        computer_exists: block_on(world.modules.core.networks.topology.computer(computer))
            .expect("the computer is read")
            .is_some(),
        payments: block_on(world.modules.economy.bank.bank_subscriptions())
            .expect("the payments are read")
            .into_iter()
            .filter(|payment| payment.bank == number)
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
            .collect(),
    }
}

/// The player has a guild that owns shares of the company they are on, and rents that network; strike `strikes` is
/// given for the company in both worlds, which are compared by what they have of the player afterwards.
pub async fn strike(strikes: u32) {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let suffix = std::process::id() % 10000;
    let bank = client
        .setup_blockchain(&format!("Gu{suffix}"), &format!("u{suffix}"))
        .await
        .unwrap()
        .bank_number
        .to_string();
    // A newly generated, listed company: only listed kinds issue shares and persist their return
    // (`StocksCompany.IsListedOnStockMarket`, `StocksCompany.SyncMarketData`), and the game copies keep the shares an
    // earlier run's guild bought.
    let snapshot = join_new_network_of_kind_where(&client, &["Supermercados"], "ssh", |snapshot| {
        snapshot.kind == "Supermercados"
    })
    .await;
    let address = snapshot.public_ip.to_string();

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
    client.rent_network(&address, RENTAL_AMOUNT).await.unwrap();

    let before = client.network_state().await.expect("the network is read");
    let guilds = client
        .query_guilds(std::slice::from_ref(&address))
        .await
        .expect("the guilds are read");
    let world = mirrored_world(&user, &before, &guilds);

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
            .guilds
            .traces
            .set_strikes(u8::try_from(strikes - 1).unwrap()),
    )
    .expect("the strikes are set");
    block_on(
        world
            .modules
            .guilds
            .traces
            .strike_player(router, TraceKind::Active),
    )
    .expect("the strike is given");
    // The strike is an event the player domain ends the game on; the world server delivers events at the end of each
    // cycle, so the test does the same.
    block_on(world.registry.events().deliver_async()).expect("the strike is delivered");

    let ended = client
        .end_game(strikes, &address)
        .await
        .expect("the game gives the strike");
    let after = client
        .query_guilds(std::slice::from_ref(&address))
        .await
        .expect("the guilds are read");

    assert_eq!(
        ended_world(&world, player, &bank),
        ended_game(&ended, &bank)
    );
    assert_eq!(world_guild_exp(&world), game_guild_exp(&after));
    assert_eq!(world_holders(&world, router), game_holders(&after));
    assert_eq!(ended.computer_exists, strikes < 3);
    assert_eq!(
        block_on(world.modules.guilds.traces.strikes()).unwrap(),
        if strikes < 3 { strikes as u8 } else { 0 }
    );

    client.shutdown();
}

fn world_guild_exp(world: &greyhack_world::World) -> Vec<(String, u32)> {
    block_on(world.modules.guilds.guilds.guilds())
        .expect("the guilds are read")
        .into_iter()
        .map(|guild| (guild.name, guild.total_exp))
        .collect()
}

fn game_guild_exp(
    reply: &greyhack_test_support_client::protocol::test_support::GuildsReply,
) -> Vec<(String, u32)> {
    reply
        .guilds
        .iter()
        .map(|guild| (guild.name.to_string(), guild.total_exp))
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
        .map(|holder| (holder.bank_account.to_string(), holder.shares))
        .collect()
}
