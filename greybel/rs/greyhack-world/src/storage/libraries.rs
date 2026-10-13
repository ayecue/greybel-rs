//! Libraries keyed by kind and version, with zones, weaknesses, requirements, and references from
//! files, ports, and router kernels. Tracks the newest version of each kind.

use std::cmp::Ordering;

use greyhack_behaviour::{LibraryKind, Requirement, compare_versions};
use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_all, fetch_column, fetch_value, insert};
use crate::{Library, LibraryId, MemoryZone, NetworkId, NodeId, Result, Weakness};

struct ZoneRow {
    id: i64,
    address: String,
    hide: bool,
}

impl FromRow for ZoneRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            address: row.get("address")?,
            hide: row.get("hide")?,
        })
    }
}

struct WeaknessRow {
    id: i64,
    weakness: Weakness,
}

impl FromRow for WeaknessRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            weakness: Weakness {
                remote: row.get("remote")?,
                unsec_value: row.get("unsec_value")?,
                details: row.get("details")?,
                reward: row.get("reward")?,
                user: row.get("user")?,
                required_lib: row.get("required_library")?,
                req_lib_version: row.get("required_version")?,
                metaxploit_version: row.get("metaxploit_version")?,
                num_register_users: row.get("num_register_users")?,
                num_port_forward: row.get("num_port_forward")?,
                num_conn_gateway: row.get("num_conn_gateway")?,
                path_exist: row.get("path_exist")?,
                random_path: row.get("random_path")?,
                requirements: Vec::new(),
            },
        })
    }
}

impl Database {
    /// Returns library `id` with zones and weaknesses.
    pub(crate) async fn library(&self, id: LibraryId) -> Result<Option<Library>> {
        let head: Option<(LibraryKind, String)> = fetch_all(
            &self.conn,
            "SELECT kind, version FROM libraries WHERE id = ?1",
            params![id.0],
        )
        .await?
        .into_iter()
        .next();
        let Some((kind, version)) = head else {
            return Ok(None);
        };
        let mut zones = Vec::new();

        for zone in fetch_all::<ZoneRow>(
            &self.conn,
            "SELECT id, address, hide FROM memory_zones WHERE library_id = ?1 ORDER BY id",
            params![id.0],
        )
        .await?
        {
            let mut weaknesses = Vec::new();

            for mut row in fetch_all::<WeaknessRow>(
                &self.conn,
                "SELECT id, remote, unsec_value, details, reward, user, required_library, required_version, \
                 metaxploit_version, num_register_users, num_port_forward, num_conn_gateway, path_exist, random_path \
                 FROM weaknesses WHERE zone_id = ?1 ORDER BY id",
                params![zone.id],
            )
            .await?
            {
                row.weakness.requirements = fetch_column::<Requirement>(
                    &self.conn,
                    "SELECT requirement FROM weakness_requirements WHERE weakness_id = ?1 ORDER BY id",
                    params![row.id],
                )
                .await?;
                weaknesses.push(row.weakness);
            }

            zones.push(MemoryZone {
                address: zone.address,
                hide: zone.hide,
                weaknesses,
            });
        }

        Ok(Some(Library {
            kind,
            version,
            zones,
        }))
    }

    /// Returns library `kind` at `version`.
    pub(crate) async fn library_of(
        &self,
        kind: LibraryKind,
        version: &str,
    ) -> Result<Option<LibraryId>> {
        Ok(fetch_value::<u32>(
            &self.conn,
            "SELECT id FROM libraries WHERE kind = ?1 AND version = ?2",
            params![kind.code(), version],
        )
        .await?
        .map(LibraryId))
    }

    /// Returns the newest library of `kind`.
    pub(crate) async fn latest_library(&self, kind: LibraryKind) -> Result<Option<LibraryId>> {
        Ok(fetch_value::<u32>(
            &self.conn,
            "SELECT library_id FROM library_latest WHERE kind = ?1",
            params![kind.code()],
        )
        .await?
        .map(LibraryId))
    }

