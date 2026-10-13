//! Tests direct reference generation and Markdown serialization.

use greybel_api_reference::protocol::Search;
use greybel_api_reference::{reference, search};

#[test]
fn should_read_the_same_sections_the_search_finds() {
    let (overview, sections) = reference();
    let matches = search(&Search {
        query: "shell.host".into(),
        excluded_tags: Vec::new().into(),
    });

    assert_eq!(overview.contents.len(), sections.len());
    assert!(
        matches
            .members
            .iter()
            .all(
                |path| sections.iter().any(|section| section.name == path.owner
                    && section
                        .members
                        .iter()
                        .any(|member| member.name == path.name))
            )
    );
}

#[test]
fn should_write_a_section_as_markdown() {
    let (_, sections) = reference();
    let shell = sections
        .iter()
        .find(|section| section.name == "shell")
        .expect("a shell section");
    let markdown = shell.markdown("greyscript");

    assert!(markdown.starts_with("# shell\n"));
    assert!(markdown.contains("## `shell.host_computer(): computer`"));
    assert!(markdown.contains("```greyscript\n"));
}
