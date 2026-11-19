use crate::support::Project;
use crate::support::release::{ReleaseFeedServer, greybel_copy, run_update};

#[test]
fn should_replace_itself_with_the_newer_release() {
    let project = Project::new(&[]);
    let program = greybel_copy(&project);
    let feed = ReleaseFeedServer::start("99.0.0", "the new greybel", None);

    let (success, output) = run_update(&program, &["update"], &feed);

    assert!(success, "{output}");
    assert!(output.contains("Updated greybel from "), "{output}");
    assert_eq!(
        std::fs::read_to_string(&program).expect("the program"),
        "the new greybel"
    );
}

#[test]
fn should_only_report_the_newer_release_with_check() {
    let project = Project::new(&[]);
    let program = greybel_copy(&project);
    let feed = ReleaseFeedServer::start("99.0.0", "the new greybel", None);

    let (success, output) = run_update(&program, &["update", "--check"], &feed);

    assert!(success, "{output}");
    assert!(
        output.starts_with("greybel 99.0.0 is available (this is "),
        "{output}"
    );
    assert_ne!(
        std::fs::read(&program).expect("the program"),
        b"the new greybel"
    );
}

#[test]
fn should_stay_unchanged_when_it_is_the_newest() {
    let project = Project::new(&[]);
    let program = greybel_copy(&project);
    let feed = ReleaseFeedServer::start("0.0.1", "an old greybel", None);

    let (success, output) = run_update(&program, &["update"], &feed);

    assert!(success, "{output}");
    assert!(output.ends_with("is the newest version.\n"), "{output}");
    assert_ne!(
        std::fs::read(&program).expect("the program"),
        b"an old greybel"
    );
}

#[test]
fn should_reject_an_archive_with_another_sha256() {
    let project = Project::new(&[]);
    let program = greybel_copy(&project);
    let feed = ReleaseFeedServer::start("99.0.0", "a changed greybel", Some(&"0".repeat(64)));

    let (success, output) = run_update(&program, &["update"], &feed);

    assert!(!success);
    assert!(output.contains("failed its checksum check"), "{output}");
    assert_ne!(
        std::fs::read(&program).expect("the program"),
        b"a changed greybel"
    );
}
