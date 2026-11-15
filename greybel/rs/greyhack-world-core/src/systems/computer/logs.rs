//! System logs (`/var/system.log`): the entries that connections, logins, deletions and exploits write on the
//! computers they reach (`PlayerUtils.WriteLog`, `PlayerUtils.UpdateBounceLog`).

use std::rc::Rc;

use greyhack_behaviour::{
    FileData, FileKind, LogAction, LogEntry, PermissionSet, ROOT_USER, SYSTEM_LOG_FOLDER,
    SYSTEM_LOG_FOLDER_PERMISSIONS, SYSTEM_LOG_NAME, SYSTEM_LOG_PERMISSIONS, SystemLog, format_date,
    is_device, player_net_id, template_timestamp, token_trace,
};
use greyhack_world_modules::Dep;

use crate::systems::{FileChangeService, FileLookup, LogService as LogsService, NetworkTopology};
use crate::{ComputerId, Node, NodeDraft, Result, WorldRef};

/// What one log entry says besides the time and the player (`PlayerUtils.WriteLog` parameters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogWrite<'a> {
    pub action: LogAction,
    /// Address of the origin (`origIp`).
    pub ip: &'a str,
    pub port: u16,
    /// The path a `remove` entry names (`nombreFile`).
    pub file: &'a str,
    /// Address the connection went on to (`bounceIp`).
    pub bounce_ip: &'a str,
}

impl LogsService {
    /// Returns the ID the system log entries name the player by (`GreyInterpreter.GetPlayerID`).
    #[must_use]
    pub fn player_net_id(&self) -> String {
        player_net_id(self.world.seed())
    }

    /// Returns the trace token the system log entries carry before any game over by a passive trace
    /// (`Database.GetPlayerTokenTrace`).
    #[must_use]
    pub fn token_trace(&self) -> String {
        token_trace(self.world.seed(), 0)
    }

    /// Appends an entry to the system log of `computer`, creating `/var` and `/var/system.log` when they are
    /// missing, and dates the log from the entry (`PlayerUtils.WriteLog`, `Archivo.SetContenido`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn write_log(&self, computer: ComputerId, write: &LogWrite<'_>) -> Result<()> {
        let Some(root) = self.lookup.root(computer).await? else {
            return Ok(());
        };
        let var = match self
            .lookup
            .folder(computer, SYSTEM_LOG_FOLDER, true)
            .await?
        {
            Some(var) => var,
            None => self.create_log_folder(&root).await?,
        };
        let current = self
            .lookup
            .child(&var, SYSTEM_LOG_NAME)
            .await?
            .filter(|node| {
                node.data()
                    .is_some_and(|data| data.kind == FileKind::SystemLog)
            });
        let mut log = match current {
            Some(log) => log,
            None => {
                let mut data = FileData::binary(FileData::binary_size(SYSTEM_LOG_NAME, false));

                data.kind = FileKind::SystemLog;

                let draft = NodeDraft::file(
                    SYSTEM_LOG_NAME,
                    ROOT_USER,
                    PermissionSet::parse(SYSTEM_LOG_PERMISSIONS).expect("valid permissions"),
                    data,
                );

                self.changes.add_node(&var, &draft).await?
            }
        };
        let data = log.data_mut().expect("the log is a file");
        let mut system_log = SystemLog::parse(&data.content).unwrap_or_default();

        system_log.add(LogEntry {
            action: write.action,
            ip: write.ip.to_owned(),
            fecha: format_date(self.world.time_seconds()),
            puerto: i64::from(write.port),
            file: write.file.to_owned(),
            bounce_ip: write.bounce_ip.to_owned(),
            player_net_id: self.player_net_id(),
            tutorial: false,
            token_trace: self.traces.player_token_trace(&self.world).await?,
        });
        data.set_content(&system_log.to_json());
        log.timestamp = self.world.time_seconds();

        self.changes.save_node(&log).await
    }

    /// Writes the `open` entry of a login or net session on `target` from the address `origin`
    /// (`PlayerUtils.ConnectToService`, `MetaExploitIntrinsics.net_use`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn write_open_log(&self, target: ComputerId, origin: &str, port: u16) -> Result<()> {
        self.write_log(
            target,
            &LogWrite {
                action: LogAction::Open,
                ip: origin,
                port,
                file: "",
                bounce_ip: "",
            },
        )
        .await
    }

    /// Creates the `/var` folder, dated like any generated entry of its parent (`FileSystem.ForceAddFolder`,
    /// `Fichero.InitializeTimestamp`).
    async fn create_log_folder(&self, root: &Node) -> Result<Node> {
        let draft = NodeDraft::folder(
            "var",
            ROOT_USER,
            PermissionSet::parse(SYSTEM_LOG_FOLDER_PERMISSIONS).expect("valid permissions"),
        );
        let mut var = self.changes.add_node(root, &draft).await?;

        if root.timestamp != 0 {
            var.timestamp = template_timestamp(root.timestamp, "var");
            self.changes.save_node(&var).await?;
        }

        Ok(var)
    }

    /// Writes the three entries of a shell the player opened on `bounce` from `orig`: `shell` on `bounce`, and
    /// `bounce` on `orig` and on the router of `orig`. `prev_ip` is the address the player came to `orig` from
    /// (`PlayerUtils.UpdateBounceLog`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn write_bounce_logs(
        &self,
        prev_ip: &str,
        port: u16,
        orig: ComputerId,
        bounce: ComputerId,
    ) -> Result<()> {
        let (Some(orig_row), Some(_)) = (
            self.topology.computer(orig).await?,
            self.topology.computer(bounce).await?,
        ) else {
            return Ok(());
        };
        let orig_public = self.topology.public_ip(orig).await?;
        let bounce_public = self.topology.public_ip(bounce).await?;

        self.write_log(
            bounce,
            &LogWrite {
                action: LogAction::Shell,
                ip: &orig_public,
                port,
                file: "",
                bounce_ip: "",
            },
        )
        .await?;
        self.write_log(
            orig,
            &LogWrite {
                action: LogAction::Bounce,
                ip: prev_ip,
                port: 0,
                file: "",
                bounce_ip: &bounce_public,
            },
        )
        .await?;

        // `Computer.GetRouter` of a router or switch is the device itself.
        let router = if orig_row.router.is_none() && is_device(orig_row.kind) {
            Some(orig)
        } else {
            self.topology
                .router_of(orig)
                .await?
                .map(|router| router.id.computer())
        };

        if let Some(router) = router {
            self.write_log(
                router,
                &LogWrite {
                    action: LogAction::Bounce,
                    ip: &orig_row.local_ip,
                    port: 0,
                    file: "",
                    bounce_ip: &bounce_public,
                },
            )
            .await?;
        }

        Ok(())
    }
}

/// Computer system-log rules.
pub struct LogService {
    pub(crate) world: WorldRef,
    pub traces: Dep<dyn crate::PlayerTraceService>,
    pub(crate) changes: Dep<FileChangeService>,
    pub(crate) lookup: Dep<FileLookup>,
    pub(crate) topology: Dep<NetworkTopology>,
}

impl LogService {
    pub(crate) fn new(registry: &Rc<greyhack_world_modules::Registry>) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            traces: registry.get(),
            changes: registry.get(),
            lookup: registry.get(),
            topology: registry.get(),
        }
    }
}
