//! Tests the files of a path and the paths shown for documents.

use std::path::Path;

use greybel_documents::Uri;
use greybel_documents_fs_provider::FsProvider;

use crate::support::find::{project, uris};

#[test]
fn should_list_every_file_in_a_folder() {
    let folder = project();
    let found = FsProvider::files(&folder.path().join("lib"));

    assert_eq!(found, uris(&folder, &["lib/util.gs"]));
}

#[test]
fn should_list_a_file_on_its_own() {
    let folder = project();
    let found = FsProvider::files(&folder.path().join("notes.txt"));

    assert_eq!(found, uris(&folder, &["notes.txt"]));
}

#[test]
fn should_list_nothing_for_a_missing_path() {
    let folder = project();
    let found = FsProvider::files(&folder.path().join("missing"));

    assert_eq!(found, Vec::<Uri>::new());
}

#[test]
fn should_find_the_files_in_a_folder_matching_a_glob() {
    let folder = project();
    let found = FsProvider::find_in(folder.path(), "*.{src,txt}");

    assert_eq!(found, uris(&folder, &["main.src", "notes.txt"]));
}

#[test]
fn should_find_in_the_current_folder_for_an_empty_path() {
    let found = FsProvider::find_in(Path::new(""), "Cargo.toml");

    assert_eq!(found, vec![FsProvider::uri(Path::new("Cargo.toml"))]);
}

#[test]
fn should_show_a_file_in_the_current_folder_relative_to_it() {
    let uri = FsProvider::uri(Path::new("src/lib.rs"));

    assert_eq!(
        FsProvider::display_path(&uri),
        Path::new("src/lib.rs").display().to_string()
    );
}

#[test]
fn should_show_other_files_by_their_absolute_path() {
    let folder = project();
    let path = folder.path().join("main.src");

    assert_eq!(
        FsProvider::display_path(&FsProvider::uri(&path)),
        path.display().to_string()
    );
}

#[test]
fn should_show_a_document_that_isnt_a_file_by_its_uri() {
    let uri = Uri::parse("untitled:Untitled-1").expect("a URI");

    assert_eq!(FsProvider::display_path(&uri), "untitled:Untitled-1");
}

#[test]
fn should_read_the_text_of_a_file_and_nothing_for_a_folder() {
    let folder = project();

    assert_eq!(
        FsProvider::text(&FsProvider::uri(&folder.path().join("notes.txt"))).as_deref(),
        Some(
            std::fs::read_to_string(folder.path().join("notes.txt"))
                .expect("notes")
                .as_str()
        )
    );
    assert_eq!(
        FsProvider::text(&FsProvider::uri(&folder.path().join("lib"))),
        None
    );
}
