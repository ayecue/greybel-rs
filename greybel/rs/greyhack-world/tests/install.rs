use greyhack_behaviour::ServiceId;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_assets::{ssh_decode, ssh_encode};

#[tokio::test]
async fn should_install_ssh_with_its_config_and_port() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.handle.player();

    let installed = world
        .modules
        .services
        .installs
        .install_service(player, ServiceId::Ssh, "1.2.0")
        .await
        .unwrap();

    assert!(installed);
    assert!(
        world
            .modules
            .core
            .computers
            .processes
            .has_service(player, ServiceId::Ssh)
            .await
            .unwrap()
    );

    let port = world
        .modules
        .core
        .computers
        .processes
        .ports(player)
        .await
        .unwrap()
        .into_iter()
        .find(|port| port.service == ServiceId::Ssh)
        .unwrap();

    assert_eq!((port.number, port.version.as_str()), (22, "1.2.0"));
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .content(player, "/server/conf/sshd.conf", false)
            .await
            .unwrap()
            .unwrap()
            .contains("\"encryption_enabled\": false")
    );
    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .content(player, "/server/encode.src", false)
            .await
            .unwrap(),
        Some(ssh_encode().to_owned())
    );
    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .content(player, "/server/decode.bin", false)
            .await
            .unwrap(),
        Some(ssh_decode().to_owned())
    );
    assert!(
        !world
            .modules
            .core
            .computers
            .processes
            .processes(player)
            .await
            .unwrap()
            .iter()
            .any(|process| process.name == "sshd")
    );
}

#[tokio::test]
async fn should_install_the_repository_packages() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.handle.player();

    world
        .modules
        .services
        .installs
        .install_service(player, ServiceId::Repository, "1.0.0")
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .content(player, "/server/aptfiles/helloworld", false)
            .await
            .unwrap(),
        Some("print(\"hello world!\")".to_owned())
    );
    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .content(player, "/server/aptfiles/heythere", false)
            .await
            .unwrap(),
        Some("print(\"hey there!\")".to_owned())
    );
}

#[tokio::test]
async fn should_configure_smtp_with_the_routers_domain() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.handle.player();
    let domain = world
        .modules
        .core
        .networks
        .topology
        .router_of(player)
        .await
        .unwrap()
        .map(|router| router.domain)
        .filter(|domain| !domain.is_empty())
        .unwrap_or_else(|| greyhack_behaviour::DEFAULT_MAIL_DOMAIN.to_owned());

    world
        .modules
        .services
        .installs
        .install_service(player, ServiceId::Smtp, "1.0.0")
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .content(player, "/server/conf/smtpd.conf", false)
            .await
            .unwrap(),
        Some(domain)
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .file(player, "/server/conf/UserConfig.sys", false)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn should_not_add_a_second_port_when_installing_again() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.handle.player();

    world
        .modules
        .services
        .installs
        .install_service(player, ServiceId::Ssh, "1.0.0")
        .await
        .unwrap();

    world
        .modules
        .services
        .installs
        .install_service(player, ServiceId::Ssh, "1.0.0")
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .computers
            .processes
            .ports(player)
            .await
            .unwrap()
            .iter()
            .filter(|port| port.service == ServiceId::Ssh)
            .count(),
        1
    );
}
