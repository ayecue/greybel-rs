//! Persistence of the core: connection handling, schemas, and one repository per table area.

mod computers;
mod database;
mod hardware;
mod libraries;
pub mod nodes;
mod persons;
mod routers;
pub mod schema;
pub mod sql;
mod users;
pub mod world_meta;

use std::rc::Rc;

pub(crate) use computers::ComputerRepository;
pub use computers::seconds;
pub use database::Database;
pub(crate) use hardware::HardwareRepository;
pub(crate) use libraries::LibraryIdentityRepository;
pub(crate) use nodes::NodeRepository;
pub(crate) use persons::PersonRepository;
pub(crate) use routers::RouterRepository;
pub(crate) use users::UserRepository;
pub(crate) use world_meta::WorldMetaRepository;

/// Repositories shared by core services and built once with their module.
#[derive(Clone)]
pub(crate) struct CoreRepositories {
    pub computers: Rc<ComputerRepository>,
    pub hardware: Rc<HardwareRepository>,
    pub nodes: Rc<NodeRepository>,
    pub users: Rc<UserRepository>,
    pub routers: Rc<RouterRepository>,
    pub libraries: Rc<LibraryIdentityRepository>,
    pub persons: Rc<PersonRepository>,
    pub world_meta: Rc<WorldMetaRepository>,
}

impl CoreRepositories {
    pub(crate) fn new(db: Database) -> Rc<Self> {
        Rc::new(Self {
            computers: Rc::new(ComputerRepository::new(db.clone())),
            hardware: Rc::new(HardwareRepository::new(db.clone())),
            nodes: Rc::new(NodeRepository::new(db.clone())),
            users: Rc::new(UserRepository::new(db.clone())),
            routers: Rc::new(RouterRepository::new(db.clone())),
            libraries: Rc::new(LibraryIdentityRepository::new(db.clone())),
            persons: Rc::new(PersonRepository::new(db.clone())),
            world_meta: Rc::new(WorldMetaRepository::new(db)),
        })
    }
}
