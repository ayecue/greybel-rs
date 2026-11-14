//! Tree-local typed script objects keyed by [`ObjectId`]. Entries retain the world row keys they access
//! until released by the program.

use std::any::Any;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::rc::Rc;

use crate::protocol::{ObjectId, ScriptFailure};
use crate::{RouteError, RouteResult};

/// Script object class used to separate object identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectClass {
    /// Shell access.
    Shell,
    /// Computer access.
    Computer,
    /// File access.
    File,
    /// Router access.
    Router,
    /// Metaxploit library access.
    Metaxploit,
    /// Cryptography library access.
    Crypto,
    /// Package-manager access.
    AptClient,
    /// Blockchain library access.
    Blockchain,
    /// Smart-appliance access.
    SmartAppliance,
    /// Traffic-network access.
    TrafficNet,
    /// Computer-service access.
    Service,
    /// Library opened through metaxploit.
    MetaLib,
    /// Network session opened through metaxploit.
    NetSession,
    /// Debug-library access.
    DebugLibrary,
    /// Logged-in mail account.
    MetaMail,
    /// Authenticated wallet.
    Wallet,
    /// Authenticated coin.
    Coin,
    /// Coin sub-wallet.
    SubWallet,
    /// Capture-the-flag event.
    Ctf,
}

/// Hashable identity of one game object. The table verifies candidate identities for equality, so a hash
/// collision cannot merge different objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectIdentity {
    class: ObjectClass,
    hash: u64,
}

impl ObjectIdentity {
    /// Builds an identity for `class` from a hashable game identity.
    pub fn of<T: Hash + ?Sized>(class: ObjectClass, value: &T) -> Self {
        let mut hasher = DefaultHasher::new();

        value.hash(&mut hasher);

        Self {
            class,
            hash: hasher.finish(),
        }
    }

    /// Uses another class for identity comparisons, for handles that share the same game object.
    #[must_use]
    pub const fn with_class(self, class: ObjectClass) -> Self {
        Self { class, ..self }
    }
}

/// Computer through which an object can lose its connection. A router reaches its host computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectReach<C> {
    /// Object directly associated with a computer.
    Computer(C),
    /// Router object associated with its host computer.
    Router(C),
}

/// Typed script object stored in an [`ObjectTable`].
pub trait Handle<C>: Any {
    /// Protocol object class built by the runtime.
    const CLASS: ObjectClass;

    /// Returns the game identity used by `HelperScript`'s object registration.
    fn identity(&self) -> ObjectIdentity;

    /// Checks game identity against a candidate with the same identity hash.
    fn same_identity(&self, other: &dyn Any) -> bool;

    /// Returns the computer through which this object is reached.
    fn reach(&self) -> Option<ObjectReach<C>>;

    /// Returns whether registering the same game identity replaces the earlier handle. Shell access replaces
    /// computer access so one ID continues to serve both roles; other registrations reuse the earlier handle.
    fn replaces(&self) -> bool {
        false
    }
}

struct Entry<C> {
    handle: Box<dyn Any>,
    identity: ObjectIdentity,
    reach: Option<ObjectReach<C>>,
}

/// Objects owned by one runtime connection, keyed by ID.
pub struct ObjectTable<C> {
    objects: HashMap<u32, Entry<C>>,
    identities: HashMap<ObjectIdentity, Vec<u32>>,
    /// Objects interrupted because their computer rebooted after registration (`CheckBreakConnecion`).
    interruptions: HashSet<u32>,
    next: u32,
    computer: PhantomData<fn() -> C>,
}

impl<C> Default for ObjectTable<C> {
    fn default() -> Self {
        Self {
            objects: HashMap::new(),
            identities: HashMap::new(),
            interruptions: HashSet::new(),
            next: 0,
            computer: PhantomData,
        }
    }
}

/// Owned access to a connection's object table. Methods borrow the table only for one synchronous call.
#[derive(Clone)]
pub struct ObjectTableHandle<C>(pub(crate) Rc<RefCell<ObjectTable<C>>>);

impl<C: Copy + PartialEq> ObjectTableHandle<C> {
    /// Adds `handle` and returns its object ID.
    pub fn insert<H: Handle<C>>(&self, handle: H) -> ObjectId {
        self.0.borrow_mut().insert(handle)
    }

    /// Reads the table in one synchronous closure.
    pub fn read<R>(&self, read: impl FnOnce(&ObjectTable<C>) -> R) -> R {
        read(&self.0.borrow())
    }

