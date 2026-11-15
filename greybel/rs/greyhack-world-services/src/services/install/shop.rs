//! The hack shop's downloads and catalogs on generated computers (`Router.PostGenComputer`).

use greyhack_behaviour::{
    DOWNLOADS_PATH, FIRST_VERSION, FileData, FileKind, HACK_SHOP_LIBRARIES, NetworkKind,
    PermissionSet, ROOT_USER, ServiceId, detail_key, path, shop_programs,
};
use greyhack_world_assets::{GameTextTable, Script};
use indexmap::IndexMap;

use crate::model::service_config::config_text;
use crate::services::InstallService;
use crate::{ComputerId, HttpConfig, NodeDraft, RepositoryConfig, Result, SoftwareInfo};

/// Returns the English detail text of the program or library file `name`, or its translation key
/// when the game has no text (`TranslationManager.GetText`).
fn program_detail(name: &str) -> String {
    let key = detail_key(name);

    GameTextTable::english()
        .get(&key)
        .map_or(key, str::to_owned)
}

/// Methods of [`World`] for services install shop.
impl InstallService {
    pub async fn update_hackshop_libs(&self, computer: ComputerId, folder: &str) -> Result<bool> {
        let Some(row) = self.networks.topology.computer(computer).await? else {
            return Ok(false);
        };

        if self
            .networks
            .settings
            .network(row.network)
            .await?
            .map(|network| network.kind)
            != Some(NetworkKind::HackShop)
        {
            return Ok(false);
        }

        let Some(folder) = self.files.lookup.folder(computer, folder, true).await? else {
            return Ok(false);
        };
        let mut changed = false;

        for kind in HACK_SHOP_LIBRARIES {
            let id = match self.files.latest_library(kind).await? {
                Some(id) => id,
                None => self.files.library_row(kind, FIRST_VERSION).await?,
            };
            let Some(latest) = self.files.libraries.library_identity(id).await? else {
                continue;
            };
            let name = kind.file_name();
            let current = self.files.lookup.child(&folder, &name).await?;

            if let Some(node) = &current
                && node
                    .data()
                    .is_some_and(|data| data.kind == FileKind::Library)
                && self
                    .files
                    .libraries
                    .library_of(node)
                    .await?
                    .is_some_and(|library| library.version == latest.version)
            {
                continue;
            }

            if let Some(node) = current {
                self.files.changes.remove_node(&node).await?;
            }

            let mut library = NodeDraft::file(
                &name,
                ROOT_USER,
                PermissionSet::default_for(false),
                FileData::library(FileData::binary_size(&name, true)),
            );

            library.library = Some(Box::new(id));
            self.files
                .changes
                .add_node_with(&folder, &library, async |id| Ok(*id))
                .await?;

            changed = true;
        }

        Ok(changed)
    }

    pub async fn configure_shop(&self, computer: ComputerId, kind: NetworkKind) -> Result<()> {
        let Some(downloads) = shop_programs(kind) else {
            return Ok(());
        };

        if !self
            .computers
            .processes
            .has_service(computer, ServiceId::Http)
            .await?
        {
            return Ok(());
        }

        let mut config: HttpConfig = self
            .files
            .lookup
            .content(computer, ServiceId::Http.config_path(), false)
            .await?
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default();

        for (name, price) in downloads {
            let data = Script::find(name).map_or_else(
                || FileData::binary(FileData::binary_size(name, false)),
                |script| FileData::binary_from_source(script.source),
            );

            self.files
                .changes
                .write_file(computer, &path::join(DOWNLOADS_PATH, name), data)
                .await?;

            config.software.insert(
                (*name).to_owned(),
                SoftwareInfo {
                    description: program_detail(name),
                    price: *price,
                },
            );
        }

        self.files
            .changes
            .write_file(
                computer,
                ServiceId::Http.config_path(),
                FileData::text(&config_text(&config)),
            )
            .await?;

        if kind == NetworkKind::HackShop
            && self
                .computers
                .processes
                .has_service(computer, ServiceId::Repository)
                .await?
        {
            self.update_hackshop_libs(computer, DOWNLOADS_PATH).await?;

            let mut repository = RepositoryConfig {
                path_repo: DOWNLOADS_PATH.to_owned(),
                repo_sources: IndexMap::new(),
            };

            if let Some(folder) = self
                .files
                .lookup
                .folder(computer, DOWNLOADS_PATH, true)
                .await?
            {
                for file in self.files.lookup.files(&folder).await? {
                    repository
                        .repo_sources
                        .insert(file.name().to_owned(), program_detail(file.name()));
                }
            }

            self.files
                .changes
                .write_file(
                    computer,
                    ServiceId::Repository.config_path(),
                    FileData::text(&config_text(&repository)),
                )
                .await?;
        }

        Ok(())
    }
}
