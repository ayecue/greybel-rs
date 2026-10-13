//! Network, router, and switch storage. Each router combines a device computer row with a `routers`
//! row and owns forwards, firewall rules, reverse shells, traffic data, and vehicles.
//! `computers.parent_id` defines the LAN tree (`computers.rs`).

use greyhack_behaviour::{FirewallRule, NetworkKind};
use turso::params;

use super::Database;
use super::sql::{
    FromRow, RowView, execute, fetch_all, fetch_column, fetch_opt, fetch_value, insert,
};
use crate::{
    ComputerId, Contact, LanPlace, Network, NetworkAdmin, NetworkDraft, NetworkId, PersonId, Port,
    PortForward, Result, ReverseShell, Router, RouterDraft, RouterId, TrafficNet, WorkPerformance,
};

/// [`Router`] columns from router, computer, and network rows, including main router, LAN address,
/// and Wi-Fi password.
const ROUTER_COLUMNS: &str = "routers.computer_id, computers.network_id, computers.kind, \
     (SELECT mains.id FROM computers AS mains WHERE mains.network_id = computers.network_id \
     AND mains.parent_id IS NULL) AS main_id, networks.public_ip, computers.local_ip, networks.domain, \
     networks.contact_name, networks.contact_mail, networks.contact_phone, routers.essid, routers.bssid, \
     COALESCE(passwords.plain, '') AS wifi_password, routers.firewall_active, networks.map_x, networks.map_y \
     FROM routers JOIN computers ON computers.id = routers.computer_id \
     JOIN networks ON networks.id = computers.network_id \
     LEFT JOIN passwords ON passwords.hash = routers.wifi_password_hash";

/// Filters [`ROUTER_COLUMNS`] to select main routers only.
const MAIN: &str = "computers.parent_id IS NULL";

/// [`PortForward`] columns with device, internal port, and port library version.
const FORWARD_COLUMNS: &str = "port_forwards.number AS forward, computers.id AS computer_id, \
     computers.local_ip AS lan_ip, computer_ports.number, computer_ports.service, libraries.version, \
     computer_ports.closed, computer_ports.visible FROM port_forwards \
     JOIN computer_ports ON computer_ports.id = port_forwards.port_id \
     JOIN computers ON computers.id = computer_ports.computer_id \
     JOIN libraries ON libraries.id = computer_ports.library_id";

impl FromRow for Router {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        let contact = match (
            row.get::<Option<String>>("contact_name")?,
            row.get::<Option<String>>("contact_mail")?,
            row.get::<Option<String>>("contact_phone")?,
        ) {
            (Some(name), Some(mail), Some(phone)) => Some(Contact { name, mail, phone }),
            _ => None,
        };

        Ok(Self {
            id: RouterId(row.get("computer_id")?),
            network: NetworkId(row.get("network_id")?),
            kind: row.get("kind")?,
            main: RouterId(row.get("main_id")?),
            public_ip: row.get("public_ip")?,
            local_ip: row.get("local_ip")?,
            domain: row.get("domain")?,
            contact,
            essid: row.get("essid")?,
            bssid: row.get("bssid")?,
            wifi_password: row.get("wifi_password")?,
            map: [row.get("map_x")?, row.get("map_y")?],
            firewall_active: row.get("firewall_active")?,
        })
    }
}

impl FromRow for NetworkAdmin {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        let code: u8 = row.get("work_performance")?;

        Ok(Self {
            intelligence: row.get("intelligence")?,
            pc_experience: row.get("pc_experience")?,
            work_performance: WorkPerformance::from_code(code).ok_or_else(|| {
                turso::Error::ConversionFailure(format!("{code} is no work performance"))
            })?,
            dead_or_fired: row.get("dead_or_fired")?,
        })
    }
}

impl FromRow for Network {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: NetworkId(row.get("id")?),
            kind: row.get("kind")?,
            has_plan: row.get("has_plan")?,
            disabled: row.get("disabled")?,
            net_speed: row.get("net_speed")?,
        })
    }
}

