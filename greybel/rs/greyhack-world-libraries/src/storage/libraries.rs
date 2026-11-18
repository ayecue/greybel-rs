//! Library zones, weaknesses, requirements, and per-network update reports.

use std::cell::RefCell;
use std::collections::HashMap;

use greyhack_behaviour::{LibraryKind, Requirement};
use greyhack_world_core::{ComputerId, Database, LanPlace, LibraryId, NetworkId, Result, RouterId};
use greyhack_world_storage::sql::{
    Code, FromRow, RowView, execute, fetch_all, fetch_column, insert,
};
use turso::params;

use crate::{ComputerDraft, FileSystemDraft, Library, MemoryZone, RouterDraft, Weakness};

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
                reward: row.get_code("reward")?,
                user: row.get("user")?,
                required_lib: row.get_code_opt("required_library")?,
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

/// Queries of [`Database`] for library exploit content and update reports.
/// Queries of the tables owned by this area.
pub(crate) struct LibraryRepository {
    db: Database,
    cache: RefCell<Option<HashMap<LibraryId, Library>>>,
    computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
}

/// Enables the full-library cache only for one initial-history operation. Dropping it disables and empties the cache,
/// including when generation returns an error or panics.
pub(crate) struct LibraryCacheScope<'a> {
    repository: &'a LibraryRepository,
}

impl Drop for LibraryCacheScope<'_> {
    fn drop(&mut self) {
        *self.repository.cache.borrow_mut() = None;
    }
}

impl LibraryRepository {
    pub(crate) fn new(
        db: Database,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    ) -> Self {
        Self {
            db,
            cache: RefCell::new(None),
            computers,
            files,
            networks,
        }
    }

    pub(crate) fn cache_scope(&self) -> LibraryCacheScope<'_> {
        assert!(
            self.cache.borrow().is_none(),
            "initial library history is not reentrant"
        );
        *self.cache.borrow_mut() = Some(HashMap::new());

