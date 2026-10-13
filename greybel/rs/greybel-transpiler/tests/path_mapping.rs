use greybel_transpiler::documents::Uri;

use crate::support::path_mapping::mapping;

#[test]
fn should_map_a_file_below_the_root_into_the_ingame_directory() {
    let uri = Uri::memory("/project/lib/tool.src");
    let mapped = mapping().map(&uri);

    assert_eq!(mapped, "/root/lib/tool.src");
}

#[test]
fn should_not_map_a_sibling_folder_sharing_the_roots_name_as_prefix() {
    let uri = Uri::memory("/project2/tool.src");
    let mapped = mapping().map(&uri);

    assert_eq!(mapped, "/project2/tool.src");
}

#[test]
fn should_not_map_a_file_of_another_scheme() {
    let uri = Uri::file("/project/tool.src");
    let relative = mapping().relative(&uri);

    assert_eq!(relative, None);
}
