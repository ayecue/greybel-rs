//! New library versions released while the world runs (`VersionsControl`): a version patches the weaknesses players
//! used too often, and the world stores the versions it released.

use std::collections::HashMap;

use greyhack_behaviour::{
    FIRST_VERSION, LibraryKind, LibraryRole, MAX_WEAKNESS_USES, Reward, Rng, count_use,
    is_worn_out, is_zone_exhausted, next_version, replacement_zone_count, saved_versions_to_prune,
};
use greyhack_world_core::{LibraryId, Result};

use super::report::ExploitOutcome;
use crate::Library;
use crate::generation::LibraryGeneration;
use crate::generation::library::{VersionTable, WeaknessContext};
use crate::storage::UseCount;

/// Libraries a player cannot report weaknesses of (`FileHelperServer.ExploitReportServerRpc`).
const UNREPORTABLE: [LibraryKind; 3] = [
    LibraryKind::Metaxploit,
    LibraryKind::Crypto,
    LibraryKind::Rshell,
];

/// Why a weakness was not reported (`FileHelperServer.ExploitReportServerRpc`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeaknessReportFailure {
    /// The library is one players cannot report weaknesses of.
    Unreportable,
    /// The world has no library with the file name.
    UnknownLibrary(String),
    /// The newest version of the library has no memory zone at the address.
    UnknownZone,
    /// The memory zone has no weakness with the value.
    UnknownValue { value: String, zone: String },
    /// The player reported a weakness since the last check for library versions.
    Pending,
}

/// Methods of [`World`] for exploits library versions.
/// Mutable state used while building one library release.
struct LibraryRelease {
    old_id: LibraryId,
    old: Library,
    metaxploit_id: LibraryId,
    metaxploit: Library,
    uses: Vec<UseCount>,
    patches: HashMap<String, u32>,
    table: VersionTable,
    rng: Rng,
    new: Library,
    exhausted_zones: Vec<String>,
    patched: bool,
}

impl VersionService {
    pub async fn weakness_use_count(
        &self,
        kind: LibraryKind,
        address: &str,
        unsec_value: &str,
    ) -> Result<u32> {
        self.library_versions
            .weakness_use_count(kind, address, unsec_value)
            .await
    }

    pub async fn count_weakness_use(
        &self,
        kind: LibraryKind,
        address: &str,
        unsec_value: &str,
    ) -> Result<()> {
        let Some(library) = self.generator.latest_full_library(kind).await? else {
            return Ok(());
        };

        if library.weakness(address, unsec_value).is_none() {
            return Ok(());
        }

        let uses = self
            .library_versions
            .weakness_use_count(kind, address, unsec_value)
            .await?;

        self.library_versions
            .set_weakness_use_count(kind, address, unsec_value, count_use(uses))
            .await
    }

    pub async fn count_attack_use(
        &self,
        library: &str,
        attack: (&str, &str, Reward),
        outcome: &ExploitOutcome,
    ) -> Result<()> {
        let (address, unsec_value, reward) = attack;
        let is_use = match outcome {
            ExploitOutcome::Shell { .. }
            | ExploitOutcome::File { .. }
            | ExploitOutcome::Computer { .. } => true,
            ExploitOutcome::Success => {
                matches!(reward, Reward::ChangePass | Reward::FirewallDisable)
            }
            _ => false,
        };

        match LibraryKind::of_file(library) {
            Some(kind) if is_use => self.count_weakness_use(kind, address, unsec_value).await,
            _ => Ok(()),
        }
    }

