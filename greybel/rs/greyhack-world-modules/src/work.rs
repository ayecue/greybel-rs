//! Typed work waited on by routes across world cycles.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;

use crate::{Event, EventError, EventQueue};

/// Identifier of one waiting unit of work.
pub type WorkId = u64;

/// Typed handle returned when work starts.
pub struct Work<T> {
    id: WorkId,
    result: PhantomData<fn() -> T>,
}

impl<T> Copy for Work<T> {}
impl<T> Clone for Work<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Work<T> {
    /// Returns the work identifier used by domain storage.
    #[must_use]
    pub const fn id(&self) -> WorkId {
        self.id
    }
}

/// The waiter was cancelled before a typed result was produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("work was cancelled")]
pub struct WorkCancel;

/// Future returned when a domain cancels its waiting work.
pub type WorkCancelFuture<'a> = Pin<Box<dyn Future<Output = Result<bool, EventError>> + 'a>>;

/// A domain that owns waiting work and removes it when its program ends.
pub trait WorkOwner {
    /// Cancels `id` when this domain owns it, returning whether a waiter was cancelled.
    fn cancel(&self, id: WorkId) -> WorkCancelFuture<'_>;

    /// Clears this domain's tree state when `id` ends or is cancelled.
    fn clear_tree(&self, _id: WorkId, _state: &mut greyhack_world_programs::StateMap) {}
}

/// Type-erased completion transferred from an event to its waiting route.
pub struct WorkEnvelope {
    /// Completed work identifier.
    pub id: WorkId,
    result: Result<Box<dyn Any>, WorkCancel>,
}

impl std::fmt::Debug for WorkEnvelope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkEnvelope")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl WorkEnvelope {
    /// Returns the erased result for a cycle-driver boundary.
    pub fn into_result(self) -> Result<Box<dyn Any>, WorkCancel> {
        self.result
    }

    /// Takes the typed result, or returns cancellation.
    pub fn take<T: 'static>(self) -> Result<T, WorkCancel> {
        match self.result {
            Ok(result) => Ok(*result
                .downcast::<T>()
                .expect("work result matches the type requested by its waiter")),
            Err(error) => Err(error),
        }
    }
}

/// Completion event carried by the ordered event queue.
pub struct WorkCompletion {
    /// Completed work identifier.
    pub id: WorkId,
    result: RefCell<Option<WorkOutcome>>,
}

impl WorkCompletion {
    /// Takes the completion once for transfer to its waiter.
    pub fn take_envelope(&self) -> Option<WorkEnvelope> {
        let result = match self.result.borrow_mut().take()? {
            WorkOutcome::Complete(result) => Ok(result),
            WorkOutcome::Cancel => Err(WorkCancel),
        };

        Some(WorkEnvelope {
            id: self.id,
            result,
        })
    }

    /// Takes the typed result once, returning cancellation when no result was produced.
    pub fn take<T: 'static>(&self) -> Option<Result<T, WorkCancel>> {
        self.take_envelope().map(WorkEnvelope::take)
    }
}

impl Event for WorkCompletion {}

enum WorkOutcome {
    Complete(Box<dyn Any>),
    Cancel,
}

/// Starts and ends typed work, publishing completions into the shared ordered event queue.
#[derive(Debug)]
pub struct WorkQueue {
    next: Cell<WorkId>,
    events: Rc<EventQueue>,
    computers: RefCell<HashMap<u32, Vec<WorkId>>>,
}

impl WorkQueue {
    /// Creates a work queue publishing into `events`.
    pub fn new(events: Rc<EventQueue>) -> Self {
        Self {
            next: Cell::new(0),
            events,
            computers: RefCell::new(HashMap::new()),
        }
    }

    /// Starts work whose waiter expects `T`.
    pub fn start<T: 'static>(&self) -> Work<T> {
        let id = self.next.get();

        self.next.set(id.checked_add(1).expect("work IDs fit u64"));

        Work {
            id,
            result: PhantomData,
        }
    }

    /// Associates waiting work with a computer for removal cleanup.
    pub fn link_computer(&self, work: WorkId, computer: u32) {
        self.computers
            .borrow_mut()
            .entry(computer)
            .or_default()
            .push(work);
    }

    /// Cancels all waiting work associated with `computers`.
    pub fn cancel_computers(&self, computers: &[u32]) {
        for computer in computers {
            let work = self
                .computers
                .borrow_mut()
                .remove(computer)
                .unwrap_or_default();

            for id in work {
                self.cancel(id);
            }
        }
    }

    /// Finishes `work` with its typed result.
    pub fn finish<T: 'static>(&self, work: WorkId, result: T) {
        self.unlink(work);
        self.events.publish(WorkCompletion {
            id: work,
            result: RefCell::new(Some(WorkOutcome::Complete(Box::new(result)))),
        });
    }

    /// Cancels `work` and wakes its waiter without a result.
    pub fn cancel(&self, work: WorkId) {
        self.unlink(work);
        self.events.publish(WorkCompletion {
            id: work,
            result: RefCell::new(Some(WorkOutcome::Cancel)),
        });
    }

    fn unlink(&self, work: WorkId) {
        self.computers.borrow_mut().retain(|_, ids| {
            ids.retain(|id| *id != work);

            !ids.is_empty()
        });
    }
}
