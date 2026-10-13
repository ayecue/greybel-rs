use greybel_source_map::SourceMap;

use crate::support::at;

#[test]
fn should_locate_a_line_in_its_source() {
    let map = SourceMap::of_lines([at("main.src", 1), None, at("lib.src", 7), at("lib.src", 8)]);

    assert_eq!(map.locate(1), at("main.src", 1));
    assert_eq!(map.locate(2), None);
    assert_eq!(map.locate(4), at("lib.src", 8));
    assert_eq!(map.locate(5), None);
}
