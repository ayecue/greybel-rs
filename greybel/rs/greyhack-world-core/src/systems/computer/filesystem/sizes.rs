//! File counts, sizes, and free disk space.

use std::rc::Rc;

use greyhack_behaviour::{BYTES_PER_MB, path};

use crate::storage::*;
use crate::systems::{FileSizeService as FileSizesService, *};
use crate::*;

impl FileSizesService {
    /// Counts computer files and folders excluding root (`GetTotalFicheros`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn file_count(&self, computer: ComputerId) -> Result<usize> {
        self.node_rows.count_below(computer, path::ROOT).await
    }

    /// Counts files and folders below `folder`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn descendant_count(&self, folder: &Node) -> Result<usize> {
        self.node_rows
            .count_below(folder.computer, &folder.path)
            .await
    }

    /// Computes `GetSizeBytes`: symlink target length, file size, or recursive folder size. With
    /// `metadata`, each folder and text file adds 14,000 bytes.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn size(&self, node: &Node, metadata: bool) -> Result<u64> {
        if !node.is_folder() || node.is_symlink() {
            return Ok(node.own_size(metadata));
        }

        let below = self.node_rows.tree_below(node.computer, &node.path).await?;

        Ok(node.own_size(metadata)
            + below
                .iter()
                .map(|node| node.own_size(metadata))
                .sum::<u64>())
    }

    /// Returns free disk MB: computer capacity minus file sizes.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn available_storage_mb(&self, computer: ComputerId) -> Result<f64> {
        let Some(root) = self.lookup.root(computer).await? else {
            return Ok(0.0);
        };
        let usage_mb = self.size(&root, false).await? as f64 / BYTES_PER_MB;

        Ok(f64::from(
            self.hardware_rows
                .hardware(computer)
                .await?
                .disk_capacity_mb(),
        ) - usage_mb)
    }
}

/// File-system size and capacity rules.
pub struct FileSizeService {
    pub(crate) node_rows: Rc<NodeRepository>,
    pub(crate) hardware_rows: Rc<HardwareRepository>,
    pub(crate) lookup: Dep<FileLookup>,
}

impl FileSizeService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        node_rows: Rc<NodeRepository>,
        hardware_rows: Rc<HardwareRepository>,
    ) -> Self {
        Self {
            node_rows,
            hardware_rows,
            lookup: registry.get(),
        }
    }
}
