use greybel_transpiler::PathMapping;
use greybel_transpiler::documents::Uri;

pub fn mapping() -> PathMapping {
    PathMapping {
        local_root: Uri::memory("/project"),
        ingame_directory: "/root/".to_owned(),
    }
}
