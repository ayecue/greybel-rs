use greyhack_behaviour::{
    APT_BINARY_FOLDER, APT_LIBRARY_FOLDER, AptSource, DEFAULT_REPOSITORY_PORT,
    default_apt_install_folder, format_apt_sources, parse_apt_official, parse_apt_sources,
};

#[test]
fn should_parse_and_format_apt_sources() {
    let repositories = vec![
        AptSource {
            address: "45.45.45.45".to_owned(),
            port: 1542,
        },
        AptSource {
            address: "8.8.8.8".to_owned(),
            port: 1543,
        },
    ];
    let content = format_apt_sources(&repositories, false);

    assert_eq!(parse_apt_sources(&content), repositories);
    assert!(!parse_apt_official(&content));
    assert_eq!(
        parse_apt_sources("hackshop.org\n\npackages.test\n"),
        vec![
            AptSource {
                address: "hackshop.org".to_owned(),
                port: DEFAULT_REPOSITORY_PORT,
            },
            AptSource {
                address: "packages.test".to_owned(),
                port: DEFAULT_REPOSITORY_PORT,
            },
        ]
    );
    assert!(parse_apt_official(&format_apt_sources(&[], true)));
}

#[test]
fn should_use_the_package_kind_for_its_default_install_folder() {
    assert_eq!(default_apt_install_folder(true), APT_LIBRARY_FOLDER);
    assert_eq!(default_apt_install_folder(false), APT_BINARY_FOLDER);
}
