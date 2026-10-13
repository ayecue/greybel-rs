//! The hack shop's downloads and catalogs on generated computers (`Router.PostGenComputer`).

use greyhack_behaviour::{
    DOWNLOADS_PATH, FIRST_VERSION, FileData, FileKind, HACK_SHOP_LIBRARIES, NetworkKind,
    PermissionSet, ROOT_USER, ServiceId, detail_key, path, shop_programs,
};
use greyhack_world_assets::{GameTextTable, Script};
use indexmap::IndexMap;

use crate::model::network::service_config::config_text;
use crate::{ComputerId, HttpConfig, NodeDraft, RepositoryConfig, Result, SoftwareInfo, World};

/// Returns the English detail text of the program or library file `name`, or its translation key
/// when the game has no text (`TranslationManager.GetText`).
fn program_detail(name: &str) -> String {
    let key = detail_key(name);

    GameTextTable::english()
        .get(&key)
        .map_or(key, str::to_owned)
}

impl World {
    /// Puts the newest version of each library a HackShop sells in `folder` (`PlayerUtils.UpdateHackshopLibs`): a
    /// library missing, not a library, or of another version than `VersionsControl.lastLibs` is replaced, which
    /// appends it to the folder. Does nothing on a computer of another network kind. Returns whether it changed
    /// the folder.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub(crate) async fn update_hackshop_libs(
        &mut self,
        computer: ComputerId,
        folder: &str,
    ) -> Result<bool> {
        let Some(row) = self.computer(computer).await? else {
            return Ok(false);
        };

        if self.network(row.network).await?.map(|network| network.kind)
            != Some(NetworkKind::HackShop)
        {
            return Ok(false);
        }

        let Some(folder) = self.folder(computer, folder, true).await? else {
            return Ok(false);
        };
        let mut changed = false;

        for kind in HACK_SHOP_LIBRARIES {
            let id = match self.db.latest_library(kind).await? {
                Some(id) => id,
                None => self.db.library_row(kind, FIRST_VERSION).await?,
            };
            let Some(latest) = self.db.library(id).await? else {
                continue;
            };
            let name = kind.file_name();
            let current = self.child(&folder, &name).await?;

            if let Some(node) = &current
                && node
                    .data()
                    .is_some_and(|data| data.kind == FileKind::Library)
                && self
                    .library_of(node)
                    .await?
                    .is_some_and(|library| library.version == latest.version)
            {
                continue;
            }

            if let Some(node) = current {
                self.remove_node(&node).await?;
            }

            let mut library = NodeDraft::file(
                &name,
                ROOT_USER,
                PermissionSet::default_for(false),
                FileData::library(FileData::binary_size(&name, true)),
            );

            library.library = Some(Box::new(latest));
            self.add_node(&folder, &library).await?;

            changed = true;
        }

        Ok(changed)
    }

    /// Adds device-specific downloads and HTTP/repository catalog entries from
    /// `Router.PostGenComputer`.
    pub(crate) async fn configure_shop(
        &mut self,
        computer: ComputerId,
        kind: NetworkKind,
    ) -> Result<()> {
        let Some(downloads) = shop_programs(kind) else {
            return Ok(());
        };

        if !self.has_service(computer, ServiceId::Http).await? {
            return Ok(());
        }

        let mut config: HttpConfig = self
            .content(computer, ServiceId::Http.config_path(), false)
            .await?
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default();

        for (name, price) in downloads {
            let data = Script::find(name).map_or_else(
                || FileData::binary(FileData::binary_size(name, false)),
                |script| FileData::binary_from_source(script.source),
            );

            self.write_file(computer, &path::join(DOWNLOADS_PATH, name), data)
                .await?;

            config.software.insert(
                (*name).to_owned(),
                SoftwareInfo {
                    description: program_detail(name),
                    price: *price,
                },
            );
        }

        self.write_file(
            computer,
            ServiceId::Http.config_path(),
            FileData::text(&config_text(&config)),
        )
        .await?;

        if kind == NetworkKind::HackShop
            && self.has_service(computer, ServiceId::Repository).await?
        {
            self.update_hackshop_libs(computer, DOWNLOADS_PATH).await?;

            let mut repository = RepositoryConfig {
                path_repo: DOWNLOADS_PATH.to_owned(),
                repo_sources: IndexMap::new(),
            };

            if let Some(folder) = self.folder(computer, DOWNLOADS_PATH, true).await? {
                for file in self.files(&folder).await? {
                    repository
                        .repo_sources
                        .insert(file.name().to_owned(), program_detail(file.name()));
                }
            }

            self.write_file(
                computer,
                ServiceId::Repository.config_path(),
                FileData::text(&config_text(&repository)),
            )
            .await?;
        }

        Ok(())
    }
}
