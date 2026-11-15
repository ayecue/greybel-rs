//! In-memory world state: random generator, clock, player, events, and transient work.

mod events;
mod persistent_state;
mod reference;

pub use events::{ClockAdvance, ClockAdvanceEnd, RebootEvent};
use greyhack_behaviour::{Rng, WORLD_SECRET_LENGTH, WorldSeed};
use greyhack_world_effects::{EventQueue, WorkQueue};
use greyhack_world_programs::StateMap;
pub use persistent_state::PersistentState;
pub use reference::WorldRef;

use crate::{ComputerId, PersonId};

/// Shared world state. Entities live in database rows and services reach this state through
/// [`WorldRef`], never by carrying a borrow across an await or another service call.
#[derive(Debug)]
pub struct World {
    /// Seed the world was generated from.
    pub seed: u64,
    /// The world's random generator, saved with `world_meta`.
    pub rng: Rng,
    /// The player computer, [`ComputerId`] 0 after a game over.
    pub player: ComputerId,
    /// The player's person.
    pub player_person: PersonId,
    /// The world clock in seconds.
    pub time_seconds: u64,
    /// Fractional world time accumulated by clock ticks but not yet counted, in `1 / rate`
    /// tick-second units. Always below the tick rate.
    pub clock_carry: u32,
    /// World secret (`WorldSeed`) derived from its seed, or copied from Grey Hack for mirrored
    /// worlds.
    pub world_seed: WorldSeed,
    /// Ordered domain events awaiting dispatch.
    pub events: std::rc::Rc<EventQueue>,
    /// Typed work completions published into `events`.
    pub works: std::rc::Rc<WorkQueue>,
    /// Computers with processes ended since [`World::take_changed_computers`].
    pub process_ends: std::collections::BTreeSet<ComputerId>,
    /// In-memory state owned by the domains above core.
    states: StateMap,
}

impl World {
    /// Creates the shared state of a world generated from `seed`.
    #[must_use]
    pub fn new(seed: u64, player: ComputerId, time_seconds: u64) -> Self {
        let events = std::rc::Rc::new(EventQueue::default());
        let works = std::rc::Rc::new(WorkQueue::new(events.clone()));

        Self {
            seed,
            rng: Rng::new(seed),
            player,
            player_person: PersonId(0),
            time_seconds,
            clock_carry: 0,
            world_seed: WorldSeed::new(secret_of(seed)),
            events,
            works,
            process_ends: std::collections::BTreeSet::new(),
            states: StateMap::default(),
        }
    }

    /// Attaches the queues created by module assembly.
    pub fn attach_queues(
        &mut self,
        events: std::rc::Rc<EventQueue>,
        works: std::rc::Rc<WorkQueue>,
    ) {
        self.events = events;
        self.works = works;
    }

    /// Returns computers whose processes were killed, rebooted, or removed since the last call.
    pub fn take_changed_computers(&mut self) -> Vec<ComputerId> {
        std::mem::take(&mut self.process_ends).into_iter().collect()
    }

    /// Returns domain state `T`, creating its default value on first access.
    pub fn state<T: Default + Send + 'static>(&mut self) -> &T {
        self.states.state::<T>()
    }

    /// Returns mutable domain state `T`, creating its default value on first access.
    pub fn state_mut<T: Default + Send + 'static>(&mut self) -> &mut T {
        self.states.state_mut::<T>()
    }

    /// Returns domain state `T` when it has been created.
    #[must_use]
    pub fn state_ref<T: Send + 'static>(&self) -> Option<&T> {
        self.states.get::<T>()
    }

    /// Returns whether the player has a computer: not after a game over.
    #[must_use]
    pub fn has_player(&self) -> bool {
        self.player != ComputerId(0)
    }

    /// Returns whether `computer` belongs to the player.
    #[must_use]
    pub fn is_player(&self, computer: ComputerId) -> bool {
        self.player == computer
    }

    /// Returns the world secret (`WorldSeed`).
    #[must_use]
    pub fn world_seed(&self) -> &WorldSeed {
        &self.world_seed
    }

    /// Replaces the world secret from mirrored Grey Hack state (`InfoGen.WorldSecret`). Existing
    /// license plates retain their original secret.
    pub fn set_world_secret(&mut self, secret: Vec<u8>) {
        self.world_seed = WorldSeed::new(secret);
    }
}

/// Derives [`WORLD_SECRET_LENGTH`] secret bytes from `seed`, matching Grey Hack's draw count.
fn secret_of(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed ^ SECRET_SALT);

    (0..WORLD_SECRET_LENGTH)
        .map(|_| u8::try_from(rng.range(0, 255)).expect("value fits the target integer type"))
        .collect()
}

/// Separates secret generation from the world random sequence.
const SECRET_SALT: u64 = 0x5745_524c_4453_4543;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
