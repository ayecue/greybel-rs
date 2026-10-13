use greybel_source_map::{Location, SourceMap};
use greyhack_behaviour::{Import, ImportedCodeTooLarge, UnclosedImport, import_paths, put_imports};

#[test]
fn should_find_every_import_but_one_right_after_a_comment() {
    let code = "import_code(\"/a.src\")\n//import_code(\"/b.src\")\n// import_code(\"/c.src\")\ns = \"import_code(/d.src)\"";

    assert_eq!(
        import_paths(code),
        Ok(vec![
            "/a.src".to_owned(),
            "/c.src".to_owned(),
            "/d.src".to_owned()
        ])
    );
}

#[test]
fn should_refuse_an_import_without_a_closing_parenthesis() {
    assert_eq!(import_paths("import_code(\"/a.src\""), Err(UnclosedImport));
}

#[test]
fn should_put_an_import_in_place_of_its_line_in_the_source_map() {
    let import = Import {
        path: "/lib.src".to_owned(),
        code: "a = 1\nb = 2".to_owned(),
        source_map: SourceMap::entire("lib.src", 2),
    };
    let (code, map) = put_imports(
        "print 1\nimport_code(\"/lib.src\")\nprint 2".to_owned(),
        SourceMap::entire("", 3),
        &[import],
        None,
    )
    .unwrap();

    assert_eq!(code, "print 1\na = 1\nb = 2\nprint 2");
    assert_eq!(map.locate(1), Some(Location::new("", 1)));
    assert_eq!(map.locate(3), Some(Location::new("lib.src", 2)));
    assert_eq!(map.locate(4), Some(Location::new("", 3)));
}

#[test]
fn should_refuse_code_that_grows_past_the_limit() {
    let import = Import {
        path: "/lib.src".to_owned(),
        code: "x".repeat(20),
        source_map: SourceMap::default(),
    };
    let result = put_imports(
        "import_code(\"/lib.src\")".to_owned(),
        SourceMap::default(),
        &[import],
        Some(10),
    );

    assert_eq!(result, Err(ImportedCodeTooLarge));
}
