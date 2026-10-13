use crate::support::scan::scan_world;
use crate::support::scripts::run_on_host;
use crate::support::{library, set_library};

#[test]
fn should_hide_weaknesses_from_an_outdated_metaxploit() {
    let (world, _, code) = scan_world("0.0.1", "1.0.0");

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert!(output.starts_with("**Warning:"), "{output}");
    assert!(output.contains("\n0\n"), "{output}");
    assert!(
        output.contains("No vulnerabilities were found."),
        "{output}"
    );
}

#[test]
fn should_scan_weaknesses_with_a_current_metaxploit() {
    let (world, _, code) = scan_world("1.0.0", "1.0.0");
    // The world's newest SSH library is newer than this 1.0.0 file, so hidden zones are included.
    let expected = library(&world, world.player(), "/lib/libssh.so")
        .zones
        .len();

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert!(output.starts_with(&format!("{expected}\n")), "{output}");
    assert!(output.contains("Unsafe check:"), "{output}");
}

#[test]
fn should_include_hidden_zones_of_an_outdated_library_and_warn() {
    let (world, _, code) = scan_world("1.0.0", "0.9.0");

    let mut outdated = library(&world, world.player(), "/lib/libssh.so");

    outdated.version = "0.8.0".to_owned();
    outdated
        .zones
        .iter_mut()
        .find(|zone| zone.hide)
        .unwrap()
        .weaknesses[0]
        .metaxploit_version = "9.9.9".to_owned();
    set_library(&world, world.player(), "/lib/libssh.so", outdated);

    let expected = library(&world, world.player(), "/lib/libssh.so")
        .zones
        .len();

    let (output, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert!(output.contains("**Warning:"), "{output}");
    assert!(output.contains(&format!("{expected}\n")), "{output}");
}
