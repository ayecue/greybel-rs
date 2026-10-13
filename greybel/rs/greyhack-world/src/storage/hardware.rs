//! Shared hardware-part rows and per-computer installation rows built by `parts_query`.

use greyhack_behaviour::HardwareKind;
use turso::{Value, params};

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_all, fetch_value, insert};
use crate::{
    ComputerId, Cpu, Gpu, HardDisk, Hardware, Motherboard, NetCard, Part, PowerSupply, Ram, Result,
};

/// Values built by `params!` for statements executed twice.
fn values<const N: usize>(params: [turso::Result<Value>; N]) -> turso::Result<Vec<Value>> {
    params.into_iter().collect()
}

/// Returns installed parts of `kind` with installation health, in installation order.
fn parts_query(table: &str, columns: &str, kind: HardwareKind) -> String {
    format!(
        "SELECT {table}.name, {table}.quality, {columns}, computer_hardware.health FROM computer_hardware \
         JOIN {table} ON {table}.id = computer_hardware.hardware_id \
         WHERE computer_hardware.computer_id = ?1 AND computer_hardware.kind = {} ORDER BY computer_hardware.id",
        kind.code()
    )
}

fn part(row: &RowView<'_>) -> turso::Result<Part> {
    Ok(Part {
        name: row.get("name")?,
        quality: row.get("quality")?,
        required_power: row.get::<Option<f32>>("required_power")?.unwrap_or(0.0),
        health: row.get("health")?,
    })
}

impl FromRow for HardDisk {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            part: part(row)?,
            speed: row.get("speed")?,
            size_mb: row.get("size_mb")?,
        })
    }
}

impl FromRow for Cpu {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            part: part(row)?,
            speed: row.get("speed")?,
            cores: row.get("cores")?,
            socket: row.get("socket")?,
        })
    }
}

impl FromRow for PowerSupply {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            part: part(row)?,
            power: row.get("power")?,
        })
    }
}

impl FromRow for Motherboard {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            part: part(row)?,
            cpu_slots: row.get("cpu_slots")?,
            cpu_socket: row.get("cpu_socket")?,
            ram_model: row.get("ram_model")?,
            ram_slots: row.get("ram_slots")?,
            max_ram_mb: row.get("max_ram_mb")?,
            pci_slots: row.get("pci_slots")?,
        })
    }
}

impl FromRow for Ram {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            part: part(row)?,
            memory_mb: row.get("memory_mb")?,
            model: row.get("model")?,
        })
    }
}

impl FromRow for NetCard {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            part: part(row)?,
            wifi: row.get("wifi")?,
            monitor_support: row.get("monitor_support")?,
            monitor_enabled: row.get("monitor_enabled")?,
            active: row.get("active")?,
        })
    }
}

impl FromRow for Gpu {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            part: part(row)?,
            hashrate: row.get("hashrate")?,
        })
    }
}

impl Database {
    /// Returns every part installed in `computer`.
    pub(crate) async fn hardware(&self, computer: ComputerId) -> Result<Hardware> {
        let hard_disks = fetch_all(
            &self.conn,
            &parts_query(
                "hard_disks",
                "hard_disks.required_power, hard_disks.speed, hard_disks.size_mb",
                HardwareKind::HardDisk,
            ),
            params![computer.0],
        )
        .await?;
        let cpus = fetch_all(
            &self.conn,
            &parts_query(
                "cpus",
                "cpus.required_power, cpus.speed, cpus.cores, cpus.socket",
                HardwareKind::Cpu,
            ),
            params![computer.0],
        )
        .await?;
        let power_supplies: Vec<PowerSupply> = fetch_all(
            &self.conn,
            &parts_query(
                "power_supplies",
                "NULL AS required_power, power_supplies.power",
                HardwareKind::PowerSupply,
            ),
            params![computer.0],
        )
        .await?;
        let motherboards: Vec<Motherboard> = fetch_all(
            &self.conn,
            &parts_query(
                "motherboards",
                "motherboards.required_power, motherboards.cpu_slots, motherboards.cpu_socket, \
                 motherboards.ram_model, motherboards.ram_slots, motherboards.max_ram_mb, motherboards.pci_slots",
                HardwareKind::Motherboard,
            ),
            params![computer.0],
        )
        .await?;
        let rams = fetch_all(
            &self.conn,
            &parts_query(
                "rams",
                "rams.required_power, rams.memory_mb, rams.model",
                HardwareKind::Ram,
            ),
            params![computer.0],
        )
        .await?;
        let gpus = fetch_all(
            &self.conn,
            &parts_query(
                "gpus",
                "gpus.required_power, gpus.hashrate",
                HardwareKind::Gpu,
            ),
            params![computer.0],
        )
        .await?;

        Ok(Hardware {
            motherboard: motherboards.into_iter().next(),
            power_supply: power_supplies.into_iter().next(),
            cpus,
            rams,
            gpus,
            hard_disks,
            net_cards: self.net_cards(computer).await?,
        })
    }