    /// Changes the table in one synchronous closure.
    pub fn write<R>(&self, write: impl FnOnce(&mut ObjectTable<C>) -> R) -> R {
        write(&mut self.0.borrow_mut())
    }

    /// Removes released objects.
    pub fn release(&self, ids: &[ObjectId]) {
        self.0.borrow_mut().release(ids);
    }

    /// Interrupts objects reached through `computer`.
    pub fn interrupt(&self, computer: C) {
        self.0.borrow_mut().interrupt(computer);
    }

    /// Returns the number of registered objects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    /// Returns whether no objects are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }
}

impl<C: Copy + PartialEq> ObjectTable<C> {
    /// Adds `handle` and returns its local ID. A matching uninterrupted handle reuses its existing ID so
    /// scripts preserve game object identity.
    pub fn insert<H: Handle<C>>(&mut self, handle: H) -> ObjectId {
        let identity = handle.identity();
        let reach = handle.reach();
        let identity_id = self
            .identities
            .get(&identity)
            .into_iter()
            .flatten()
            .copied()
            .filter(|id| !self.interruptions.contains(id))
            .find(|id| {
                self.objects
                    .get(id)
                    .is_some_and(|entry| handle.same_identity(entry.handle.as_ref()))
            });

        if let Some(id) = identity_id {
            if handle.replaces() {
                self.objects.insert(
                    id,
                    Entry {
                        handle: Box::new(handle),
                        identity,
                        reach,
                    },
                );
            }

            return ObjectId { id };
        }

        let id = self.next;

        self.next = self.next.wrapping_add(1);
        self.objects.insert(
            id,
            Entry {
                handle: Box::new(handle),
                identity,
                reach,
            },
        );
        self.identities.entry(identity).or_default().push(id);

        ObjectId { id }
    }

    /// Returns handle `H` for `id`.
    ///
    /// # Errors
    ///
    /// Returns [`RouteError::UnknownObject`] for a missing ID or wrong class, and
    /// [`ScriptFailure::ConnectionInterrupted`] if the computer rebooted after registration.
    pub fn get<H: Handle<C>, E>(&self, id: ObjectId) -> RouteResult<&H, E> {
        self.check_connection(id)?;

        self.objects
            .get(&id.id)
            .and_then(|entry| entry.handle.downcast_ref::<H>())
            .ok_or(RouteError::UnknownObject)
    }

    /// Returns mutable handle `H` for `id`.
    ///
    /// # Errors
    ///
    /// Returns [`RouteError::UnknownObject`] for a missing ID or wrong class, and
    /// [`ScriptFailure::ConnectionInterrupted`] if the computer rebooted after registration.
    pub fn get_mut<H: Handle<C>, E>(&mut self, id: ObjectId) -> RouteResult<&mut H, E> {
        self.check_connection(id)?;

        self.objects
            .get_mut(&id.id)
            .and_then(|entry| entry.handle.downcast_mut::<H>())
            .ok_or(RouteError::UnknownObject)
    }

    /// Removes released `ids`, their identities, and their interruption state.
    pub fn release(&mut self, ids: &[ObjectId]) {
        for id in ids {
            if let Some(entry) = self.objects.remove(&id.id)
                && let Some(candidates) = self.identities.get_mut(&entry.identity)
            {
                candidates.retain(|candidate| *candidate != id.id);
                if candidates.is_empty() {
                    self.identities.remove(&entry.identity);
                }
            }

            self.interruptions.remove(&id.id);
        }
    }

    /// Marks objects reached through a rebooted `computer` as interrupted. Later operations raise
    /// [`ScriptFailure::ConnectionInterrupted`] like `CheckBreakConnecion`. Router objects are interrupted
    /// when the router's computer reboots.
    pub fn interrupt(&mut self, computer: C) {
        let objects = self
            .objects
            .iter()
            .filter(|(_, entry)| match entry.reach {
                Some(ObjectReach::Computer(id) | ObjectReach::Router(id)) => id == computer,
                None => false,
            })
            .map(|(id, _)| *id);

        self.interruptions.extend(objects);
    }

    /// Counts objects owned by this tree.
    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    /// Returns whether this tree owns no objects.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    /// Returns [`ScriptFailure::ConnectionInterrupted`] when object `id` was interrupted.
    fn check_connection<E>(&self, id: ObjectId) -> RouteResult<(), E> {
        if self.interruptions.contains(&id.id) {
            return Err(RouteError::Script(ScriptFailure::ConnectionInterrupted));
        }

        Ok(())
    }
}
