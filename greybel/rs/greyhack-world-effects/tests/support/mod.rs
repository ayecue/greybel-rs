//! Shared helpers for world-effects integration tests.

pub mod effects;
pub mod scheduler;

use std::cell::RefCell;
use std::rc::Rc;

use greyhack_world_effects::{
    Result as RoutineResult, Routine, RoutineError, RoutineId, Scheduler,
};
use greyhack_world_storage::sql::{execute, fetch_value};
use greyhack_world_storage::{Database, Schema, TableSet};
use turso::{Connection, params};

#[derive(Default)]
pub struct State {
    pub log: Vec<String>,
    pub output: Vec<String>,
}

/// Database and observable state shared with test routine instances.
#[derive(Clone)]
pub struct Host {
    pub _database: Database,
    pub conn: Connection,
    pub state: Rc<RefCell<State>>,
}

impl Host {
    pub fn log(&self) -> Vec<String> {
        self.state.borrow().log.clone()
    }
}

/// Greets with a text; the lowest priority value, so it runs first.
pub struct Greeting {
    state: Rc<RefCell<State>>,
    conn: Connection,
}

impl Greeting {
    pub fn new(host: &Host) -> Self {
        Self {
            state: host.state.clone(),
            conn: host.conn.clone(),
        }
    }
}

impl Routine for Greeting {
    type Data = String;

    const KEY: &'static str = "test.greeting";
    const PRIORITY: i64 = 0;

    fn tables() -> TableSet {
        TableSet::new(
            "CREATE TABLE IF NOT EXISTS routine_greetings (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    text       TEXT NOT NULL
);
",
            &["routine_greetings"],
        )
    }

    async fn store(&self, conn: &Connection, id: RoutineId, data: &String) -> RoutineResult<()> {
        execute(
            conn,
            "INSERT INTO routine_greetings (routine_id, text) VALUES (?1, ?2)",
            params![id.get().cast_signed(), data.as_str()],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<String>> {
        Ok(fetch_value(
            conn,
            "SELECT text FROM routine_greetings WHERE routine_id = ?1",
            params![id.get().cast_signed()],
        )
        .await?)
    }

    async fn run(&self, _id: RoutineId, data: String) -> RoutineResult<()> {
        let queued = fetch_value::<i64>(&self.conn, "SELECT COUNT(*) FROM routines", ())
            .await?
            .unwrap_or_default();
        let mut state = self.state.borrow_mut();

        state.log.push(format!("greet {data} with {queued} queued"));
        state.output.push(data.clone());
        drop(state);
        if data == "fail" {
            return Err(RoutineError::new(Failure("the greeting failed".to_owned())));
        }

        Ok(())
    }
}

/// Counts with a higher priority value.
pub struct Counter {
    state: Rc<RefCell<State>>,
}

impl Routine for Counter {
    type Data = i64;

    const KEY: &'static str = "test.counter";
    const PRIORITY: i64 = 5;

    fn tables() -> TableSet {
        TableSet::new(
            "CREATE TABLE IF NOT EXISTS routine_counters (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    amount     INTEGER NOT NULL
);
",
            &["routine_counters"],
        )
    }

    async fn store(&self, conn: &Connection, id: RoutineId, data: &i64) -> RoutineResult<()> {
        execute(
            conn,
            "INSERT INTO routine_counters (routine_id, amount) VALUES (?1, ?2)",
            params![id.get().cast_signed(), *data],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<i64>> {
        Ok(fetch_value(
            conn,
            "SELECT amount FROM routine_counters WHERE routine_id = ?1",
            params![id.get().cast_signed()],
        )
        .await?)
    }

    async fn run(&self, _id: RoutineId, data: i64) -> RoutineResult<()> {
        let mut state = self.state.borrow_mut();

        state.log.push(format!("count {data}"));
        state.output.push(data.to_string());

        Ok(())
    }
}

pub type TestScheduler = Scheduler;

pub fn scheduler(host: &Host) -> TestScheduler {
    Scheduler::new()
        .with(Rc::new(Greeting::new(host)))
        .with(Rc::new(Counter {
            state: host.state.clone(),
        }))
}

pub fn schema() -> Schema {
    Schema::new()
        .with(QUEUE_TABLES)
        .with(Greeting::tables())
        .with(Counter::tables())
}

pub async fn host() -> Host {
    let database = Database::in_memory(&schema()).await.unwrap();
    let conn = database.connection().clone();

    Host {
        _database: database,
        conn: conn.clone(),
        state: Rc::new(RefCell::new(State::default())),
    }
}

#[derive(Debug)]
pub struct Failure(pub String);

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Failure {}

use greyhack_world_effects::QUEUE_TABLES;
