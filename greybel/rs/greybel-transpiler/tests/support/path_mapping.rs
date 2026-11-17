use greybel_documents::Uri;
use greybel_transpiler::PathMapping;

pub fn mapping() -> PathMapping {
    PathMapping {
        local_root: Uri::memory("/project"),
        ingame_directory: "/root/".to_owned(),
    }
}
