//! Tests the overview and per-type sections streamed by the documentation service.

use crate::support::Harness;
use crate::support::reader::{Block, Highlight, Link, Literal, MemberKind, Style, TypePart};
use crate::support::reference::{code, links, member, section, spans, text, type_text};

#[test]
fn should_list_every_type_with_its_members_before_the_sections() {
    let mut harness = Harness::new();

    let (overview, sections) = harness.reference();

    let contents: Vec<&str> = overview
        .contents
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    let names: Vec<&str> = sections
        .iter()
        .map(|section| section.name.as_str())
        .collect();

    assert_eq!(contents, names);
    assert_eq!(&names[..3], ["admin", "adminComputer", "adminNetwork"]);
    assert!(!names.contains(&"any") && !names.contains(&"class"));

    let shell = overview
        .contents
        .iter()
        .find(|entry| entry.name == "shell")
        .expect("shell");

    assert!(shell.members.contains(&"host_computer".to_owned()));
    assert!(overview.tags.contains(&"method".to_owned()));
}

#[test]
fn should_describe_a_function_with_its_arguments_and_returns() {
    let mut harness = Harness::new();

    let (_, sections) = harness.reference();

    let shell = section(&sections, "shell");

    assert_eq!(shell.extends.as_deref(), Some("map"));

    let host_computer = member(shell, "host_computer");

    assert_eq!(host_computer.tags, ["method"]);
    assert_eq!(
        text(&host_computer.description),
        "Returns a computer related to the shell."
    );

    let MemberKind::Function {
        arguments, returns, ..
    } = &host_computer.kind
    else {
        panic!("host_computer is a function");
    };

    assert!(arguments.is_empty());
    assert_eq!(
        returns[0].parts,
        [TypePart::Name {
            name: "computer".into()
        }]
    );
}

#[test]
fn should_write_nested_types_with_their_names_apart() {
    let mut harness = Harness::new();

    let (_, sections) = harness.reference();

    let MemberKind::Function { returns, .. } =
        &member(section(&sections, "computer"), "get_ports").kind
    else {
        panic!("get_ports is a function");
    };

    assert_eq!(type_text(&returns[0]), "list<port>");
    assert_eq!(
        returns[0].parts[2],
        TypePart::Name {
            name: "port".into()
        }
    );
}

#[test]
fn should_resolve_the_groups_of_return_variations() {
    let mut harness = Harness::new();

    let (_, sections) = harness.reference();

    let MemberKind::Function {
        return_variations, ..
    } = &member(section(&sections, "computer"), "create_folder").kind
    else {
        panic!("create_folder is a function");
    };

    assert!(return_variations.contains(&Literal::String {
        value: "permission denied".into()
    }));
    assert!(return_variations.contains(&Literal::Number { value: 1.0 }));
    assert!(!return_variations.iter().any(|variation| matches!(
        variation,
        Literal::String { value } if value.starts_with('!')
    )));
}

#[test]
fn should_parse_descriptions_with_code_and_links_to_sections() {
    let mut harness = Harness::new();

    let (_, sections) = harness.reference();

    let shell = section(&sections, "shell");
    let links = links(&shell.description);

    assert!(links.contains(&(
        "get_shell".to_owned(),
        Link::Member {
            owner: "general".into(),
            name: "get_shell".into()
        }
    )));
    assert!(links.contains(&(
        "overflow".to_owned(),
        Link::Member {
            owner: "metaLib".into(),
            name: "overflow".into()
        }
    )));
    assert!(
        spans(&shell.description)
            .iter()
            .any(|span| span.style == Style::Code && span.text == "\"shell\"")
    );
}

#[test]
fn should_link_every_anchor_of_the_descriptions_to_a_section() {
    let mut harness = Harness::new();

    let (_, sections) = harness.reference();

    let descriptions = sections.iter().flat_map(|section| {
        std::iter::once(&section.description)
            .chain(section.members.iter().map(|member| &member.description))
    });

    for description in descriptions {
        for (_, link) in links(description) {
            match link {
                Link::Type { name } => assert_eq!(section(&sections, &name).name, name),
                Link::Member { owner, name } => {
                    assert_eq!(member(section(&sections, &owner), &name).name, name);
                }
                Link::Web { url } => assert!(url.starts_with("http"), "{url}"),
                Link::Page { anchor } => panic!("#{anchor} is no section's anchor"),
            }
        }
    }
}

#[test]
fn should_read_markdown_lists_as_list_blocks() {
    let mut harness = Harness::new();

    let (_, sections) = harness.reference();

    let current_date = member(section(&sections, "general"), "current_date");
    let Some(Block::List { items }) = current_date.description.get(1) else {
        panic!(
            "current_date's description has a list: {:?}",
            current_date.description
        );
    };

    assert_eq!(items.len(), 2);
}

