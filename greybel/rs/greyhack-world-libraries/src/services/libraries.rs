//! Persistence entry points for generated drafts containing full library entities.

use std::rc::Rc;

use greyhack_world_core::{ComputerId, LanPlace, NetworkId, Result, RouterId};

use crate::storage::LibraryRepository;
use crate::{ComputerDraft, FileSystemDraft, RouterDraft};

/// Stores generated core drafts together with their full library entities.
pub struct LibraryService {
    pub(crate) libraries: Rc<LibraryRepository>,
}

impl LibraryService {
    pub(crate) fn new(libraries: Rc<LibraryRepository>) -> Self {
        Self { libraries }
    }

    /// Stores a generated file system and its full library entities on `computer`.
    pub async fn insert_full_filesystem(
        &self,
        computer: ComputerId,
        draft: &FileSystemDraft,
    ) -> Result<()> {
        self.libraries.insert_full_filesystem(computer, draft).await
    }

    /// Stores a generated computer and its full library entities, returning its ID.
    pub async fn insert_full_computer(
        &self,
        draft: &ComputerDraft,
        network: NetworkId,
        place: &LanPlace,
    ) -> Result<ComputerId> {
        self.libraries
            .insert_full_computer(draft, network, place)
            .await
    }

    /// Stores a generated router and its full library entities, returning its ID.
    pub async fn insert_full_router(
        &self,
        draft: &RouterDraft,
        network: NetworkId,
        place: Option<&LanPlace>,
    ) -> Result<RouterId> {
        self.libraries
            .insert_full_router(draft, network, place)
            .await
    }
}
