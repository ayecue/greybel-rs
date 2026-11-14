//! Computer storage for processes, ports, sessions, and smart-appliance settings. Hardware, users,
//! and files live in `hardware.rs`, `users.rs`, and `nodes.rs`.

use greyhack_behaviour::{FIRST_SPAWN_PID, NetworkKind, PROCESS_RAM_MB, ServiceId};
use turso::params;

use super::Database;
use super::sql::{
    FromRow, RowView, execute, fetch_all, fetch_column, fetch_opt, fetch_value, insert,
};
use crate::{
    Computer, ComputerDraft, ComputerId, LanPlace, LibraryId, NetworkId, PersonId, Port, Process,
    Result, RouterId, SavedNetwork,
};

/// Database columns for [`Computer`].
const COMPUTER_COLUMNS: &str = "computers.id, computers.kind, computers.network_id, computers.parent_id, \
     computers.router_id, computers.gateway, computers.name, computers.local_ip, computers.next_pid, \
     computers.reboot_end, computers.hidden, computers.npc_owner, computers.network_kind FROM computers";

/// Database columns for [`Port`], including its library version.
const PORT_COLUMNS: &str = "computer_ports.number, computer_ports.service, libraries.version, computer_ports.closed, \
     computer_ports.visible FROM computer_ports JOIN libraries ON libraries.id = computer_ports.library_id";

const PROCESS_COLUMNS: &str =
    "pid, user, name, cpu, ram_mb, protected, service, terminal FROM computer_processes";

/// Database columns for [`SavedNetwork`], with its plain password.
const SAVED_NETWORK_COLUMNS: &str = "computer_saved_networks.bssid, computer_saved_networks.essid, \
     COALESCE(passwords.plain, '') AS password, computer_saved_networks.public_ip \
     FROM computer_saved_networks LEFT JOIN passwords ON passwords.hash = computer_saved_networks.password_hash";

impl FromRow for Computer {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: ComputerId(row.get("id")?),
            kind: row.get_code("kind")?,
            network: NetworkId(row.get("network_id")?),
            parent: row.get::<Option<u32>>("parent_id")?.map(ComputerId),
            router: row.get::<Option<u32>>("router_id")?.map(RouterId),
            gateway: row.get("gateway")?,
            name: row.get("name")?,
            local_ip: row.get("local_ip")?,
            next_pid: row.get("next_pid")?,
            reboot_end: row.get("reboot_end")?,
            hidden: row.get("hidden")?,
            npc_owner: row.get::<Option<u32>>("npc_owner")?.map(PersonId),
            network_kind: row.get_code("network_kind")?,
        })
    }
}

impl FromRow for Process {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            pid: row.get("pid")?,
            user: row.get("user")?,
            name: row.get("name")?,
            cpu: row.get("cpu")?,
            ram_mb: row.get("ram_mb")?,
            protected: row.get("protected")?,
            service: row.get_code_opt("service")?,
            is_terminal: row.get("terminal")?,
        })
    }
}

impl FromRow for SavedNetwork {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            bssid: row.get("bssid")?,
            essid: row.get("essid")?,
            password: row.get("password")?,
            public_ip: row.get("public_ip")?,
        })
    }
}

impl FromRow for Port {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            number: row.get("number")?,
            service: row.get_code("service")?,
            version: row.get("version")?,
            closed: row.get("closed")?,
            visible: row.get("visible")?,
        })
    }
}

