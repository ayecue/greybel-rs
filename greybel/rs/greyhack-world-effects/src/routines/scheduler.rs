//! The queue of due routines and the registry of kinds that run them.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;

use greyhack_world_storage::TableSet;
use greyhack_world_storage::sql::{
    FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value, insert,
};
use turso::{Connection, params};

use super::kind::{Adapter, DynRoutine};
use crate::{Result, Routine, RoutineId};

/// The table set of the queue: routine ID, due second, and stable string kind key.
pub const QUEUE_TABLES: TableSet = TableSet::new(
    "-- scheduled work; each kind's data is in its own table, whose row a deleted computer, NPC or coin removes: the runner
-- then deletes the routine
CREATE TABLE IF NOT EXISTS routines (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    at   INTEGER NOT NULL,
    kind TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS routines_due ON routines (at, id);

",
    &["routines"],
);

/// A queued routine with its typed data.
#[derive(Debug)]
pub struct Entry {
    /// Queue row ID.
    pub id: RoutineId,
    /// World second the routine is due.
    pub at: u64,
    /// Stable key of the routine kind.
    pub key: &'static str,
    data: Box<dyn Any>,
}

impl Entry {
    /// Returns the routine's data if it is a `D`.
    #[must_use]
    pub fn data<D: Any>(&self) -> Option<&D> {
        self.data.downcast_ref()
    }

    /// Returns the routine's data if it is a `D`.
    #[must_use]
    pub fn into_data<D: Any>(self) -> Option<D> {
        self.data.downcast().ok().map(|data| *data)
    }

    /// Returns the type-erased data for a read-only scheduled-routine view.
    #[must_use]
    pub fn into_any(self) -> Box<dyn Any> {
        self.data
    }
}

/// When routines are due and how fast waiting raises their priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// Current world second: routines due at or before it are due.
    pub now: u64,
    /// Seconds late that raise a routine's priority by one level. Zero counts as one.
    pub raise_after: u64,
}

/// Queue row without its kind data.
struct QueueRow {
    id: i64,
    at: i64,
    kind: String,
}

impl FromRow for QueueRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            at: row.get("at")?,
            kind: row.get("kind")?,
        })
    }
}

/// Registered routine instances and operations on their queue.
pub struct Scheduler {
    kinds: RefCell<HashMap<&'static str, Rc<dyn DynRoutine>>>,
    order: RefCell<Vec<&'static str>>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            kinds: RefCell::new(HashMap::new()),
            order: RefCell::new(Vec::new()),
        }
    }
}

impl std::fmt::Debug for Scheduler {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_list()
            .entries(self.order.borrow().iter())
            .finish()
    }
}

impl Scheduler {
    /// Creates a scheduler with no kinds.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers routine instance `kind`.
    ///
    /// # Panics
    ///
    /// Panics if its key is already registered.
    pub fn register<R: Routine>(&self, kind: Rc<R>) -> &Self {
        assert!(
            !self.kinds.borrow().contains_key(R::KEY),
            "routine kind key {} is registered twice",
            R::KEY
        );
        self.kinds
            .borrow_mut()
            .insert(R::KEY, Rc::new(Adapter(kind)));
        self.order.borrow_mut().push(R::KEY);

        self
    }

    /// Returns this scheduler with routine instance `kind` registered.
    #[must_use]
    pub fn with<R: Routine>(self, kind: Rc<R>) -> Self {
        self.register(kind);

        self
    }

    /// Returns the table sets of the registered kinds in registration order.
    #[must_use]
    pub fn kind_tables(&self) -> Vec<TableSet> {
        self.order
            .borrow()
            .iter()
            .map(|key| self.kinds.borrow()[key].tables())
            .collect()
    }

    fn kind(&self, key: &str) -> Option<Rc<dyn DynRoutine>> {
        self.kinds.borrow().get(key).cloned()
    }

    /// Queues a routine of kind `R` with `data` for world second `at`.
    ///
    /// # Errors
    ///
    /// Returns a database or routine-storage error.
    ///
    /// # Panics
    ///
    /// Panics if `R` is not registered.
    pub async fn schedule<R: Routine>(
        &self,
        conn: &Connection,
        at: u64,
        data: &R::Data,
    ) -> Result<RoutineId> {
        let kind = self
            .kind(R::KEY)
            .unwrap_or_else(|| panic!("routine kind key {} is not registered", R::KEY));
        let id = insert(
            conn,
            "INSERT INTO routines (at, kind) VALUES (?1, ?2)",
            params![at.cast_signed(), R::KEY],
        )
        .await?
        .cast_unsigned();
        let id = RoutineId::new(id);

        kind.store(conn, id, data).await?;

        Ok(id)
    }

