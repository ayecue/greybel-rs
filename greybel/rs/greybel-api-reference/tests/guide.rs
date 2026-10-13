//! Tests the page's guide in `greybel/ts/greybel-api-reference-web/guide`: what it renders through the documentation,
//! its code and its links.

use greybel_core::{Dialect, ParserOptions};

use crate::support::Harness;
use crate::support::guide::Guide;
use crate::support::reader::{Block, Link};
use crate::support::reference::{links, member, section};

#[test]
fn should_start_every_chapter_with_its_title() {
    let mut harness = Harness::new();

    for (anchor, chapter) in Guide::read().chapters {
        let blocks = harness.render(&chapter);

        assert!(
            matches!(blocks.first(), Some(Block::Heading { level: 1, .. })),
            "{anchor}"
        );
        assert_eq!(
            blocks
                .iter()
                .filter(|block| matches!(block, Block::Heading { level: 1, .. }))
                .count(),
            1
        );
    }
}

#[test]
fn should_parse_every_code_block_as_greyscript() {
    let mut harness = Harness::new();

    for markdown in Guide::read().markdown() {
        for block in harness.render(markdown) {
            let Block::Code {
                tokens,
                language: None,
            } = block
            else {
                continue;
            };
            let code: String = tokens.iter().map(|token| token.text.as_str()).collect();

            if let Err(error) = greybel_core::parse(&code, ParserOptions::new(Dialect::GreyScript))
            {
                panic!("can't parse\n{code}\n{error:?}");
            }
        }
    }
}

#[test]
fn should_link_to_sections_and_chapters_that_exist() {
    let mut harness = Harness::new();
    let guide = Guide::read();
    let anchors: Vec<&String> = guide.chapters.iter().map(|(anchor, _)| anchor).collect();
    let (_, sections) = harness.reference();

    for markdown in guide.markdown() {
        for (_, link) in links(&harness.render(markdown)) {
            match link {
                Link::Type { name } => assert_eq!(section(&sections, &name).name, name),
                Link::Member { owner, name } => {
                    assert_eq!(member(section(&sections, &owner), &name).name, name);
                }
                Link::Web { url } => assert!(url.starts_with("https://"), "{url}"),
                Link::Page { anchor } => {
                    assert!(anchors.contains(&&anchor), "the guide has no #{anchor}");
                }
            }
        }
    }
}
