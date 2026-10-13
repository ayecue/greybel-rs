use greyhack_world_assets::{
    PublicService, ServiceDefinition, content_file, public_services, website_default,
};

#[test]
fn should_read_the_games_service_paths() {
    let name = "ssh";

    let service = ServiceDefinition::find(name);

    let service = service.expect("ssh is in the game table");

    assert_eq!(service.port, 22);
    assert_eq!(service.daemon, "sshd");
    assert_eq!(service.config_path, "/server/conf/sshd.conf");
    assert_eq!(service.executable_path, "/server/sshd");
}

#[test]
fn should_have_a_definition_for_every_script_service() {
    let names = ["ftp", "http", "smtp", "repository"];

    let paths: Vec<_> = names
        .iter()
        .map(|name| ServiceDefinition::find(name).map(|service| service.config_path.as_str()))
        .collect();

    assert_eq!(
        paths,
        [
            Some("/server/conf/ftpd.conf"),
            Some("/server/conf/httpd.conf"),
            Some("/server/conf/smtpd.conf"),
            Some("/server/conf/repod.conf"),
        ]
    );
}

#[test]
fn should_read_a_network_kinds_public_services() {
    let supermarket = public_services("Supermercados");

    assert_eq!(
        supermarket,
        [
            PublicService {
                service: 2,
                chance: 30,
                closed: false
            },
            PublicService {
                service: 3,
                chance: 20,
                closed: false
            },
            PublicService {
                service: 5,
                chance: 100,
                closed: true
            },
            PublicService {
                service: 0,
                chance: 100,
                closed: false
            },
        ]
    );
    assert!(public_services("CTF").is_empty());
}

#[test]
fn should_read_the_games_websites_by_name() {
    assert!(content_file("Router").contains("<h1>Router configuration</h1>"));
    assert_eq!(content_file("NoSuchFile"), website_default());
}