    /// Returns network cards in installation order, which determines interface names.
    pub(crate) async fn net_cards(&self, computer: ComputerId) -> Result<Vec<NetCard>> {
        Ok(fetch_all(
            &self.conn,
            &format!(
                "SELECT net_cards.name, net_cards.quality, net_cards.required_power, net_cards.wifi, \
                 net_cards.monitor_support, computer_net_cards.active, computer_net_cards.monitor_enabled, \
                 computer_hardware.health FROM computer_hardware \
                 JOIN net_cards ON net_cards.id = computer_hardware.hardware_id \
                 JOIN computer_net_cards ON computer_net_cards.computer_hardware_id = computer_hardware.id \
                 WHERE computer_hardware.computer_id = ?1 AND computer_hardware.kind = {} ORDER BY computer_hardware.id",
                HardwareKind::NetCard.code()
            ),
            params![computer.0],
        )
        .await?)
    }

    /// Updates active and monitor-mode flags for `computer` network card `index`.
    pub(crate) async fn save_net_card(
        &self,
        computer: ComputerId,
        index: usize,
        card: &NetCard,
    ) -> Result<()> {
        execute(
            &self.conn,
            "UPDATE computer_net_cards SET active = ?3, monitor_enabled = ?4 WHERE computer_hardware_id = \
             (SELECT id FROM computer_hardware WHERE computer_id = ?1 AND kind = ?5 ORDER BY id LIMIT 1 OFFSET ?2)",
            params![
                computer.0,
                i64::try_from(index).unwrap_or(i64::MAX),
                card.active,
                card.monitor_enabled,
                HardwareKind::NetCard.code(),
            ],
        )
        .await?;

        Ok(())
    }

    /// Installs `card` after existing network cards (`HardwareHelperServer.InstallHardware`).
    pub(crate) async fn insert_net_card(&self, computer: ComputerId, card: &NetCard) -> Result<()> {
        let id = self
            .part_row(
                "net_cards",
                "name, quality, required_power, wifi, monitor_support",
                values(params![
                    card.part.name.as_str(),
                    card.part.quality,
                    card.part.required_power,
                    card.wifi,
                    card.monitor_support
                ])?,
            )
            .await?;
        let installation = self
            .install(computer, HardwareKind::NetCard, id, &card.part)
            .await?;

        execute(
            &self.conn,
            "INSERT INTO computer_net_cards (computer_hardware_id, active, monitor_enabled) VALUES (?1, ?2, ?3)",
            params![installation, card.active, card.monitor_enabled],
        )
        .await?;

        Ok(())
    }

