use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use greyhack_behaviour::Gender;
use greyhack_world_storage::Schema;

use crate::hooks::{
    HookFuture, LibraryVersions, NetworkAdminHook, NetworkGeneration, NpcHook, NpcLookup, NpcState,
    PersistentState, PlayerTraces,
};
use crate::storage::schema::{CORE_TABLES, WORLD_META_TABLES};
use crate::{Database, Person, PersonId, RouterId, WorldRef};

/// Hooks of a world without higher domains: nothing is generated, no NPC exists, and the calls
/// to `router_or_generate` are counted.
#[derive(Debug, Default)]
pub struct StubHooks {
    /// How often `router_or_generate` was called.
    pub generated: AtomicUsize,
}

impl NetworkGeneration for StubHooks {
    fn router_or_generate<'a>(
        &'a self,
        _world: &'a WorldRef,
        _public_ip: &'a str,
    ) -> HookFuture<'a, Option<RouterId>> {
        self.generated.fetch_add(1, Ordering::Relaxed);

        Box::pin(async { Ok(None) })
    }
}

impl NpcHook for StubHooks {
    fn npc_state<'a>(
        &'a self,
        _world: &'a WorldRef,
        _lookup: NpcLookup<'a>,
    ) -> HookFuture<'a, Option<NpcState>> {
        Box::pin(async { Ok(None) })
    }
}

impl PlayerTraces for StubHooks {
    fn player_token_trace<'a>(&'a self, _world: &'a WorldRef) -> HookFuture<'a, String> {
        Box::pin(async { Ok(String::new()) })
    }
}

impl LibraryVersions for StubHooks {}
impl NetworkAdminHook for StubHooks {}
impl PersistentState for StubHooks {}

pub fn bind(registry: &Rc<greyhack_world_modules::Registry>, hooks: &Rc<StubHooks>) {
    registry.register_service::<dyn NetworkGeneration>(hooks.clone());
    registry.register_service::<dyn NpcHook>(hooks.clone());
    registry.register_service::<dyn PlayerTraces>(hooks.clone());
    registry.register_service::<dyn LibraryVersions>(hooks.clone());
    registry.register_service::<dyn NetworkAdminHook>(hooks.clone());
    registry.register_capability::<dyn PersistentState>(hooks.clone());
}

pub async fn database() -> Database {
    let schema = CORE_TABLES
        .into_iter()
        .fold(Schema::new(), |schema, set| schema.with(set))
        .with(WORLD_META_TABLES);

    Database::in_memory_with(&schema).await.unwrap()
}

pub async fn player(database: &Database) -> PersonId {
    crate::storage::PersonRepository::new(database.clone())
        .insert_person(&Person {
            id: PersonId(0),
            name: "Ada".to_owned(),
            surname: "Lovelace".to_owned(),
            gender: Gender::Female,
        })
        .await
        .unwrap()
}