/// Converts world second `value` to a column value, saturating at the largest one.
pub fn seconds(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Queries of the tables owned by this area.
pub(crate) struct ComputerRepository {
    db: Database,
}

impl ComputerRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Deletes all computer-owned data except its row and users: processes, sessions, ports,
    /// hardware, appliance settings, and files.
    pub async fn clear_computer_contents(&self, computer: ComputerId) -> Result<()> {
        for table in [
            "computer_processes",
            "computer_sessions",
            "computer_ports",
            "computer_saved_networks",
            "computer_hardware",
            "nodes",
        ] {
            execute(
                &self.db.connection(),
                &format!("DELETE FROM {table} WHERE computer_id = ?1"),
                params![computer.0],
            )
            .await?;
        }

        Ok(())
    }

    /// Deletes `id` with everything it owns.
    pub async fn remove_computer(&self, id: ComputerId) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM computers WHERE id = ?1",
            params![id.0],
        )
        .await?;

        Ok(())
    }

    /// Returns computer `id`.
    pub async fn computer(&self, id: ComputerId) -> Result<Option<Computer>> {
        Ok(fetch_opt(
            &self.db.connection(),
            &format!("SELECT {COMPUTER_COLUMNS} WHERE computers.id = ?1"),
            params![id.0],
        )
        .await?)
    }

    /// Returns whether computer `id` exists.
    pub async fn has_computer(&self, id: ComputerId) -> Result<bool> {
        Ok(fetch_value::<i64>(
            &self.db.connection(),
            "SELECT 1 FROM computers WHERE id = ?1",
            params![id.0],
        )
        .await?
        .is_some())
    }

    /// Returns network devices and computers in parent-before-child order, preserving child
    /// insertion order.
    pub async fn computers_in(&self, network: NetworkId) -> Result<Vec<Computer>> {
        Ok(fetch_all(
            &self.db.connection(),
            &format!(
                "SELECT {COMPUTER_COLUMNS} WHERE computers.network_id = ?1 \
                 ORDER BY computers.parent_id IS NOT NULL, computers.parent_seq, computers.id"
            ),
            params![network.0],
        )
        .await?)
    }

    /// Returns computers connected through `router` in join order.
    pub async fn computer_ids_of(&self, router: RouterId) -> Result<Vec<ComputerId>> {
        let ids: Vec<u32> = fetch_column(
            &self.db.connection(),
            "SELECT id FROM computers WHERE router_id = ?1 ORDER BY id",
            params![router.0],
        )
        .await?;

        Ok(ids.into_iter().map(ComputerId).collect())
    }

    /// Returns all devices and computers in `network`.
    pub async fn computer_ids_in(&self, network: NetworkId) -> Result<Vec<ComputerId>> {
        let ids: Vec<u32> = fetch_column(
            &self.db.connection(),
            "SELECT id FROM computers WHERE network_id = ?1 ORDER BY id",
            params![network.0],
        )
        .await?;

        Ok(ids.into_iter().map(ComputerId).collect())
    }

    /// Updates computer name, address, kind, next PID, and reboot end.
    pub async fn save_computer(&self, computer: &Computer) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE computers SET kind = ?2, name = ?3, local_ip = ?4, next_pid = ?5, reboot_end = ?6 WHERE id = ?1",
            params![
                computer.id.0,
                computer.kind.code(),
                computer.name.as_str(),
                computer.local_ip.as_str(),
                computer.next_pid,
                seconds(computer.reboot_end),
            ],
        )
        .await?;

        Ok(())
    }

    /// Moves `computer` to `place` as the last child of its parent device.
    pub async fn move_computer(&self, computer: ComputerId, place: &LanPlace) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE computers SET parent_id = ?2, router_id = ?3, gateway = ?4, \
             network_id = (SELECT network_id FROM computers WHERE id = ?2), \
             parent_seq = (SELECT COALESCE(MAX(parent_seq), 0) + 1 FROM computers WHERE parent_id = ?2) \
             WHERE id = ?1",
            params![
                computer.0,
                place.parent.0,
                place.router.map(|router| router.0),
                place.gateway.as_str()
            ],
        )
        .await?;

        Ok(())
    }

    /// Inserts only the computer row from `draft` into `network` at `place`, last under its parent.
    /// `None` creates the network main router.
    pub async fn insert_computer_row<L, S>(
        &self,
        draft: &ComputerDraft<L, S>,
        network: NetworkId,
        place: Option<&LanPlace>,
    ) -> Result<ComputerId> {
        let id = insert(
            &self.db.connection(),
            "INSERT INTO computers (kind, network_id, parent_id, parent_seq, router_id, gateway, name, local_ip, \
             next_pid, reboot_end, hidden, npc_owner, network_kind) VALUES (?1, ?2, ?3, CASE WHEN ?3 IS NULL THEN \
             NULL ELSE (SELECT COALESCE(MAX(parent_seq), 0) + 1 FROM computers WHERE parent_id = ?3) END, ?4, ?5, \
             ?6, ?7, ?8, ?9, ?10, ?11, COALESCE(?12, (SELECT kind FROM networks WHERE id = ?2)))",
            params![
                draft.kind.code(),
                network.0,
                place.map(|place| place.parent.0),
                place.and_then(|place| place.router).map(|router| router.0),
                place.map_or("", |place| place.gateway.as_str()),
                draft.name.as_str(),
                draft.local_ip.as_str(),
                draft.next_pid,
                seconds(draft.reboot_end),
                draft.hidden,
                draft.npc_owner.map(|person| person.0),
                draft.network_kind.map(NetworkKind::code),
            ],
        )
        .await?;

        Ok(ComputerId(u32::try_from(id).expect("computer ids fit u32")))
    }

    /// Returns the networks `computer` connected to, the most recent last.
    pub async fn saved_networks(&self, computer: ComputerId) -> Result<Vec<SavedNetwork>> {
        Ok(fetch_all(
            &self.db.connection(),
            &format!(
                "SELECT {SAVED_NETWORK_COLUMNS} WHERE computer_saved_networks.computer_id = ?1 \
                 ORDER BY computer_saved_networks.id"
            ),
            params![computer.0],
        )
        .await?)
    }

    /// Makes `network` the most recent saved network of `computer`, replacing the one with its BSSID.
    pub async fn add_saved_network(
        &self,
        computer: ComputerId,
        network: &SavedNetwork,
        password_hash: &str,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM computer_saved_networks WHERE computer_id = ?1 AND bssid = ?2",
            params![computer.0, network.bssid.as_str()],
        )
        .await?;
        execute(
            &self.db.connection(),
            "INSERT INTO computer_saved_networks (computer_id, bssid, essid, password_hash, public_ip) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                computer.0,
                network.bssid.as_str(),
                network.essid.as_str(),
                password_hash,
                network.public_ip.as_str(),
            ],
        )
        .await?;

        Ok(())
    }

    /// Returns the processes of `computer` in process-ID order.
    pub async fn processes(&self, computer: ComputerId) -> Result<Vec<Process>> {
        Ok(fetch_all(
            &self.db.connection(),
            &format!("SELECT {PROCESS_COLUMNS} WHERE computer_id = ?1 ORDER BY pid"),
            params![computer.0],
        )
        .await?)
    }

    /// Returns process `pid` of `computer`.
    pub async fn process(&self, computer: ComputerId, pid: u32) -> Result<Option<Process>> {
        Ok(fetch_opt(
            &self.db.connection(),
            &format!("SELECT {PROCESS_COLUMNS} WHERE computer_id = ?1 AND pid = ?2"),
            params![computer.0, pid],
        )
        .await?)
    }

    /// Adds `process` to `computer`.
    pub async fn insert_process(&self, computer: ComputerId, process: &Process) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO computer_processes (computer_id, pid, user, name, cpu, ram_mb, protected, service, terminal) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                computer.0,
                process.pid,
                process.user.as_str(),
                process.name.as_str(),
                process.cpu,
                process.ram_mb,
                process.protected,
                process.service.map(ServiceId::code),
                process.is_terminal,
            ],
        )
        .await?;

        Ok(())
    }

    /// Starts `command` as `user` using the next computer PID and returns that PID.
    pub async fn spawn_process(
        &self,
        computer: ComputerId,
        user: &str,
        command: &str,
        service: Option<ServiceId>,
        is_terminal: bool,
    ) -> Result<u32> {
        self.spawn_process_as(
            computer,
            &Process {
                pid: 0,
                user: user.to_owned(),
                name: command.to_owned(),
                cpu: 0.0,
                ram_mb: PROCESS_RAM_MB,
                protected: false,
                service,
                is_terminal,
            },
        )
        .await
    }

    /// Starts `process` with the next computer PID in place of its own and returns that PID.
    pub async fn spawn_process_as(&self, computer: ComputerId, process: &Process) -> Result<u32> {
        let pid: u32 = fetch_value(
            &self.db.connection(),
            "SELECT next_pid FROM computers WHERE id = ?1",
            params![computer.0],
        )
        .await?
        .unwrap_or(FIRST_SPAWN_PID);

        execute(
            &self.db.connection(),
            "UPDATE computers SET next_pid = ?2 WHERE id = ?1",
            params![computer.0, pid + 1],
        )
        .await?;
        self.insert_process(
            computer,
            &Process {
                pid,
                ..process.clone()
            },
        )
        .await?;

        Ok(pid)
    }

    /// Ends process `pid` of `computer`. Returns whether it existed.
    pub async fn end_process(&self, computer: ComputerId, pid: u32) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "DELETE FROM computer_processes WHERE computer_id = ?1 AND pid = ?2",
            params![computer.0, pid],
        )
        .await?
            > 0)
    }

    /// Ends the daemons of `service` on `computer`.
    pub async fn end_service_processes(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM computer_processes WHERE computer_id = ?1 AND service = ?2",
            params![computer.0, service.code()],
        )
        .await?;

        Ok(())
    }

    /// Ends all computer processes and sessions. PID allocation goes on, so a process started
    /// afterwards never takes the PID of a program the restart ended (`Computer.RemoveAllProcess`).
    pub async fn clear_processes(&self, computer: ComputerId) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM computer_processes WHERE computer_id = ?1",
            params![computer.0],
        )
        .await?;
        execute(
            &self.db.connection(),
            "DELETE FROM computer_sessions WHERE computer_id = ?1",
            params![computer.0],
        )
        .await?;

        Ok(())
    }

    /// Returns the ports of `computer` in creation order.
    pub async fn ports(&self, computer: ComputerId) -> Result<Vec<Port>> {
        Ok(fetch_all(
            &self.db.connection(),
            &format!("SELECT {PORT_COLUMNS} WHERE computer_ports.computer_id = ?1 ORDER BY computer_ports.id"),
            params![computer.0],
        )
        .await?)
    }

    /// Returns port `number` of `computer`.
    pub async fn port(&self, computer: ComputerId, number: u16) -> Result<Option<Port>> {
        Ok(fetch_opt(
            &self.db.connection(),
            &format!("SELECT {PORT_COLUMNS} WHERE computer_ports.computer_id = ?1 AND computer_ports.number = ?2"),
            params![computer.0, number],
        )
        .await?)
    }

    /// Returns the port of `computer` that runs `service`.
    pub async fn service_port(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<Option<Port>> {
        Ok(fetch_opt(
            &self.db.connection(),
            &format!(
                "SELECT {PORT_COLUMNS} WHERE computer_ports.computer_id = ?1 AND computer_ports.service = ?2 \
                 ORDER BY computer_ports.id"
            ),
            params![computer.0, service.code()],
        )
        .await?)
    }

    /// Opens `port` on `computer` for the service running on `library`.
    pub async fn insert_port(
        &self,
        computer: ComputerId,
        port: &Port,
        library: LibraryId,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO computer_ports (computer_id, number, service, library_id, closed, visible) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                computer.0,
                port.number,
                port.service.code(),
                library.0,
                port.closed,
                port.visible,
            ],
        )
        .await?;

        Ok(())
    }

    /// Points computer port `number` at `library`. Returns whether the port exists.
    pub async fn set_port_library(
        &self,
        computer: ComputerId,
        number: u16,
        library: LibraryId,
    ) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "UPDATE computer_ports SET library_id = ?3 WHERE computer_id = ?1 AND number = ?2",
            params![computer.0, number, library.0],
        )
        .await?
            > 0)
    }

    /// Removes `service` ports from `computer` and their router forwards.
    pub async fn remove_service_ports(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM computer_ports WHERE computer_id = ?1 AND service = ?2",
            params![computer.0, service.code()],
        )
        .await?;

        Ok(())
    }

    /// Returns users with open sessions on `computer` in login order.
    pub async fn sessions(&self, computer: ComputerId) -> Result<Vec<String>> {
        Ok(fetch_column(
            &self.db.connection(),
            "SELECT user FROM computer_sessions WHERE computer_id = ?1 ORDER BY id",
            params![computer.0],
        )
        .await?)
    }

    /// Opens a session of `user` on `computer`.
    pub async fn insert_session(&self, computer: ComputerId, user: &str) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO computer_sessions (computer_id, user) VALUES (?1, ?2)",
            params![computer.0, user],
        )
        .await?;

        Ok(())
    }

    /// Closes the session of `user` on `computer`. Returns whether it existed.
    pub async fn remove_session(&self, computer: ComputerId, user: &str) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "DELETE FROM computer_sessions WHERE id = \
             (SELECT id FROM computer_sessions WHERE computer_id = ?1 AND user = ?2 ORDER BY id LIMIT 1)",
            params![computer.0, user],
        )
        .await?
            > 0)
    }
}
