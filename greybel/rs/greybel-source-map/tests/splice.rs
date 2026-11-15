use greybel_source_map::SourceMap;

use crate::support::at;

/// Matches the line mapping produced by Grey Hack's `HelperImport.GetInfoErrorUpdated` for imported binaries.
#[test]
fn should_put_the_lines_of_an_import_in_place_of_its_line() {
    let mut map = SourceMap::entire("", 4);

    map.splice(2, &SourceMap::entire("lib.src", 3), 3);

    let lines: Vec<_> = (1..=7).map(|line| map.locate(line)).collect();

    assert_eq!(
        lines,
        [
            Some(at("", 1)),
            Some(at("lib.src", 1)),
            Some(at("lib.src", 2)),
            Some(at("lib.src", 3)),
            Some(at("", 3)),
            Some(at("", 4)),
            None,
        ]
    );
}

#[test]
fn should_keep_the_mappings_of_an_import_inside_a_mapped_line() {
    let mut map = SourceMap::of_lines([Some(at("main.src", 1)), Some(at("main.src", 2))]);
    let import = SourceMap::of_lines([None, Some(at("inner.src", 5))]);

    map.splice(1, &import, 2);

    assert_eq!(map.locate(1), None);
    assert_eq!(map.locate(2), Some(at("inner.src", 5)));
    assert_eq!(map.locate(3), Some(at("main.src", 2)));
}

#[test]
fn should_leave_out_the_mappings_of_an_import_past_its_lines() {
    let mut map = SourceMap::entire("", 1);

    map.splice(1, &SourceMap::entire("lib.src", 5), 2);

    assert_eq!(map.locate(2), Some(at("lib.src", 2)));
    assert_eq!(map.locate(3), None);
}
