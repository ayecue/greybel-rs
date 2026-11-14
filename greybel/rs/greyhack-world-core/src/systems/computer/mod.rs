//! Computers: hardware, startup, system logs, accounts, processes and services, network cards, files, and
//! what scripts do to them through `ComputerIntrinsics`.

use std::rc::Rc;

use greyhack_behaviour::ServiceId;

use crate::storage::*;
use crate::systems::computer::hardware::Hardware;
use crate::systems::*;
use crate::*;

pub mod failure;
pub mod filesystem;
pub mod hardware;
pub mod logs;
pub mod net_cards;
pub mod processes;
pub mod startup;
pub mod users;

/// Computer services.
pub struct ComputerService {
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) hardware_rows: Rc<HardwareRepository>,
    pub(crate) user_rows: Rc<UserRepository>,
    pub(crate) person_rows: Rc<PersonRepository>,
    pub hardware: Dep<Hardware>,
    pub users: Dep<UserService>,
    pub processes: Dep<ProcessService>,
    pub startup: Dep<Startup>,
    pub logs: Dep<Logs>,
}

impl ComputerService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        hardware_rows: Rc<HardwareRepository>,
        user_rows: Rc<UserRepository>,
        person_rows: Rc<PersonRepository>,
    ) -> Self {
        Self {
            computer_rows,
            hardware_rows,
            user_rows,
            person_rows,
            hardware: registry.get(),
            users: registry.get(),
            processes: registry.get(),
            startup: registry.get(),
            logs: registry.get(),
        }
    }
}

impl ComputerService {
    /// Removes the files and other contents owned by `computer`.
    pub async fn clear_computer_contents(&self, computer: ComputerId) -> Result<()> {
        self.computer_rows.clear_computer_contents(computer).await
    }

    /// Removes computer `id` and its dependent rows.
    pub async fn remove_computer(&self, id: ComputerId) -> Result<()> {
        self.computer_rows.remove_computer(id).await
    }

    /// Returns computer `id`, if it exists.
    pub async fn computer(&self, id: ComputerId) -> Result<Option<Computer>> {
        self.computer_rows.computer(id).await
    }

    /// Returns whether computer `id` exists.
    pub async fn has_computer(&self, id: ComputerId) -> Result<bool> {
        self.computer_rows.has_computer(id).await
    }

    /// Returns the computers belonging to `network`.
    pub async fn computers_in(&self, network: NetworkId) -> Result<Vec<Computer>> {
        self.computer_rows.computers_in(network).await
    }

    /// Returns the IDs of computers behind `router`.
    pub async fn computer_ids_of(&self, router: RouterId) -> Result<Vec<ComputerId>> {
        self.computer_rows.computer_ids_of(router).await
    }

    /// Returns the IDs of computers belonging to `network`.
    pub async fn computer_ids_in(&self, network: NetworkId) -> Result<Vec<ComputerId>> {
        self.computer_rows.computer_ids_in(network).await
    }

    /// Writes the mutable fields of `computer`.
    pub async fn save_computer(&self, computer: &Computer) -> Result<()> {
        self.computer_rows.save_computer(computer).await
    }

    /// Inserts only the computer row from `draft` into `network` at `place`.
    pub async fn insert_computer_row<L, S>(
        &self,
        draft: &ComputerDraft<L, S>,
        network: NetworkId,
        place: Option<&LanPlace>,
    ) -> Result<ComputerId> {
        self.computer_rows
            .insert_computer_row(draft, network, place)
            .await
    }

    /// Inserts a computer and its hardware at `place`, resolving libraries with `library_id`.
    pub async fn insert_computer_at_with<L, S>(
        &self,
        draft: &ComputerDraft<L, S>,
        network: NetworkId,
        place: Option<&LanPlace>,
        library_id: impl AsyncFn(&L) -> Result<crate::LibraryId>,
    ) -> Result<ComputerId> {
        self.hardware
            .insert_computer_at_with(draft, network, place, library_id)
            .await
    }

    /// Returns the processes stored on `computer`.
    pub async fn processes(&self, computer: ComputerId) -> Result<Vec<Process>> {
        self.computer_rows.processes(computer).await
    }

