//! Writing files with their parent folders created as needed.

use greyhack_behaviour::{FileData, PermissionSet, ROOT_USER, path};

use crate::systems::FileChangeService as FileChangesService;
use crate::{ComputerId, Node, NodeDraft, Result};

impl FileChangesService {
    /// Returns folder `path` on `computer`, creating missing parents.
    pub async fn ensure_folder(&self, computer: ComputerId, path: &str) -> Result<Option<Node>> {
        let Some(mut current) = self.lookup.root(computer).await? else {
            return Ok(None);
        };

        for (segment, full) in path::steps(path) {
            current = match self.lookup.folder(computer, &full, false).await? {
                Some(folder) => folder,
                None => {
                    self.add_node(
                        &current,
                        &NodeDraft::folder(segment, ROOT_USER, PermissionSet::default_for(true)),
                    )
                    .await?
                }
            };
        }

        Ok(Some(current))
    }

    /// Writes `data` to root-owned `path` on `computer`, creating the file and parent folders as
    /// needed.
    pub async fn write_file(&self, computer: ComputerId, path: &str, data: FileData) -> Result<()> {
        self.write_file_as(computer, path, ROOT_USER, data).await
    }

    /// Writes `data` to a file owned by `owner` for NPC account configuration.
    pub async fn write_file_as(
        &self,
        computer: ComputerId,
        path: &str,
        owner: &str,
        data: FileData,
    ) -> Result<()> {
        if let Some(mut file) = self.lookup.file(computer, path, false).await? {
            if let Some(contents) = file.data_mut() {
                *contents = data;

                return self.save_node(&file).await;
            }

            self.remove_node(&file).await?;
        }

        let Some(parent) = self.ensure_folder(computer, path::parent(path)).await? else {
            return Ok(());
        };

        self.add_node(
            &parent,
            &NodeDraft::file(
                path::file_name(path),
                owner,
                PermissionSet::default_for(false),
                data,
            ),
        )
        .await?;

        Ok(())
    }
}