    pub async fn report_weakness(
        &self,
        library: &str,
        address: &str,
        unsec_value: &str,
    ) -> Result<Result<(), WeaknessReportFailure>> {
        let kind = LibraryKind::of_file(library);

        if kind.is_some_and(|kind| UNREPORTABLE.contains(&kind)) {
            return Ok(Err(WeaknessReportFailure::Unreportable));
        }

        let newest = match kind {
            Some(kind) => self
                .generator
                .latest_full_library(kind)
                .await?
                .zip(Some(kind)),
            None => None,
        };
        let Some((newest, kind)) = newest else {
            return Ok(Err(WeaknessReportFailure::UnknownLibrary(
                library.to_owned(),
            )));
        };
        let Some(zone) = newest.zones.iter().find(|zone| zone.address == address) else {
            return Ok(Err(WeaknessReportFailure::UnknownZone));
        };

        if !zone
            .weaknesses
            .iter()
            .any(|weakness| weakness.unsec_value == unsec_value)
        {
            return Ok(Err(WeaknessReportFailure::UnknownValue {
                value: unsec_value.to_owned(),
                zone: address.to_owned(),
            }));
        }

        self.library_versions
            .set_weakness_use_count(kind, address, unsec_value, MAX_WEAKNESS_USES)
            .await?;

        if !self
            .library_versions
            .add_library_reporter(self.world.player())
            .await?
        {
            return Ok(Err(WeaknessReportFailure::Pending));
        }

        Ok(Ok(()))
    }

    pub async fn wear_out_visible_weaknesses(&self, kind: LibraryKind) -> Result<()> {
        let Some(library) = self.generator.latest_full_library(kind).await? else {
            return Ok(());
        };

        for zone in library.zones.iter().filter(|zone| !zone.hide) {
            for weakness in &zone.weaknesses {
                self.library_versions
                    .set_weakness_use_count(
                        kind,
                        &zone.address,
                        &weakness.unsec_value,
                        MAX_WEAKNESS_USES,
                    )
                    .await?;
            }
        }

        Ok(())
    }

    /// Configures the v1 contents and exploit batches, then performs the game's five initial releases for every other kind.
    /// A saved history is never regenerated, including a mirrored history registered by the test bridge.
    pub async fn prepare_initial_versions(&self) -> Result<()> {
        if !self.library_versions.saved_versions().await?.is_empty() {
            return Ok(());
        }

        for kind in LibraryKind::ALL.iter().copied() {
            let mut rng = Rng::new(
                self.world
                    .world_seed()
                    .derive_u64("library-history-v1", &[&kind.code().to_string()]),
            );
            let library = Library::generate(&mut rng, kind, FIRST_VERSION);
            let id = self.libraries.library_id(&library).await?;

            self.library_versions.save_library(kind, id).await?;
            self.libraries
                .files
                .library_row(kind, FIRST_VERSION)
                .await?;
            self.shop.library_exploits(id, &library).await?;
        }

        for kind in LibraryKind::ALL
            .iter()
            .copied()
            .filter(|kind| *kind != LibraryKind::Metaxploit)
        {
            for _ in 0..5 {
                self.release_library_version(kind).await?;
            }
        }

        Ok(())
    }

    pub async fn release_library_version(&self, kind: LibraryKind) -> Result<Option<String>> {
        self.wear_out_visible_weaknesses(kind).await?;

        self.check_library(kind).await
    }

    pub async fn check_library_versions(&self) -> Result<Vec<LibraryKind>> {
        let mut kinds = Vec::new();

        for &kind in LibraryKind::ALL {
            if self.check_library(kind).await?.is_some() {
                kinds.push(kind);
            }
        }

        Ok(kinds)
    }