impl FromRow for PortForward {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            number: row.get("forward")?,
            computer: ComputerId(row.get("computer_id")?),
            lan_ip: row.get("lan_ip")?,
            port: Port::from_row(row)?,
        })
    }
}

impl FromRow for ReverseShell {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            computer: ComputerId(row.get("computer_id")?),
            port: row.get("port")?,
            user: row.get("user")?,
            pid: row.get("pid")?,
            creation_second: row.get::<i64>("created_at")?.try_into().unwrap_or(0),
        })
    }
}

impl FromRow for TrafficNet {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            credentials: row.get::<Option<u32>>("credentials_id")?.map(PersonId),
            override_enabled: row.get("override_enabled")?,
            override_used: row.get("override_used")?,
        })
    }
}

impl FromRow for FirewallRule {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            allow: row.get("allow")?,
            port: row.get("port")?,
            source: row.get("source")?,
            destination: row.get("destination")?,
        })
    }
}

impl Database {
    /// Deletes all port forwards for `router`.
    pub(crate) async fn clear_port_forwards(&self, router: RouterId) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM port_forwards WHERE router_id = ?1",
            params![router.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn network(&self, id: NetworkId) -> Result<Option<Network>> {
        Ok(fetch_opt(
            &self.conn,
            "SELECT id, kind, has_plan, disabled, net_speed FROM networks WHERE id = ?1",
            params![id.0],
        )
        .await?)
    }

    /// Inserts `draft` network and police traffic data without devices.
    /// Returns the administrator of `network`, if any.
    pub(crate) async fn network_admin(&self, network: NetworkId) -> Result<Option<NetworkAdmin>> {
        Ok(fetch_opt(
            &self.conn,
            "SELECT intelligence, pc_experience, work_performance, dead_or_fired FROM network_admins \
             WHERE network_id = ?1",
            params![network.0],
        )
        .await?)
    }

    /// Updates network connection speed (`NetworkLan.netSpeed`).
    pub(crate) async fn set_net_speed(&self, network: NetworkId, net_speed: u8) -> Result<()> {
        execute(
            &self.conn,
            "UPDATE networks SET net_speed = ?2 WHERE id = ?1",
            params![network.0, net_speed],
        )
        .await?;

        Ok(())
    }

    /// Enables or disables `network` for nonpayment (`BankSubs.DisableNetwork`).
    pub(crate) async fn disable_network(&self, network: NetworkId, disabled: bool) -> Result<()> {
        execute(
            &self.conn,
            "UPDATE networks SET disabled = ?2 WHERE id = ?1",
            params![network.0, disabled],
        )
        .await?;

        Ok(())
    }

    /// Records the address the starter mission mail of `network`'s NPCs names (`Persona.GenerateMailStarterMission`).
    pub(crate) async fn set_mission_contact(
        &self,
        network: NetworkId,
        contact: &str,
    ) -> Result<()> {
        execute(
            &self.conn,
            "UPDATE networks SET mission_contact = ?2 WHERE id = ?1",
            params![network.0, contact],
        )
        .await?;

        Ok(())
    }

    /// The address the starter mission mail of `network`'s NPCs names, if a mission was placed in it.
    pub(crate) async fn mission_contact(&self, network: NetworkId) -> Result<Option<String>> {
        Ok(fetch_value::<Option<String>>(
            &self.conn,
            "SELECT mission_contact FROM networks WHERE id = ?1",
            params![network.0],
        )
        .await?
        .flatten())
    }

    /// Returns the world NPC administering `network`, if any.
    pub(crate) async fn network_admin_person(
        &self,
        network: NetworkId,
    ) -> Result<Option<PersonId>> {
        Ok(fetch_value::<Option<u32>>(
            &self.conn,
            "SELECT person_id FROM network_admins WHERE network_id = ?1",
            params![network.0],
        )
        .await?
        .flatten()
        .map(PersonId))
    }

    /// Assigns `admin` to `network`, linking NPC `person` when present in this world.
    pub(crate) async fn set_network_admin(
        &self,
        network: NetworkId,
        person: Option<PersonId>,
        admin: &NetworkAdmin,
    ) -> Result<()> {
        execute(
            &self.conn,
            "INSERT OR REPLACE INTO network_admins (network_id, person_id, intelligence, pc_experience, \
             work_performance, dead_or_fired) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                network.0,
                person.map(|person| person.0),
                admin.intelligence,
                admin.pc_experience,
                admin.work_performance as u8,
                admin.dead_or_fired,
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_network(&self, draft: &NetworkDraft) -> Result<NetworkId> {
        let contact = draft.contact.as_ref();
        let id = insert(
            &self.conn,
            "INSERT INTO networks (kind, public_ip, domain, contact_name, contact_mail, contact_phone, map_x, map_y, \
             has_plan) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                draft.kind.code(),
                draft.public_ip.as_str(),
                draft.domain.as_str(),
                contact.map(|contact| contact.name.as_str()),
                contact.map(|contact| contact.mail.as_str()),
                contact.map(|contact| contact.phone.as_str()),
                draft.position[0],
                draft.position[1],
                draft.has_plan,
            ],
        )
        .await?;

        Ok(NetworkId(u32::try_from(id).expect("network ids fit u32")))
    }

    pub(crate) async fn set_network_kind(&self, id: NetworkId, kind: NetworkKind) -> Result<()> {
        execute(
            &self.conn,
            "UPDATE networks SET kind = ?2 WHERE id = ?1",
            params![id.0, kind.code()],
        )
        .await?;
        execute(
            &self.conn,
            "UPDATE computers SET network_kind = ?2 WHERE network_id = ?1",
            params![id.0, kind.code()],
        )
        .await?;

        Ok(())
    }

    /// Deletes `network`, all devices and routers, and their dependent rows.
    pub(crate) async fn remove_network(&self, id: NetworkId) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM networks WHERE id = ?1",
            params![id.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn router(&self, id: RouterId) -> Result<Option<Router>> {
        Ok(fetch_opt(
            &self.conn,
            &format!("SELECT {ROUTER_COLUMNS} WHERE routers.computer_id = ?1"),
            params![id.0],
        )
        .await?)
    }

    /// Returns the main router at `public_ip`.
    pub(crate) async fn router_at(&self, public_ip: &str) -> Result<Option<Router>> {
        Ok(fetch_opt(
            &self.conn,
            &format!("SELECT {ROUTER_COLUMNS} WHERE networks.public_ip = ?1 AND {MAIN}"),
            params![public_ip],
        )
        .await?)
    }

    /// Returns the main router with `domain`.
    pub(crate) async fn router_of_domain(&self, domain: &str) -> Result<Option<Router>> {
        Ok(fetch_opt(
            &self.conn,
            &format!(
                "SELECT {ROUTER_COLUMNS} WHERE networks.domain = ?1 COLLATE NOCASE AND networks.domain <> '' \
                 AND {MAIN} ORDER BY routers.computer_id"
            ),
            params![domain],
        )
        .await?)
    }

    /// Returns the main router with Wi-Fi `bssid` and `essid` (`ServerMap.GetRouter`).
    pub(crate) async fn router_of_wifi(&self, bssid: &str, essid: &str) -> Result<Option<Router>> {
        Ok(fetch_opt(
            &self.conn,
            &format!(
                "SELECT {ROUTER_COLUMNS} WHERE routers.bssid = ?1 COLLATE NOCASE AND routers.essid = ?2 AND {MAIN} \
                 ORDER BY routers.computer_id"
            ),
            params![bssid, essid],
        )
        .await?)
    }

    /// Returns network routers and switches with the main router first.
    pub(crate) async fn routers_of(&self, network: NetworkId) -> Result<Vec<Router>> {
        Ok(fetch_all(
            &self.conn,
            &format!(
                "SELECT {ROUTER_COLUMNS} WHERE computers.network_id = ?1 \
                 ORDER BY computers.parent_id IS NOT NULL, routers.computer_id"
            ),
            params![network.0],
        )
        .await?)
    }

    /// Returns main routers within `radius` of `map`.
    pub(crate) async fn routers_near(&self, map: [f32; 2], radius: f32) -> Result<Vec<Router>> {
        Ok(fetch_all(
            &self.conn,
            &format!(
                "SELECT {ROUTER_COLUMNS} WHERE networks.map_x BETWEEN ?1 AND ?2 AND networks.map_y BETWEEN ?3 AND ?4 \
                 AND {MAIN} ORDER BY routers.computer_id"
            ),
            params![map[0] - radius, map[0] + radius, map[1] - radius, map[1] + radius],
        )
        .await?)
    }

    pub(crate) async fn public_addresses(&self) -> Result<Vec<String>> {
        Ok(fetch_column(&self.conn, "SELECT public_ip FROM networks ORDER BY id", ()).await?)
    }

    /// Updates router network public data, Wi-Fi, firewall state, and LAN address.
    pub(crate) async fn save_router(&self, router: &Router) -> Result<()> {
        let contact = router.contact.as_ref();
        let password = self.register_password(&router.wifi_password).await?;

        execute(
            &self.conn,
            "UPDATE networks SET public_ip = ?2, domain = ?3, contact_name = ?4, contact_mail = ?5, \
             contact_phone = ?6, map_x = ?7, map_y = ?8 WHERE id = ?1",
            params![
                router.network.0,
                router.public_ip.as_str(),
                router.domain.as_str(),
                contact.map(|contact| contact.name.as_str()),
                contact.map(|contact| contact.mail.as_str()),
                contact.map(|contact| contact.phone.as_str()),
                router.map[0],
                router.map[1],
            ],
        )
        .await?;
        execute(
            &self.conn,
            "UPDATE routers SET essid = ?2, bssid = ?3, wifi_password_hash = ?4, firewall_active = ?5 \
             WHERE computer_id = ?1",
            params![
                router.id.0,
                router.essid.as_deref(),
                router.bssid.as_deref(),
                password.as_str(),
                router.firewall_active,
            ],
        )
        .await?;
        execute(
            &self.conn,
            "UPDATE computers SET local_ip = ?2 WHERE id = ?1",
            params![router.id.0, router.local_ip.as_str()],
        )
        .await?;

        Ok(())
    }

    /// Inserts `draft` as a router or switch in `network` at `place`, or as the main router for
    /// `None`, including its computer, router row, and firewall rules.
    pub(crate) async fn insert_router(
        &self,
        draft: &RouterDraft,
        network: NetworkId,
        place: Option<&LanPlace>,
    ) -> Result<RouterId> {
        let id = RouterId(
            self.insert_computer_at(&draft.computer, network, place)
                .await?
                .0,
        );
        let password = self.register_password(&draft.wifi_password).await?;

        execute(
            &self.conn,
            "INSERT INTO routers (computer_id, essid, bssid, wifi_password_hash, firewall_active) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id.0,
                draft.essid.as_deref(),
                draft.bssid.as_deref(),
                password.as_str(),
                draft.firewall_active,
            ],
        )
        .await?;

        for rule in &draft.firewall_rules {
            self.insert_firewall_rule(id, rule).await?;
        }

        Ok(id)
    }

    /// Returns `router` forwards in creation order.
    pub(crate) async fn port_forwards(&self, router: RouterId) -> Result<Vec<PortForward>> {
        Ok(fetch_all(
            &self.conn,
            &format!("SELECT {FORWARD_COLUMNS} WHERE port_forwards.router_id = ?1 ORDER BY port_forwards.rowid"),
            params![router.0],
        )
        .await?)
    }

    pub(crate) async fn port_forward(
        &self,
        router: RouterId,
        number: u16,
    ) -> Result<Option<PortForward>> {
        Ok(fetch_opt(
            &self.conn,
            &format!("SELECT {FORWARD_COLUMNS} WHERE port_forwards.router_id = ?1 AND port_forwards.number = ?2"),
            params![router.0, number],
        )
        .await?)
    }

    /// Forwards router public `number` to computer `port`. Returns whether both ports exist and the
    /// public number was available.
    pub(crate) async fn insert_port_forward(
        &self,
        router: RouterId,
        number: u16,
        computer: ComputerId,
        port: u16,
    ) -> Result<bool> {
        Ok(execute(
            &self.conn,
            "INSERT OR IGNORE INTO port_forwards (router_id, number, port_id) \
             SELECT ?1, ?2, id FROM computer_ports WHERE computer_id = ?3 AND number = ?4",
            params![router.0, number, computer.0, port],
        )
        .await?
            > 0)
    }

    /// Removes `router` forwards targeting `computer`.
    pub(crate) async fn remove_port_forwards_to(
        &self,
        router: RouterId,
        computer: ComputerId,
    ) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM port_forwards WHERE router_id = ?1 AND port_id IN \
             (SELECT id FROM computer_ports WHERE computer_id = ?2)",
            params![router.0, computer.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn firewall_rules(&self, router: RouterId) -> Result<Vec<FirewallRule>> {
        Ok(fetch_all(
            &self.conn,
            "SELECT allow, port, source, destination FROM router_firewall_rules WHERE router_id = ?1 ORDER BY id",
            params![router.0],
        )
        .await?)
    }

    pub(crate) async fn insert_firewall_rule(
        &self,
        router: RouterId,
        rule: &FirewallRule,
    ) -> Result<()> {
        execute(
            &self.conn,
            "INSERT INTO router_firewall_rules (router_id, allow, port, source, destination) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                router.0,
                rule.allow,
                rule.port,
                rule.source.as_deref(),
                rule.destination.as_deref(),
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn clear_firewall(&self, router: RouterId) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM router_firewall_rules WHERE router_id = ?1",
            params![router.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn reverse_shells(&self, router: RouterId) -> Result<Vec<ReverseShell>> {
        Ok(fetch_all(
            &self.conn,
            "SELECT computer_id, port, user, pid, created_at FROM reverse_shells WHERE router_id = ?1 ORDER BY id",
            params![router.0],
        )
        .await?)
    }

    /// Stores `shell` keyed by computer and process, replacing any prior shell for that process
    /// (`RouterConfigOS.AddReverseShell`). Ending the client process removes it.
    pub(crate) async fn insert_reverse_shell(
        &self,
        router: RouterId,
        shell: &ReverseShell,
    ) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM reverse_shells WHERE router_id = ?1 AND computer_id = ?2 AND pid = ?3",
            params![router.0, shell.computer.0, shell.pid],
        )
        .await?;
        execute(
            &self.conn,
            "INSERT INTO reverse_shells (router_id, computer_id, pid, port, user, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                router.0,
                shell.computer.0,
                shell.pid,
                shell.port,
                shell.user.as_str(),
                super::computers::seconds(shell.creation_second)
            ],
        )
        .await?;

        Ok(())
    }

    /// Returns `network` traffic-camera state or defaults if unset.
    pub(crate) async fn traffic(&self, network: NetworkId) -> Result<TrafficNet> {
        Ok(fetch_opt(
            &self.conn,
            "SELECT credentials_id, override_enabled, override_used FROM traffic_networks WHERE network_id = ?1",
            params![network.0],
        )
        .await?
        .unwrap_or_default())
    }

    /// Updates `network` traffic-camera state.
    pub(crate) async fn set_traffic(&self, network: NetworkId, traffic: &TrafficNet) -> Result<()> {
        execute(
            &self.conn,
            "INSERT INTO traffic_networks (network_id, credentials_id, override_enabled, override_used) \
             VALUES (?1, ?2, ?3, ?4) ON CONFLICT (network_id) DO UPDATE SET credentials_id = ?2, \
             override_enabled = ?3, override_used = ?4",
            params![
                network.0,
                traffic.credentials.map(|person| person.0),
                traffic.override_enabled,
                traffic.override_used,
            ],
        )
        .await?;

        Ok(())
    }
}