        LibraryCacheScope { repository: self }
    }

    pub(crate) async fn library(&self, id: LibraryId) -> Result<Option<Library>> {
        if let Some(library) = self
            .cache
            .borrow()
            .as_ref()
            .and_then(|cache| cache.get(&id))
            .cloned()
        {
            return Ok(Some(library));
        }

        let head: Option<(Code<LibraryKind>, String)> = fetch_all(
            &self.db.connection(),
            "SELECT kind, version FROM libraries WHERE id = ?1",
            params![id.0],
        )
        .await?
        .into_iter()
        .next();
        let Some((Code(kind), version)) = head else {
            return Ok(None);
        };
        let mut zones = Vec::new();

        for zone in fetch_all::<ZoneRow>(
            &self.db.connection(),
            "SELECT id, address, hide FROM memory_zones WHERE library_id = ?1 ORDER BY id",
            params![id.0],
        )
        .await?
        {
            let mut weaknesses = Vec::new();

            for mut row in fetch_all::<WeaknessRow>(
                &self.db.connection(),
                "SELECT id, remote, unsec_value, details, reward, user, required_library, required_version, \
                 metaxploit_version, num_register_users, num_port_forward, num_conn_gateway, path_exist, random_path \
                 FROM weaknesses WHERE zone_id = ?1 ORDER BY id",
                params![zone.id],
            )
            .await?
            {
                row.weakness.requirements = fetch_column::<Code<Requirement>>(
                    &self.db.connection(),
                    "SELECT requirement FROM weakness_requirements WHERE weakness_id = ?1 ORDER BY id",
                    params![row.id],
                )
                .await?
                .into_iter()
                .map(|Code(requirement)| requirement)
                .collect();
                weaknesses.push(row.weakness);
            }

            zones.push(MemoryZone {
                address: zone.address,
                hide: zone.hide,
                weaknesses,
            });
        }

        let library = Library {
            kind,
            version,
            zones,
        };
        if let Some(cache) = self.cache.borrow_mut().as_mut() {
            cache.insert(id, library.clone());
        }

        Ok(Some(library))
    }

    /// Returns the stored library matching `library` kind and version, or inserts `library` with
    /// its zones if absent.
    pub(crate) async fn library_id(&self, library: &Library) -> Result<LibraryId> {
        if let Some(id) = self
            .files
            .library_of(library.kind, &library.version)
            .await?
        {
            return Ok(id);
        }

        let identity = greyhack_world_core::Library {
            kind: library.kind,
            version: library.version.clone(),
        };
        let id = self
            .files
            .library_rows
            .library_identity_id(&identity)
            .await?;

        self.insert_library_zones(id, library).await?;
        self.cache_stored_library(id, library);

        Ok(id)
    }

    /// Caches exactly what the uniqueness constraints leave in the database.
    fn cache_stored_library(&self, id: LibraryId, library: &Library) {
        let canonical = library
            .zones
            .iter()
            .flat_map(|zone| &zone.weaknesses)
            .all(|weakness| {
                weakness
                    .requirements
                    .iter()
                    .enumerate()
                    .all(|(index, requirement)| {
                        !weakness.requirements[index + 1..].contains(requirement)
                    })
            });
        let mut cache = self.cache.borrow_mut();
        let Some(cache) = cache.as_mut() else {
            return;
        };

        cache.remove(&id);
        if canonical {
            cache.insert(id, library.clone());
        }
    }

    /// Inserts library content in deterministic row order.
    async fn insert_library_zones(&self, id: LibraryId, library: &Library) -> Result<()> {
        for zone in &library.zones {
            let zone_id = insert(
                &self.db.connection(),
                "INSERT INTO memory_zones (library_id, address, hide) VALUES (?1, ?2, ?3)",
                params![id.0, zone.address.as_str(), zone.hide],
            )
            .await?;

            for weakness in &zone.weaknesses {
                let weakness_id = insert(
                    &self.db.connection(),
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
                        &self.db.connection(),
                        "INSERT OR IGNORE INTO weakness_requirements (weakness_id, requirement) VALUES (?1, ?2)",
                        params![weakness_id, requirement.code()],
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// Replaces the zones of an existing library identity, as unlocking a zero-day does for its selected file.
    pub(crate) async fn replace_library_zones(
        &self,
        id: LibraryId,
        library: &Library,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM memory_zones WHERE library_id = ?1",
            params![id.0],
        )
        .await?;

        self.insert_library_zones(id, library).await?;
        self.cache_stored_library(id, library);

        Ok(())
    }

    /// Returns the libraries reported as the newest of their kind for `network`
    /// (`NetworkLan.reportedLibUpdates`).
    pub(crate) async fn network_library_updates(
        &self,
        network: NetworkId,
    ) -> Result<Vec<(LibraryKind, LibraryId)>> {
        Ok(fetch_all::<(Code<LibraryKind>, u32)>(
            &self.db.connection(),
            "SELECT kind, library_id FROM network_library_updates WHERE network_id = ?1 ORDER BY kind",
            params![network.0],
        )
        .await?
        .into_iter()
        .map(|(Code(kind), library)| (kind, LibraryId(library)))
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
            &self.db.connection(),
            "INSERT INTO network_library_updates (network_id, kind, library_id) VALUES (?1, ?2, ?3) \
             ON CONFLICT (network_id, kind) DO UPDATE SET library_id = ?3",
            params![network.0, kind.code(), library.0],
        )
        .await?;

        Ok(())
    }

    /// Returns the saved library selected by each kind's router node (`HelperLibVersions.libVersions`).
    pub(crate) async fn network_library_choices(
        &self,
        network: NetworkId,
    ) -> Result<Vec<(LibraryKind, LibraryId)>> {
        Ok(fetch_all::<(Code<LibraryKind>, u32)>(
            &self.db.connection(),
            "SELECT kind, library_id FROM network_library_choices WHERE network_id = ?1 ORDER BY kind",
            params![network.0],
        )
        .await?
        .into_iter()
        .map(|(Code(kind), library)| (kind, LibraryId(library)))
        .collect())
    }

    pub(crate) async fn set_network_library_choice(
        &self,
        network: NetworkId,
        kind: LibraryKind,
        library: LibraryId,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO network_library_choices (network_id, kind, library_id) VALUES (?1, ?2, ?3) \
             ON CONFLICT (network_id, kind) DO UPDATE SET library_id = ?3",
            params![network.0, kind.code(), library.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_full_filesystem(
        &self,
        computer: ComputerId,
        draft: &FileSystemDraft,
    ) -> Result<()> {
        self.files
            .insert_filesystem_with(computer, draft, async |library| {
                self.library_id(library).await
            })
            .await
    }

    pub(crate) async fn insert_full_computer(
        &self,
        draft: &ComputerDraft,
        network: NetworkId,
        place: &LanPlace,
    ) -> Result<ComputerId> {
        self.computers
            .insert_computer_at_with(draft, network, Some(place), async |library| {
                self.library_id(library).await
            })
            .await
    }

    pub(crate) async fn insert_full_router(
        &self,
        draft: &RouterDraft,
        network: NetworkId,
        place: Option<&LanPlace>,
    ) -> Result<RouterId> {
        self.networks
            .insert_router_with(draft, network, place, async |library| {
                self.library_id(library).await
            })
            .await
    }
}