    /// Installs all `hardware` parts in `computer`.
    pub(crate) async fn insert_hardware(
        &self,
        computer: ComputerId,
        hardware: &Hardware,
    ) -> Result<()> {
        for disk in &hardware.hard_disks {
            let id = self
                .part_row(
                    "hard_disks",
                    "name, quality, required_power, speed, size_mb",
                    values(params![
                        disk.part.name.as_str(),
                        disk.part.quality,
                        disk.part.required_power,
                        disk.speed,
                        disk.size_mb
                    ])?,
                )
                .await?;

            self.install(computer, HardwareKind::HardDisk, id, &disk.part)
                .await?;
        }

        for cpu in &hardware.cpus {
            let id = self
                .part_row(
                    "cpus",
                    "name, quality, required_power, speed, cores, socket",
                    values(params![
                        cpu.part.name.as_str(),
                        cpu.part.quality,
                        cpu.part.required_power,
                        cpu.speed,
                        cpu.cores,
                        cpu.socket.as_str()
                    ])?,
                )
                .await?;

            self.install(computer, HardwareKind::Cpu, id, &cpu.part)
                .await?;
        }

        if let Some(supply) = &hardware.power_supply {
            let id = self
                .part_row(
                    "power_supplies",
                    "name, quality, power",
                    values(params![
                        supply.part.name.as_str(),
                        supply.part.quality,
                        supply.power
                    ])?,
                )
                .await?;

            self.install(computer, HardwareKind::PowerSupply, id, &supply.part)
                .await?;
        }

        if let Some(board) = &hardware.motherboard {
            let id = self
                .part_row(
                    "motherboards",
                    "name, quality, required_power, cpu_slots, cpu_socket, ram_model, ram_slots, max_ram_mb, \
                     pci_slots",
                    values(params![
                        board.part.name.as_str(),
                        board.part.quality,
                        board.part.required_power,
                        board.cpu_slots,
                        board.cpu_socket.as_str(),
                        board.ram_model,
                        board.ram_slots,
                        board.max_ram_mb,
                        board.pci_slots
                    ])?,
                )
                .await?;

            self.install(computer, HardwareKind::Motherboard, id, &board.part)
                .await?;
        }

        for ram in &hardware.rams {
            let id = self
                .part_row(
                    "rams",
                    "name, quality, required_power, memory_mb, model",
                    values(params![
                        ram.part.name.as_str(),
                        ram.part.quality,
                        ram.part.required_power,
                        ram.memory_mb,
                        ram.model
                    ])?,
                )
                .await?;

            self.install(computer, HardwareKind::Ram, id, &ram.part)
                .await?;
        }

        for card in &hardware.net_cards {
            self.insert_net_card(computer, card).await?;
        }

        for gpu in &hardware.gpus {
            let id = self
                .part_row(
                    "gpus",
                    "name, quality, required_power, hashrate",
                    values(params![
                        gpu.part.name.as_str(),
                        gpu.part.quality,
                        gpu.part.required_power,
                        gpu.hashrate
                    ])?,
                )
                .await?;

            self.install(computer, HardwareKind::Gpu, id, &gpu.part)
                .await?;
        }

        Ok(())
    }

    /// Returns the row ID in `table` matching `columns` and `values`, inserting the row if absent.
    async fn part_row(&self, table: &str, columns: &str, values: Vec<Value>) -> Result<i64> {
        let count = columns.split(',').count();
        let marks = (1..=count)
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let matches = columns
            .split(',')
            .zip(1..)
            .map(|(column, index)| format!("{} = ?{index}", column.trim()))
            .collect::<Vec<_>>()
            .join(" AND ");

        execute(
            &self.conn,
            &format!("INSERT OR IGNORE INTO {table} ({columns}) VALUES ({marks})"),
            values.clone(),
        )
        .await?;

        Ok(fetch_value(
            &self.conn,
            &format!("SELECT id FROM {table} WHERE {matches}"),
            values,
        )
        .await?
        .expect("the part row was just inserted"))
    }

    /// Installs shared part row `hardware` of `kind` with `part` health and returns the
    /// installation ID.
    async fn install(
        &self,
        computer: ComputerId,
        kind: HardwareKind,
        hardware: i64,
        part: &Part,
    ) -> Result<i64> {
        Ok(insert(
            &self.conn,
            "INSERT INTO computer_hardware (computer_id, kind, hardware_id, health) VALUES (?1, ?2, ?3, ?4)",
            params![computer.0, kind.code(), hardware, part.health],
        )
        .await?)
    }
}
