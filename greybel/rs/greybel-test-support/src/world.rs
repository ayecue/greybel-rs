//! Helpers for constructing, persisting, mutating, and inspecting test worlds.

use greyhack_world::{DatabaseOpen, PlayerOptions, World, WorldOptions};
use greyhack_world_core::{CoinId, Computer, ComputerId, Database, Router, RouterId};
use greyhack_world_economy::SubWallet;
use greyhack_world_effects::WorkEnvelope;
use greyhack_world_libraries::Library;
use greyhack_world_traces::StrikeEvent;

/// Creates `WorldOptions` with the requested seed, network count, and default player.
#[must_use]
pub fn world_options(seed: u64, networks: u32) -> WorldOptions {
    WorldOptions {
        seed,
        networks,
        player: PlayerOptions::default(),
    }
}

/// Returns `world`'s row for computer `id`.
///
/// # Panics
///
/// Panics on world read failure or a missing computer.
pub async fn computer(world: &World, id: ComputerId) -> Computer {
    world
        .modules
        .core
        .networks
        .topology
        .computer(id)
        .await
        .expect("the world reads")
        .expect("the computer")
}

/// Returns `world`'s row for router `id`.
///
/// # Panics
///
/// Panics on world read failure or a missing router.
pub async fn router(world: &World, id: RouterId) -> Router {
    world
        .modules
        .core
        .networks
        .routers
        .router(id)
        .await
        .expect("the world reads")
        .expect("the router")
}

/// Returns all `world` routers ordered by public address.
pub async fn routers(world: &World) -> Vec<Router> {
    let mut routers = Vec::new();

    for address in world
        .modules
        .core
        .networks
        .routers
        .public_addresses()
        .await
        .expect("the world reads")
    {
        if let Some(router) = world
            .modules
            .core
            .networks
            .routers
            .router_at(&address)
            .await
            .expect("the world reads")
        {
            routers.push(router);
        }
    }

    routers
}

/// Returns all `world` computers grouped by router network.
pub async fn computers(world: &World) -> Vec<Computer> {
    let mut computers = Vec::new();

    for router in routers(world).await {
        for id in world
            .modules
            .core
            .networks
            .topology
            .devices(router.id)
            .await
            .expect("the world reads")
        {
            if let Some(computer) = world
                .modules
                .core
                .networks
                .topology
                .computer(id)
                .await
                .expect("the world reads")
            {
                computers.push(computer);
            }
        }
    }

    computers
}

/// Applies `change` to computer `id` and persists its row.
///
/// # Panics
///
/// Panics on world read/write failure or a missing computer.
pub async fn change_computer<T>(
    world: &mut World,
    id: ComputerId,
    change: impl FnOnce(&mut Computer) -> T,
) -> T {
    let mut computer = computer(world, id).await;
    let result = change(&mut computer);

    world
        .modules
        .core
        .networks
        .topology
        .save_computer(&computer)
        .await
        .expect("the world writes");

    result
}

/// Applies `change` to router `id` and persists its row.
///
/// # Panics
///
/// Panics on world read/write failure or a missing router.
pub async fn change_router<T>(
    world: &mut World,
    id: RouterId,
    change: impl FnOnce(&mut Router) -> T,
) -> T {
    let mut router = router(world, id).await;
    let result = change(&mut router);

    world
        .modules
        .core
        .networks
        .routers
        .save_router(&router)
        .await
        .expect("the world writes");

    result
}

/// Checks for computer `id` in `world`; world read failures panic.
pub async fn has_computer(world: &World, id: ComputerId) -> bool {
    world
        .modules
        .core
        .networks
        .topology
        .computer(id)
        .await
        .expect("the world reads")
        .is_some()
}

/// Clones library file `path` on `computer`; read failures and missing data panic.
pub async fn library(world: &World, computer: ComputerId, path: &str) -> Library {
    world
        .modules
        .libraries
        .generator
        .full_file_library(computer, path, true)
        .await
        .expect("the world reads")
        .expect("a library")
}

/// Copies `world` into and returns a new in-memory database.
pub async fn save_copy(world: &mut World) -> Database {
    let database = Database::in_memory().await.expect("an in-memory database");

    world.save_into(&database).await.expect("the world saves");

    database
}

/// Creates a passwordless `coin` subwallet for `user` with `balance`.
#[must_use]
pub fn subwallet(coin: CoinId, user: &str, balance: f64) -> SubWallet {
    SubWallet {
        coin,
        user: user.to_owned(),
        password_hash: String::new(),
        balance,
        info: String::new(),
        wallet_user: user.to_owned(),
    }
}

/// Comparable persistent-state fingerprint containing `world`'s date and each router's devices. Verifies reloaded
/// worlds against live state without the removed `WorldData` snapshot.
pub async fn persistent_fingerprint(world: &World) -> (String, Vec<(String, Vec<ComputerId>)>) {
    let mut routers = Vec::new();

    for address in world
        .modules
        .core
        .networks
        .routers
        .public_addresses()
        .await
        .expect("the world reads")
    {
        let router = world
            .modules
            .core
            .networks
            .routers
            .router_at(&address)
            .await
            .expect("the world reads")
            .expect("the router");
        let devices = world
            .modules
            .core
            .networks
            .topology
            .devices(router.id)
            .await
            .expect("the world reads");

        routers.push((address, devices));
    }

    (world.date(), routers)
}

thread_local! {
    static WORK_LOGS: std::cell::RefCell<std::collections::HashMap<usize, std::rc::Rc<std::cell::RefCell<Vec<WorkEnvelope>>>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Delivers queued events and returns their typed work completions.
pub fn work_ends(world: &World) -> Vec<WorkEnvelope> {
    let events = &world.assembly.events;
    let key = std::rc::Rc::as_ptr(events) as usize;
    let ends = WORK_LOGS.with(|logs| {
        logs.borrow_mut()
            .entry(key)
            .or_insert_with(|| {
                let ends = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

                events.on({
                    let ends = ends.clone();

                    move |event: &greyhack_world_effects::WorkCompletion| {
                        if let Some(end) = event.take_envelope() {
                            ends.borrow_mut().push(end);
                        }
                    }
                });

                ends.clone()
            })
            .clone()
    });

    while !events.is_empty() {
        crate::future::block_on(events.dispatch_async()).expect("domain events dispatch");
    }

    ends.borrow_mut().drain(..).collect()
}

/// Delivers queued events, including asynchronous domain subscribers, and returns strikes.
pub async fn strikes_async(world: &World) -> Vec<StrikeEvent> {
    let strikes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

    world.assembly.events.on({
        let strikes = strikes.clone();

        move |event: &StrikeEvent| strikes.borrow_mut().push(*event)
    });
    world
        .assembly
        .events
        .dispatch_async()
        .await
        .expect("domain events deliver");

    strikes.borrow_mut().drain(..).collect()
}

/// Delivers queued events and returns strikes in publish order.
pub fn strikes(world: &World) -> Vec<StrikeEvent> {
    let strikes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

    world.assembly.events.on({
        let strikes = strikes.clone();

        move |event: &StrikeEvent| strikes.borrow_mut().push(*event)
    });
    world.dispatch_events();

    strikes.borrow_mut().drain(..).collect()
}
