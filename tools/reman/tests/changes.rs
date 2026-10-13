use std::path::Path;

use indexmap::IndexMap;
use reman::changes::{ChangeFile, Level};

use crate::support::Fixture;

#[test]
fn should_measure_a_minor_move_between_two_part_versions() {
    // Act
    let level = Level::between("1.0", "1.1");

    // Assert
    assert_eq!(level, Some(Level::Minor));
}

#[test]
fn should_measure_a_major_move_between_three_part_versions() {
    // Act
    let level = Level::between("1.4.2", "2.0.0");

    // Assert
    assert_eq!(level, Some(Level::Major));
}

#[test]
fn should_measure_no_major_or_minor_move_as_a_patch() {
    // Act
    let level = Level::between("1.4.2", "1.4.3");

    // Assert
    assert_eq!(level, Some(Level::Patch));
}

#[test]
fn should_read_the_projects_and_note_of_a_change_file() {
    // Arrange
    let text = "---\nrs/core: minor\n\"greybel-vs\": patch\n---\n\nA note.\n";

    // Act
    let file = ChangeFile::parse(Path::new(".changes/a.md"), text).unwrap();

    // Assert
    assert_eq!(
        (file.levels.into_iter().collect::<Vec<_>>(), file.note),
        (
            vec![
                ("rs/core".to_owned(), Level::Minor),
                ("greybel-vs".to_owned(), Level::Patch)
            ],
            "A note.".to_owned()
        )
    );
}

#[test]
fn should_reject_a_change_file_naming_a_project_twice() {
    // Arrange
    let text = "---\ncore: minor\ncore: patch\n---\n\nA note.\n";

    // Act
    let result = ChangeFile::parse(Path::new(".changes/a.md"), text);

    // Assert
    assert!(result.is_err());
}

#[test]
fn should_reject_an_unknown_level() {
    // Arrange
    let text = "---\ncore: huge\n---\n\nA note.\n";

    // Act
    let result = ChangeFile::parse(Path::new(".changes/a.md"), text);

    // Assert
    assert!(result.is_err());
}

#[test]
fn should_read_back_a_rendered_change_file() {
    // Arrange
    let levels = IndexMap::from([("rs/core".to_owned(), Level::Major)]);
    let text = ChangeFile::render(&levels, "Breaks `x`.").unwrap();

    // Act
    let file = ChangeFile::parse(Path::new(".changes/a.md"), &text).unwrap();

    // Assert
    assert_eq!((file.levels, file.note), (levels, "Breaks `x`.".to_owned()));
}

#[test]
fn should_name_a_change_file_after_the_first_words_of_its_note() {
    // Arrange
    let fixture = Fixture::new();

    fixture.commit();

    let repository = fixture.repository();

    // Act
    let name = repository
        .changes()
        .file_name("`Clock::sleep_until` waits for the world clock, not the wall clock.");

    // Assert
    assert_eq!(name, "clock-sleep-until-waits-for-the-world-clock.md");
}

#[test]
fn should_reject_a_change_file_outside_the_pending_folder() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file(".changes/loose.md", "---\n---\n\nLoose.\n")
        .commit();

    let repository = fixture.repository();

    // Act
    let result = repository.changes().files();

    // Assert
    assert!(
        result.is_err_and(|error| error.to_string().contains(".changes/pending")),
        "a loose change file is accepted"
    );
}

#[test]
fn should_ignore_the_change_files_of_released_folders() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file(
            ".changes/2026-01-01/old.md",
            "---\nmissing: patch\n---\n\nOld.\n",
        )
        .commit();

    let repository = fixture.repository();

    // Act
    let files = repository.changes().files().unwrap();

    // Assert
    assert!(files.is_empty());
}