    /// Stores `process` on `computer`.
    pub async fn insert_process(&self, computer: ComputerId, process: &Process) -> Result<()> {
        self.computer_rows.insert_process(computer, process).await
    }

    /// Returns the open ports of `computer`.
    pub async fn ports(&self, computer: ComputerId) -> Result<Vec<Port>> {
        self.computer_rows.ports(computer).await
    }

    /// Returns port `number` of `computer`, if it is open.
    pub async fn port(&self, computer: ComputerId, number: u16) -> Result<Option<Port>> {
        self.computer_rows.port(computer, number).await
    }

    /// Returns the port on which `computer` exposes `service`, if any.
    pub async fn service_port(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<Option<Port>> {
        self.computer_rows.service_port(computer, service).await
    }

    /// Opens `port` on `computer` and associates it with `library`.
    pub async fn insert_port(
        &self,
        computer: ComputerId,
        port: &Port,
        library: LibraryId,
    ) -> Result<()> {
        self.computer_rows
            .insert_port(computer, port, library)
            .await
    }

    /// Closes every port associated with `service` on `computer`.
    pub async fn remove_service_ports(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<()> {
        self.computer_rows
            .remove_service_ports(computer, service)
            .await
    }

    /// Returns the users with sessions on `computer`.
    pub async fn sessions(&self, computer: ComputerId) -> Result<Vec<String>> {
        self.computer_rows.sessions(computer).await
    }

    /// Adds a session for `user` on `computer`.
    pub async fn insert_session(&self, computer: ComputerId, user: &str) -> Result<()> {
        self.computer_rows.insert_session(computer, user).await
    }

    /// Removes `user`’s session from `computer` and reports whether one existed.
    pub async fn remove_session(&self, computer: ComputerId, user: &str) -> Result<bool> {
        self.computer_rows.remove_session(computer, user).await
    }

    /// Returns the hardware installed in `computer`.
    pub async fn hardware(&self, computer: ComputerId) -> Result<crate::model::Hardware> {
        self.hardware_rows.hardware(computer).await
    }

    /// Installs `hardware` in `computer`.
    pub async fn insert_hardware(
        &self,
        computer: ComputerId,
        hardware: &crate::model::Hardware,
    ) -> Result<()> {
        self.hardware_rows.insert_hardware(computer, hardware).await
    }

    /// Registers `plain` in the password table and returns its hash.
    pub async fn register_password(&self, plain: &str) -> Result<String> {
        self.user_rows.register_password(plain).await
    }

    /// Returns the user accounts of `computer`.
    pub async fn users(&self, computer: ComputerId) -> Result<Vec<User>> {
        self.user_rows.users(computer).await
    }

    /// Returns account `name` on `computer`, if it exists.
    pub async fn user(&self, computer: ComputerId, name: &str) -> Result<Option<User>> {
        self.user_rows.user(computer, name).await
    }

    /// Returns the person who owns `computer`, if any.
    pub async fn person_on(&self, computer: ComputerId) -> Result<Option<PersonId>> {
        self.user_rows.person_on(computer).await
    }

    /// Adds `user` to `computer`.
    pub async fn insert_user(&self, computer: ComputerId, user: &User) -> Result<()> {
        self.user_rows.insert_user(computer, user).await
    }

    /// Writes account `user` on `computer`.
    pub async fn save_user(&self, computer: ComputerId, user: &User) -> Result<()> {
        self.user_rows.save_user(computer, user).await
    }

    /// Removes account `name` from `computer` and reports whether it existed.
    pub async fn remove_user(&self, computer: ComputerId, name: &str) -> Result<bool> {
        self.user_rows.remove_user(computer, name).await
    }

    /// Returns person `id`, if it exists.
    pub async fn person(&self, id: PersonId) -> Result<Option<Person>> {
        self.person_rows.person(id).await
    }

    /// Stores `person` and returns its ID.
    pub async fn insert_person(&self, person: &Person) -> Result<PersonId> {
        self.person_rows.insert_person(person).await
    }

    /// Writes the identity fields of `person`.
    pub async fn save_person(&self, person: &Person) -> Result<()> {
        self.person_rows.save_person(person).await
    }
}
