use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use greyhack_behaviour::Gender;
use greyhack_world_effects::BoxFuture;
use greyhack_world_storage::Schema;

use crate::storage::schema::{CORE_TABLES, WORLD_META_TABLES};
use crate::{
    Database, LibraryVersionService, NetworkAdminLookup, NetworkGeneration, NpcLookup, NpcState,
    NpcStatus, PersistentState, Person, PersonId, PlayerTraceService, Result, RouterId, WorldRef,
};

/// Services of a world without higher domains: nothing is generated, no NPC exists, and the calls
/// to `router_or_generate` are counted.
#[derive(Debug, Default)]
pub struct TestServices {
    /// How often `router_or_generate` was called.
    pub generated: AtomicUsize,
}

impl NetworkGeneration for TestServices {
    fn router_or_generate<'a>(
        &'a self,
        _world: &'a WorldRef,
        _public_ip: &'a str,
    ) -> BoxFuture<'a, Result<Option<RouterId>>> {
        self.generated.fetch_add(1, Ordering::Relaxed);

        Box::pin(async { Ok(None) })
    }
}

impl NpcState for TestServices {
    fn npc_state<'a>(
        &'a self,
        _world: &'a WorldRef,
        _lookup: NpcLookup<'a>,
    ) -> BoxFuture<'a, Result<Option<NpcStatus>>> {
        Box::pin(async { Ok(None) })
    }
}

impl PlayerTraceService for TestServices {
    fn player_token_trace<'a>(&'a self, _world: &'a WorldRef) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }
}

impl LibraryVersionService for TestServices {}
impl NetworkAdminLookup for TestServices {}
impl PersistentState for TestServices {}

pub fn bind(registry: &Rc<greyhack_world_modules::Registry>, services: &Rc<TestServices>) {
    registry.register_service::<dyn NetworkGeneration>(services.clone());
    registry.register_service::<dyn NpcState>(services.clone());
    registry.register_service::<dyn PlayerTraceService>(services.clone());
    registry.register_service::<dyn LibraryVersionService>(services.clone());
    registry.register_service::<dyn NetworkAdminLookup>(services.clone());
    registry.register_capability::<dyn PersistentState>(services.clone());
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
