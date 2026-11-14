//! Registry of lazy services and domain capabilities.

use std::any::{Any, TypeId, type_name};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use greyhack_world_router::RouteTable;
use greyhack_world_routines::{Routine, Scheduler};
use greyhack_world_storage::DomainSchema;

use crate::{CapabilityRegistry, Dep, Event, EventFuture, EventQueue, WorkQueue};

/// Parts shared by the assembled world and its server.
pub struct Assembly {
    /// Domain schemas in registration order.
    pub schemas: Vec<DomainSchema>,
    /// Routine scheduler with every kind registered.
    pub scheduler: Rc<Scheduler>,
    /// Driver-independent domain routes.
    pub routes: Rc<RouteTable>,
    /// Ordered domain event queue.
    pub events: Rc<EventQueue>,
    /// Typed waiting-work queue.
    pub works: Rc<WorkQueue>,
}

/// Type-erased holder that preserves unsized service and capability types.
pub(crate) struct ServiceHolder<T: ?Sized + 'static>(pub(crate) Rc<T>);

/// One world's lazy service and capability registry.
pub struct Registry {
    world: Box<dyn Any>,
    services: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
    requests: RefCell<BTreeMap<TypeId, &'static str>>,
    schemas: RefCell<Vec<DomainSchema>>,
    scheduler: Rc<Scheduler>,
    routes: Rc<RouteTable>,
    events: Rc<EventQueue>,
    works: Rc<WorkQueue>,
    capabilities: Rc<CapabilityRegistry>,
}

impl Registry {
    /// Creates a registry for one world handle.
    pub fn new<W: 'static>(world: W) -> Self {
        let events = Rc::new(EventQueue::default());
        let works = Rc::new(WorkQueue::new(events.clone()));

        Self {
            world: Box::new(world),
            services: RefCell::new(HashMap::new()),
            requests: RefCell::new(BTreeMap::new()),
            schemas: RefCell::new(Vec::new()),
            scheduler: Rc::new(Scheduler::new()),
            routes: Rc::new(RouteTable::new()),
            events,
            works,
            capabilities: Rc::new(CapabilityRegistry::default()),
        }
    }

    /// Returns the concrete world handle supplied at construction.
    pub fn world<W: 'static>(&self) -> &W {
        self.world
            .downcast_ref::<W>()
            .expect("registry world has its assembly type")
    }

    /// Returns a lazy dependency on `T`, recording the requested type without resolving it.
    #[must_use]
    pub fn get<T: ?Sized + 'static>(self: &Rc<Self>) -> Dep<T> {
        self.requests
            .borrow_mut()
            .insert(TypeId::of::<T>(), type_name::<T>());

        Dep {
            registry: Rc::downgrade(self),
            service: Default::default(),
        }
    }

    pub(crate) fn resolve<T: ?Sized + 'static>(&self) -> Option<Rc<T>> {
        self.services
            .borrow()
            .get(&TypeId::of::<T>())
            .cloned()
            .map(|service| {
                service
                    .downcast::<ServiceHolder<T>>()
                    .expect("services are indexed by their registered type")
                    .0
                    .clone()
            })
    }

    /// Returns requested service types for which nothing is registered.
    #[must_use]
    pub fn missing_services(&self) -> Vec<&'static str> {
        let services = self.services.borrow();

        self.requests
            .borrow()
            .iter()
            .filter(|(id, _)| !services.contains_key(id))
            .map(|(_, name)| *name)
            .collect()
    }

    /// Registers one instance of service `T`.
    pub fn register_service<T: ?Sized + 'static>(&self, service: Rc<T>) {
        let previous = self
            .services
            .borrow_mut()
            .insert(TypeId::of::<T>(), Rc::new(ServiceHolder(service)));

        assert!(
            previous.is_none(),
            "service {} is registered twice",
            type_name::<T>()
        );
    }

    /// Registers a domain schema.
    pub fn register_schema(&self, schema: DomainSchema) {
        self.schemas.borrow_mut().push(schema);
    }

    /// Registers a routine instance with the shared scheduler.
    pub fn register_routine<R: Routine>(&self, kind: Rc<R>) {
        self.scheduler.register(kind);
    }

    /// Returns the scheduler for services that schedule routines.
    #[must_use]
    pub fn scheduler(&self) -> Rc<Scheduler> {
        self.scheduler.clone()
    }

    /// Registers routes in the driver-independent table.
    pub fn register_routes(&self, routes: impl FnOnce(&RouteTable)) {
        routes(&self.routes);
    }

    /// Registers a typed event subscriber at assembly time.
    pub fn register_subscriber<E: Event>(&self, subscriber: impl Fn(&E) + 'static) {
        self.events.on(subscriber);
    }

    /// Registers an asynchronous typed event subscriber at assembly time.
    pub fn register_async_subscriber<E: Event>(
        &self,
        subscriber: impl for<'a> Fn(&'a E) -> EventFuture<'a> + 'static,
    ) {
        self.events.on_async(subscriber);
    }

    /// Registers one implementation of a multi-module capability.
    pub fn register_capability<T: ?Sized + 'static>(&self, capability: Rc<T>) {
        self.capabilities.add(capability);
    }

    /// Returns the multi-module capability collection.
    #[must_use]
    pub fn capabilities(&self) -> Rc<CapabilityRegistry> {
        self.capabilities.clone()
    }

    /// Returns the ordered event queue.
    #[must_use]
    pub fn events(&self) -> Rc<EventQueue> {
        self.events.clone()
    }

    /// Returns the typed work queue.
    #[must_use]
    pub fn works(&self) -> Rc<WorkQueue> {
        self.works.clone()
    }

    /// Returns the assembled capabilities without resolving any lazy dependency.
    #[must_use]
    pub fn assembly(&self) -> Assembly {
        Assembly {
            schemas: self.schemas.borrow().clone(),
            scheduler: self.scheduler.clone(),
            routes: self.routes.clone(),
            events: self.events.clone(),
            works: self.works.clone(),
        }
    }
}
