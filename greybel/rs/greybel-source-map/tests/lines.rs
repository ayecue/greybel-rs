use greybel_source_map::{Mapping, SourceMap};

use crate::support::at;

#[test]
fn should_join_consecutive_lines_of_a_source_into_one_mapping() {
    let map = SourceMap::of_lines([
        Some(at("main.src", 1)),
        Some(at("main.src", 2)),
        None,
        Some(at("lib.src", 4)),
        Some(at("lib.src", 5)),
        Some(at("main.src", 3)),
    ]);

    assert_eq!(map.sources(), ["main.src", "lib.src"]);
    assert_eq!(
        map.mappings(),
        [
            Mapping::new(1, 2, 0, 1),
            Mapping::new(4, 5, 1, 4),
            Mapping::new(6, 6, 0, 3),
        ]
    );
}

#[test]
fn should_start_a_new_mapping_where_the_source_lines_jump() {
    let map = SourceMap::of_lines([Some(at("main.src", 1)), Some(at("main.src", 5))]);

    assert_eq!(
        map.mappings(),
        [Mapping::new(1, 1, 0, 1), Mapping::new(2, 2, 0, 5)]
    );
}

#[test]
fn should_map_every_line_of_an_entire_source_with_one_mapping() {
    let map = SourceMap::entire("lib.src", 3);

    assert_eq!(map.mappings(), [Mapping::new(1, 3, 0, 1)]);
}

#[test]
fn should_rename_the_sources() {
    let map = SourceMap::entire("lib.src", 1).map_sources(|source| format!("/home/{source}"));

    assert_eq!(map.sources(), ["/home/lib.src"]);
}