    /// Removes queue row `id`, calls its removal hook, and returns whether it existed.
    pub async fn cancel(&self, conn: &Connection, id: RoutineId) -> Result<bool> {
        let kind = fetch_value::<String>(
            conn,
            "SELECT kind FROM routines WHERE id = ?1",
            params![id.get().cast_signed()],
        )
        .await?;
        let Some(kind) = kind else {
            return Ok(false);
        };
        let removed = execute(
            conn,
            "DELETE FROM routines WHERE id = ?1",
            params![id.get().cast_signed()],
        )
        .await?
            > 0;

        if removed && let Some(kind) = self.kind(&kind) {
            kind.on_remove(id).await?;
        }

        Ok(removed)
    }

    /// Lets every registered kind cancel routines referring to `computers`, in registration order.
    pub async fn cancel_computers(&self, conn: &Connection, computers: &[u32]) -> Result<()> {
        let order = self.order.borrow().clone();

        for key in order {
            let kind = self.kind(key).expect("ordered kind exists");

            kind.cancel_for_computers(conn, computers).await?;
        }

        Ok(())
    }

    /// Returns the earliest due second of the queue, if any.
    pub async fn next_due_at(&self, conn: &Connection) -> Result<Option<u64>> {
        Ok(
            fetch_value::<i64>(conn, "SELECT at FROM routines ORDER BY at LIMIT 1", ())
                .await?
                .map(i64::cast_unsigned),
        )
    }

    /// Returns every queued routine with data in due-second then ID order.
    pub async fn entries(&self, conn: &Connection) -> Result<Vec<Entry>> {
        let rows: Vec<QueueRow> = fetch_all(
            conn,
            "SELECT id, at, kind FROM routines ORDER BY at, id",
            (),
        )
        .await?;
        let mut entries = Vec::new();

        for row in rows {
            let Some(kind) = self.kind(&row.kind) else {
                continue;
            };

            if let Some(data) = kind
                .load(conn, RoutineId::new(row.id.cast_unsigned()))
                .await?
            {
                entries.push(Entry {
                    id: RoutineId::new(row.id.cast_unsigned()),
                    at: row.at.cast_unsigned(),
                    key: kind.key(),
                    data,
                });
            }
        }

        Ok(entries)
    }

    /// Returns the next due routine, dropping rows of unknown kinds or whose data was cascaded away.
    pub async fn next_due(&self, conn: &Connection, window: Window) -> Result<Option<Entry>> {
        loop {
            let priority = self.priority_sql();
            let sql = format!(
                "SELECT id, at, kind FROM routines WHERE at <= ?1 \
                 ORDER BY MAX({priority} - (?1 - at) / ?2, 0), at, id LIMIT 1"
            );
            let row: Option<QueueRow> = fetch_opt(
                conn,
                &sql,
                params![
                    window.now.cast_signed(),
                    window.raise_after.max(1).cast_signed()
                ],
            )
            .await?;
            let Some(row) = row else {
                return Ok(None);
            };
            let id = RoutineId::new(row.id.cast_unsigned());
            let data = match self.kind(&row.kind) {
                Some(kind) => kind.load(conn, id).await?,
                None => None,
            };

            match data {
                Some(data) => {
                    return Ok(Some(Entry {
                        id,
                        at: row.at.cast_unsigned(),
                        key: self.kind(&row.kind).expect("loaded kind exists").key(),
                        data,
                    }));
                }
                None => {
                    self.cancel(conn, id).await?;
                }
            }
        }
    }

    /// Removes and runs the next due routine, returning its entry.
    pub async fn run_next_due(&self, conn: &Connection, window: Window) -> Result<Option<Entry>> {
        let Some(entry) = self.next_due(conn, window).await? else {
            return Ok(None);
        };
        let kind = self.kind(entry.key).expect("selected kind exists");

        execute(
            conn,
            "DELETE FROM routines WHERE id = ?1",
            params![entry.id.get().cast_signed()],
        )
        .await?;
        kind.run(entry.id, entry.data).await?;

        // Rebuild the public entry after execution only when callers need completion metadata.
        Ok(Some(Entry {
            id: entry.id,
            at: entry.at,
            key: entry.key,
            data: Box::new(()),
        }))
    }

    /// Runs up to `budget` due routines, calling `after` after each one.
    pub async fn run_due(
        &self,
        conn: &Connection,
        window: Window,
        budget: usize,
        mut after: impl FnMut(Entry),
    ) -> Result<usize> {
        let mut ran = 0;

        while ran < budget {
            let Some(entry) = self.run_next_due(conn, window).await? else {
                break;
            };

            after(entry);
            ran += 1;
        }

        Ok(ran)
    }

    fn priority_sql(&self) -> String {
        let mut sql = "CASE kind".to_owned();

        for key in self.order.borrow().iter() {
            let kind = self.kind(key).expect("ordered kind exists");

            let _ = write!(
                sql,
                " WHEN '{}' THEN {}",
                key.replace('\'', "''"),
                kind.priority()
            );
        }

        sql.push_str(" ELSE 0 END");

        sql
    }
}
