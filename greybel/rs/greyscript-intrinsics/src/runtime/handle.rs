//! Reference-counted handles to server-side script objects.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::protocol::ObjectId;

/// Server-object IDs queued for release before the next operation.
#[derive(Default)]
struct ReleaseQueue(RefCell<Vec<ObjectId>>);

/// Reference-counted server-side script object; dropping the last handle queues its release.
#[derive(Clone)]
pub(crate) struct Handle(Rc<HandleState>);

struct HandleState {
    id: ObjectId,
    releases: Rc<ReleaseQueue>,
}

impl Handle {
    /// Returns the ID assigned by the game server.
    #[must_use]
    pub(crate) fn id(&self) -> ObjectId {
        self.0.id
    }
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("Handle")
            .field(&self.0.id.id)
            .finish()
    }
}

impl Drop for HandleState {
    fn drop(&mut self) {
        self.releases.0.borrow_mut().push(self.id);
    }
}

/// Live handles shared by all values representing the same server object, and the releases queued for dropped ones.
#[derive(Default)]
pub(super) struct HandleTable {
    live: RefCell<HashMap<u32, Weak<HandleState>>>,
    releases: Rc<ReleaseQueue>,
}

impl HandleTable {
    /// Removes and returns the object IDs queued for release.
    pub(super) fn take_releases(&self) -> Vec<ObjectId> {
        std::mem::take(&mut *self.releases.0.borrow_mut())
    }

    /// Returns the live handle for server object `id`, or creates one if none exists.
    /// Reuses handles returned repeatedly by the game server (`HelperScript.AddShell` and similar APIs) and cancels any queued release for `id`.
    pub(super) fn object(&self, id: ObjectId) -> Handle {
        let mut live = self.live.borrow_mut();

        if let Some(state) = live.get(&id.id).and_then(Weak::upgrade) {
            return Handle(state);
        }

        self.releases
            .0
            .borrow_mut()
            .retain(|release| release.id != id.id);
        live.retain(|_, state| state.strong_count() > 0);

        let state = Rc::new(HandleState {
            id,
            releases: Rc::clone(&self.releases),
        });

        live.insert(id.id, Rc::downgrade(&state));

        Handle(state)
    }
}