    async fn patch_weaknesses(
        &self,
        kind: LibraryKind,
        release: &mut LibraryRelease,
    ) -> Result<()> {
        for zone in 0..release.new.zones.len() {
            if release.new.zones[zone].hide {
                continue;
            }

            let address = release.new.zones[zone].address.clone();
            let exhausted = is_zone_exhausted(release.patches.get(&address).copied().unwrap_or(0));

            if exhausted {
                release.exhausted_zones.push(address.clone());
            }

            for index in 0..release.new.zones[zone].weaknesses.len() {
                let unsec_value = release.new.zones[zone].weaknesses[index]
                    .unsec_value
                    .clone();
                let use_count = release
                    .uses
                    .iter()
                    .find(|entry| entry.address == address && entry.unsec_value == unsec_value)
                    .map_or(0, |entry| entry.uses);

                if !exhausted && !is_worn_out(use_count) {
                    continue;
                }

                release.patched = true;
                self.shop
                    .patch_exploit_description(kind, &release.old.version, &address, &unsec_value)
                    .await?;

                if !exhausted {
                    *release.patches.entry(address.clone()).or_default() += 1;
                    release.new.renew_weakness(
                        &mut release.rng,
                        zone,
                        index,
                        WeaknessContext {
                            metaxploit: Some(&release.metaxploit.version),
                            table: Some(&release.table),
                        },
                    );
                }
            }
        }

        Ok(())
    }

    async fn finish_release(
        &self,
        kind: LibraryKind,
        mut release: LibraryRelease,
    ) -> Result<Option<String>> {
        if !release.patched {
            return Ok(None);
        }

        release.new.version = next_version(&release.old.version);
        release.new.remove_hidden_zones();

        if !release.exhausted_zones.is_empty() {
            let count = replacement_zone_count(
                release.exhausted_zones.len(),
                release.new.zones.len(),
                || release.rng.range(0, 9),
            );

            for address in &release.exhausted_zones {
                release.patches.remove(address);
            }

            release
                .new
                .zones
                .retain(|zone| !release.exhausted_zones.contains(&zone.address));
            release.new.add_zones(
                &mut release.rng,
                count,
                false,
                WeaknessContext {
                    metaxploit: None,
                    table: Some(&release.table),
                },
            );
        }

        // Metaxploit gets a version for every library released, its own included, which counts twice.
        let next_metaxploit = next_version(&release.metaxploit.version);

        if kind == LibraryKind::Metaxploit {
            release.new.version = next_version(&release.new.version);
        }

        let context = WeaknessContext {
            metaxploit: Some(&next_metaxploit),
            table: Some(&release.table),
        };

        release.new.add_hidden_zones(&mut release.rng, context);

        if kind.role() == LibraryRole::Router
            && let Some(zone) = release.new.random_zone(&mut release.rng, false)
        {
            release
                .new
                .force_router_weakness(&mut release.rng, zone, context);
        }

        let new_id = self.libraries.library_id(&release.new).await?;
        let metaxploit_next_id = if kind == LibraryKind::Metaxploit {
            new_id
        } else {
            let mut next = release.metaxploit.clone();

            next.version = next_metaxploit;

            self.libraries.library_id(&next).await?
        };

        self.store_release(
            LibraryKind::Metaxploit,
            release.metaxploit_id,
            metaxploit_next_id,
        )
        .await?;
        self.store_release(kind, release.old_id, new_id).await?;
        self.shop.library_exploits(new_id, &release.new).await?;
        self.prune_oldest(LibraryKind::Metaxploit).await?;

        if kind != LibraryKind::Metaxploit {
            self.prune_oldest(kind).await?;
        }

        // `Vulnerabilidad.Configure` creates fresh helperHackResult state for the new version. Reusing a weakness's
        // old unsec value must not carry its exhausted use count into the release.
        self.library_versions
            .replace_weakness_uses(kind, &[])
            .await?;

        let zone_patches: Vec<(String, u32)> = release
            .new
            .zones
            .iter()
            .filter_map(|zone| {
                release
                    .patches
                    .get(&zone.address)
                    .map(|times| (zone.address.clone(), *times))
            })
            .collect();

        self.library_versions
            .replace_zone_patches(kind, &zone_patches)
            .await?;

        Ok(Some(release.new.version))
    }

