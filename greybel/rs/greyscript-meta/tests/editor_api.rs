//! Tests the names Grey Hack's code editor colors as API functions.

#[test]
fn should_name_grey_hacks_api_without_miniscript_intrinsics() {
    for name in [
        "print",
        "wait",
        "typeof",
        "get_shell",
        "host_computer",
        "public_ip",
        "File",
    ] {
        assert!(greyscript_meta::is_api_function(name), "{name}");
    }

    for name in [
        "push",
        "len",
        "to_int",
        "str",
        "range",
        "time",
        "number",
        "map",
        "get_greybel_util",
        "x",
    ] {
        assert!(!greyscript_meta::is_api_function(name), "{name}");
    }
}
