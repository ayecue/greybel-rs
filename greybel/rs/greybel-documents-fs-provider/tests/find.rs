//! Tests glob matching within the provider's source folders.

use greybel_documents::Uri;

use crate::support::find::{project, provider, uris};

#[test]
fn should_find_the_files_matching_the_include_glob() {
    let folder = project();
    let found = provider(&folder).find("**/*.{src,gs}", None);

    assert_eq!(
        found,
        uris(
            &folder,
            &[
                ".config/hidden.src",
                "lib/util.gs",
                "main.src",
                "node_modules/dep/vendored.src"
            ]
        )
    );
}

#[test]
fn should_match_a_star_within_one_segment() {
    let folder = project();
    let found = provider(&folder).find("*.src", None);

    assert_eq!(found, uris(&folder, &["main.src"]));
}

#[test]
fn should_leave_out_the_folders_matching_the_exclude_glob() {
    let folder = project();
    let found = provider(&folder).find("**/*.{src,gs}", Some("**/{node_modules,.*}/**"));

    assert_eq!(found, uris(&folder, &["lib/util.gs", "main.src"]));
}

#[test]
fn should_find_nothing_for_an_invalid_include_glob() {
    let folder = project();
    let found = provider(&folder).find("**/*.{src", None);

    assert_eq!(found, Vec::<Uri>::new());
}

#[cfg(unix)]
#[test]
fn should_not_enter_a_symbolic_link_to_a_folder() {
    let folder = project();

    std::os::unix::fs::symlink(folder.path().join("lib"), folder.path().join("linked"))
        .expect("links the folder");

    let found = provider(&folder).find("**/*.gs", None);

    assert_eq!(found, uris(&folder, &["lib/util.gs"]));
}