#[test]
fn should_highlight_examples_and_link_their_general_functions() {
    let mut harness = Harness::new();

    let (_, sections) = harness.reference();

    let example = &member(section(&sections, "shell"), "host_computer").example;

    assert_eq!(
        code(example),
        "shell = get_shell\ncomputer = shell.host_computer\nprint(\"Computer public IP is: \" + computer.public_ip)"
    );

    let get_shell = example
        .iter()
        .find(|token| token.text == "get_shell")
        .expect("get_shell");

    assert_eq!(get_shell.highlight, Highlight::Name);
    assert_eq!(
        get_shell.link,
        Some(Link::Member {
            owner: "general".into(),
            name: "get_shell".into()
        })
    );

    let host_computer = example
        .iter()
        .find(|token| token.text == "host_computer")
        .expect("member");

    assert_eq!(host_computer.link, None);
    assert!(
        example
            .iter()
            .any(|token| token.highlight == Highlight::String)
    );
    assert!(
        example
            .iter()
            .any(|token| token.highlight == Highlight::Operator && token.text == "+")
    );
}

#[test]
fn should_keep_every_example_whole() {
    let mut harness = Harness::new();

    let (_, sections) = harness.reference();

    for section in &sections {
        for member in &section.members {
            assert!(
                member.example.iter().all(|token| !token.text.is_empty()),
                "{}.{}",
                section.name,
                member.name
            );
        }
    }

    let general = section(&sections, "general");

    assert!(
        general
            .members
            .iter()
            .filter(|member| !member.example.is_empty())
            .count()
            > 40
    );
}

#[test]
fn should_render_headings_code_and_links_to_the_page() {
    let mut harness = Harness::new();

    let blocks = harness.render(
        "# Values\n\nSee [functions](#guide-functions) and [print](#GENERAL_PRINT).\n\n## Lists\n\n```\nprint [1, 2]\n```\n\n- `a`\n\n  ```\nb\n  ```\n",
    );

    assert_eq!(
        text(&blocks),
        "# Values\nSee functions and print.\n## Lists\nprint [1, 2]\n* a\nb\n"
    );
    assert_eq!(
        links(&blocks),
        [
            (
                "functions".to_owned(),
                Link::Page {
                    anchor: "guide-functions".into()
                }
            ),
            (
                "print".to_owned(),
                Link::Member {
                    owner: "general".into(),
                    name: "print".into()
                }
            ),
        ]
    );

    let Some(Block::Code { tokens, .. }) = blocks.get(3) else {
        panic!("a code block: {blocks:?}");
    };

    assert_eq!(code(tokens), "print [1, 2]");
    assert_eq!(
        tokens
            .first()
            .map(|token| (token.highlight, token.link.clone())),
        Some((
            Highlight::Name,
            Some(Link::Member {
                owner: "general".into(),
                name: "print".into()
            })
        ))
    );
}

#[test]
fn should_render_code_of_another_language_as_plain_text() {
    let mut harness = Harness::new();

    let blocks = harness.render("```json\n{ \"print\": 1 }\n```\n\n```greyscript\nprint 1\n```\n");

    let [
        Block::Code {
            tokens: json,
            language: Some(language),
        },
        Block::Code {
            tokens: script,
            language: None,
        },
    ] = blocks.as_slice()
    else {
        panic!("two code blocks: {blocks:?}");
    };

    assert_eq!(language, "json");
    assert_eq!(
        json.iter()
            .map(|token| (token.text.as_str(), token.highlight, token.link.is_some()))
            .collect::<Vec<_>>(),
        [("{ \"print\": 1 }", Highlight::Plain, false)]
    );
    assert_eq!(
        script.first().map(|token| token.highlight),
        Some(Highlight::Name)
    );
}

#[test]
fn should_render_a_table_with_its_header_and_rows() {
    let mut harness = Harness::new();

    let blocks = harness.render(
        "| Command | What it does |\n|---|---|\n| `greybel build` | builds [print](#GENERAL_PRINT) |\n| `greybel repl` | runs code |\n",
    );

    assert_eq!(
        text(&blocks),
        "Command | What it does\ngreybel build | builds print\ngreybel repl | runs code"
    );
    assert!(matches!(blocks.as_slice(), [Block::Table { rows, .. }] if rows.len() == 2));
    assert_eq!(
        links(&blocks),
        [(
            "print".to_owned(),
            Link::Member {
                owner: "general".into(),
                name: "print".into()
            }
        )]
    );
}
