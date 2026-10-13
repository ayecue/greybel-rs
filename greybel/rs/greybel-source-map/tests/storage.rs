use greybel_source_map::{Mapping, SourceMap};

use crate::support::at;

#[test]
fn should_store_a_map_as_its_sources_and_mapping_arrays() {
    let map = SourceMap::of_lines([at("main.src", 1), at("main.src", 2), at("lib.src", 7)]);

    let json = serde_json::to_string(&map).unwrap();

    assert_eq!(
        json,
        r#"{"sources":["main.src","lib.src"],"mappings":[[1,2,0,1],[3,3,1,7]]}"#
    );
    assert_eq!(serde_json::from_str::<SourceMap>(&json).unwrap(), map);
}

#[test]
fn should_drop_stored_mappings_naming_no_source_or_overlapping_an_earlier_one() {
    let json = r#"{"sources":["main.src"],"mappings":[[3,4,0,1],[1,1,2,1],[4,5,0,9]]}"#;

    let map: SourceMap = serde_json::from_str(json).unwrap();

    assert_eq!(map.mappings(), [Mapping::new(3, 4, 0, 1)]);
    assert_eq!(map.locate(4), at("main.src", 2));
}