    /// Returns the stored library matching `library` kind and version, or inserts `library` with
    /// its zones if absent.
    pub(crate) async fn library_id(&self, library: &Library) -> Result<LibraryId> {
        if let Some(id) = self.library_of(library.kind, &library.version).await? {
            return Ok(id);
        }

        let id = self
            .insert_library_head(library.kind, &library.version)
            .await?;

        for zone in &library.zones {
            let zone_id = insert(
                &self.conn,
                "INSERT INTO memory_zones (library_id, address, hide) VALUES (?1, ?2, ?3)",
                params![id.0, zone.address.as_str(), zone.hide],
            )
            .await?;

            for weakness in &zone.weaknesses {
                let weakness_id = insert(
                    &self.conn,
                    "INSERT INTO weaknesses (zone_id, unsec_value, details, reward, user, required_library, \
                     required_version, metaxploit_version, num_register_users, num_port_forward, num_conn_gateway, \
                     path_exist, random_path, remote) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        zone_id,
                        weakness.unsec_value.as_str(),
                        weakness.details.as_str(),
                        weakness.reward.code(),
                        weakness.user.as_str(),
                        weakness.required_lib.map(LibraryKind::code),
                        weakness.req_lib_version.as_str(),
                        weakness.metaxploit_version.as_str(),
                        weakness.num_register_users,
                        weakness.num_port_forward,
                        weakness.num_conn_gateway,
                        weakness.path_exist.as_str(),
                        weakness.random_path.as_str(),
                        weakness.remote,
                    ],
                )
                .await?;

                for requirement in &weakness.requirements {
                    execute(
                        &self.conn,
                        "INSERT OR IGNORE INTO weakness_requirements (weakness_id, requirement) VALUES (?1, ?2)",
                        params![weakness_id, requirement.code()],
                    )
                    .await?;
                }
            }
        }

        Ok(id)
    }

    /// Returns library `kind` at `version`, inserting it without weaknesses if absent.
    pub(crate) async fn library_row(&self, kind: LibraryKind, version: &str) -> Result<LibraryId> {
        match self.library_of(kind, version).await? {
            Some(id) => Ok(id),
            None => self.insert_library_head(kind, version).await,
        }
    }

    /// Inserts library `kind` at `version` and marks it newest unless a newer version exists.
    async fn insert_library_head(&self, kind: LibraryKind, version: &str) -> Result<LibraryId> {
        let id = insert(
            &self.conn,
            "INSERT INTO libraries (kind, version) VALUES (?1, ?2)",
            params![kind.code(), version],
        )
        .await?;
        let id = LibraryId(u32::try_from(id).expect("library ids fit u32"));
        let latest: Option<String> = fetch_value(
            &self.conn,
            "SELECT libraries.version FROM library_latest JOIN libraries ON libraries.id = library_latest.library_id \
             WHERE library_latest.kind = ?1",
            params![kind.code()],
        )
        .await?;

        if latest.is_none_or(|latest| compare_versions(version, &latest) == Ordering::Greater) {
            execute(
                &self.conn,
                "INSERT INTO library_latest (kind, library_id) VALUES (?1, ?2) \
                 ON CONFLICT (kind) DO UPDATE SET library_id = ?2",
                params![kind.code(), id.0],
            )
            .await?;
        }

        Ok(id)
    }

    /// Returns the library associated with file `node`.
    pub(crate) async fn node_library(&self, node: NodeId) -> Result<Option<LibraryId>> {
        Ok(fetch_value::<u32>(
            &self.conn,
            "SELECT library_id FROM node_libraries WHERE node_id = ?1",
            params![node.0],
        )
        .await?
        .map(LibraryId))
    }

    /// Returns the libraries reported as the newest of their kind for `network`
    /// (`NetworkLan.reportedLibUpdates`).
    pub(crate) async fn network_library_updates(
        &self,
        network: NetworkId,
    ) -> Result<Vec<(LibraryKind, LibraryId)>> {
        Ok(fetch_all::<(LibraryKind, u32)>(
            &self.conn,
            "SELECT kind, library_id FROM network_library_updates WHERE network_id = ?1 ORDER BY kind",
            params![network.0],
        )
        .await?
        .into_iter()
        .map(|(kind, library)| (kind, LibraryId(library)))
        .collect())
    }

    /// Reports `library` as the newest of its `kind` for `network`, replacing an earlier report
    /// (`NetworkLan.RegisterReportedLibUpdate`).
    pub(crate) async fn set_network_library_update(
        &self,
        network: NetworkId,
        kind: LibraryKind,
        library: LibraryId,
    ) -> Result<()> {
        execute(
            &self.conn,
            "INSERT INTO network_library_updates (network_id, kind, library_id) VALUES (?1, ?2, ?3) \
             ON CONFLICT (network_id, kind) DO UPDATE SET library_id = ?3",
            params![network.0, kind.code(), library.0],
        )
        .await?;

        Ok(())
    }

    /// Associates `library` with file `node`, or removes the association.
    pub(crate) async fn set_node_library(
        &self,
        node: NodeId,
        library: Option<LibraryId>,
    ) -> Result<()> {
        match library {
            Some(library) => {
                execute(
                    &self.conn,
                    "INSERT INTO node_libraries (node_id, library_id) VALUES (?1, ?2) \
                 ON CONFLICT (node_id) DO UPDATE SET library_id = ?2",
                    params![node.0, library.0],
                )
                .await?
            }
            None => {
                execute(
                    &self.conn,
                    "DELETE FROM node_libraries WHERE node_id = ?1",
                    params![node.0],
                )
                .await?
            }
        };

        Ok(())
    }
}
