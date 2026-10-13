use greybel_documents::PathResolution;
#[cfg(not(target_family = "wasm"))]
use greybel_documents_fs_provider::FsProvider;

pub(in crate::server) enum FileAccess {
    Client(PathResolution),
    #[cfg(not(target_family = "wasm"))]
    Local(FsProvider),
}

impl FileAccess {
    pub(in crate::server) fn resolution(&self) -> &PathResolution {
        match self {
            Self::Client(resolution) => resolution,
            #[cfg(not(target_family = "wasm"))]
            Self::Local(provider) => provider.resolution(),
        }
    }

    pub(in crate::server) fn set_resolution(&mut self, resolution: PathResolution) {
        match self {
            Self::Client(current) => *current = resolution,
            #[cfg(not(target_family = "wasm"))]
            Self::Local(provider) => *provider = FsProvider::new(resolution),
        }
    }
}
