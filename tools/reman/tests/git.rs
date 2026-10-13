use std::path::{Path, PathBuf};

use reman::git::Git;

use crate::support::git::history;

#[test]
fn should_list_a_file_changed_in_a_commit_since_the_base() {
    // Arrange
    let (fixture, base) = history();

    // Act
    let files = Git::open(fixture.root())
        .unwrap()
        .changed_files(&base)
        .unwrap();

    // Assert
    assert_eq!(files, [PathBuf::from("changed.txt")]);
}

#[test]
fn should_list_each_untracked_file_of_a_new_folder() {
    // Arrange
    let (fixture, _) = history();

    fixture
        .file("new/a.txt", "a\n")
        .file("new/inner/b.txt", "b\n");

    let git = Git::open(fixture.root()).unwrap();

    // Act
    let files = git.changed_files(&git.head().unwrap()).unwrap();

    // Assert
    assert_eq!(
        files,
        [PathBuf::from("new/a.txt"), PathBuf::from("new/inner/b.txt")]
    );
}

#[test]
fn should_not_list_a_file_changed_back_to_its_content_at_the_base() {
    // Arrange
    let (fixture, base) = history();

    fixture.file("changed.txt", "first\n");

    // Act
    let files = Git::open(fixture.root())
        .unwrap()
        .changed_files(&base)
        .unwrap();

    // Assert
    assert!(files.is_empty(), "{files:?}");
}

#[test]
fn should_find_the_last_commit_changing_a_file() {
    // Arrange
    let (fixture, base) = history();

    fixture.file("other.txt", "other\n").commit();

    // Act
    let commit = Git::open(fixture.root())
        .unwrap()
        .last_commit_of(Path::new("kept.txt"))
        .unwrap();

    // Assert
    assert_eq!(commit, Some(base));
}

#[test]
fn should_read_a_file_at_a_commit() {
    // Arrange
    let (fixture, base) = history();

    // Act
    let text = Git::open(fixture.root())
        .unwrap()
        .show(&base, Path::new("changed.txt"))
        .unwrap();

    // Assert
    assert_eq!(text.as_deref(), Some("first\n"));
}

#[test]
fn should_read_paths_relative_to_a_folder_inside_the_working_tree() {
    // Arrange
    let (fixture, base) = history();

    fixture
        .file("sub/inner.txt", "inner\n")
        .commit()
        .file("outside.txt", "outside\n");

    // Act
    let files = Git::open(&fixture.root().join("sub"))
        .unwrap()
        .changed_files(&base)
        .unwrap();

    // Assert
    assert_eq!(files, [PathBuf::from("inner.txt")]);
}