    pub async fn check_library(&self, kind: LibraryKind) -> Result<Option<String>> {
        let Some(old_id) = self.files.latest_library(kind).await? else {
            return Ok(None);
        };
        let Some(old) = self.libraries.library(old_id).await? else {
            return Ok(None);
        };
        let (metaxploit_id, metaxploit) = self.latest_metaxploit().await?;
        let uses = self.library_versions.weakness_uses(kind).await?;
        let patches: HashMap<String, u32> = self
            .library_versions
            .zone_patches(kind)
            .await?
            .into_iter()
            .collect();
        let table = self.stored_version_table().await?;
        let rng = Rng::new(self.world.world_seed().derive_u64(
            "library-version-v1",
            &[&kind.code().to_string(), &old.version],
        ));
        let new = old.clone();
        let exhausted_zones = Vec::new();
        let patched = false;

        let mut release = LibraryRelease {
            old_id,
            old,
            metaxploit_id,
            metaxploit,
            uses,
            patches,
            table,
            rng,
            new,
            exhausted_zones,
            patched,
        };

        self.patch_weaknesses(kind, &mut release).await?;

        self.finish_release(kind, release).await
    }

    pub async fn saved_library_versions(&self, kind: LibraryKind) -> Result<Vec<String>> {
        Ok(self
            .library_versions
            .saved_versions()
            .await?
            .into_iter()
            .filter(|(entry, _)| *entry == kind)
            .map(|(_, version)| version)
            .collect())
    }
}

impl VersionService {
    pub async fn latest_metaxploit(&self) -> Result<(LibraryId, Library)> {
        if let Some(id) = self.files.latest_library(LibraryKind::Metaxploit).await?
            && let Some(library) = self.libraries.library(id).await?
        {
            return Ok((id, library));
        }

        let mut rng = Rng::new(self.world.world_seed().derive_u64(
            "library-version-v1",
            &[&LibraryKind::Metaxploit.code().to_string(), FIRST_VERSION],
        ));
        let library = Library::generate(&mut rng, LibraryKind::Metaxploit, FIRST_VERSION);
        let id = self.libraries.library_id(&library).await?;

        Ok((id, library))
    }

    pub async fn stored_version_table(&self) -> Result<VersionTable> {
        let mut table = VersionTable::new();

        for (kind, version) in self.library_versions.saved_versions().await? {
            table.entry(kind).or_default().push(version);
        }

        Ok(table)
    }

    pub async fn store_release(
        &self,
        kind: LibraryKind,
        base: LibraryId,
        next: LibraryId,
    ) -> Result<()> {
        if self
            .library_versions
            .saved_libraries(kind)
            .await?
            .is_empty()
        {
            self.library_versions.save_library(kind, base).await?;
        }

        self.library_versions.save_library(kind, next).await?;

        if let Some(library) = self.libraries.library(next).await? {
            self.libraries
                .files
                .library_row(kind, &library.version)
                .await?;
        }

        Ok(())
    }

    pub async fn prune_oldest(&self, kind: LibraryKind) -> Result<()> {
        let entries = self.library_versions.saved_libraries(kind).await?;

        self.library_versions
            .prune_saved_libraries(kind, saved_versions_to_prune(entries.len()))
            .await
    }
}

/// A world service with only its declared dependencies.
pub struct VersionService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) libraries: std::rc::Rc<crate::storage::LibraryRepository>,
    pub(crate) library_versions: std::rc::Rc<crate::storage::LibraryVersionRepository>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) generator: greyhack_world_modules::Dep<crate::Generator>,
    pub(crate) shop: greyhack_world_modules::Dep<crate::Shop>,
}

impl VersionService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        libraries: std::rc::Rc<crate::storage::LibraryRepository>,
        library_versions: std::rc::Rc<crate::storage::LibraryVersionRepository>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        generator: greyhack_world_modules::Dep<crate::Generator>,
        shop: greyhack_world_modules::Dep<crate::Shop>,
    ) -> Self {
        Self {
            world: world.clone(),
            libraries,
            library_versions,
            files,
            generator,
            shop,
        }
    }
}
