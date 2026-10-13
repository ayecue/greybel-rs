use reman::git::Git;

use super::Fixture;

/// A repository with `kept.txt` and `changed.txt` committed, then `changed.txt` changed in a second commit.
pub fn history() -> (Fixture, String) {
    let fixture = Fixture::new();

    fixture
        .file("kept.txt", "kept\n")
        .file("changed.txt", "first\n")
        .commit();

    let base = Git::open(fixture.root()).unwrap().head().unwrap();

    fixture.file("changed.txt", "second\n").commit();

    (fixture, base)
}
