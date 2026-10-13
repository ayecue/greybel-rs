use std::rc::Rc;

use duplex_test_support::local::run_local;
use greyhack_behaviour::{DeviceKind, ServiceId, hash_password};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_operations::WorldServer;
use scheduler::ManualClock;
use serde_json::json;

use crate::support::admin_computer::player_server;
use crate::support::connect;
use crate::support::program::{
    AdminAccessCctv, AdminAddNetCard, AdminInstallSmartphoneAdb, AdminNpcLogin,
    AdminRegisterPassword, AdminServiceRecords, AdminSetLibraryVersion, AdminSetPortLibraryVersion,
    AdminUpdateServiceRecord, CctvAccess, Outcome,
};

#[test]
fn should_expose_world_setup_apis_through_operations() {
    run_local(async {
        let (server, public_ip, lan_ip, _) = player_server().await;
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;

        let added = runtime
            .channel
            .send(&AdminAddNetCard {
                program: session.program,
                public_ip: public_ip.clone().into(),
                lan_ip: lan_ip.clone().into(),
                wifi: true,
                monitor_support: true,
            })
            .unwrap()
            .await;
        let file_version = runtime
            .channel
            .send(&AdminSetLibraryVersion {
                program: session.program,
                public_ip: public_ip.clone().into(),
                lan_ip: lan_ip.clone().into(),
                path: "/lib/libssh.so".into(),
                version: "9.8.7".into(),
            })
            .unwrap()
            .await;
        let port_version = runtime
            .channel
            .send(&AdminSetPortLibraryVersion {
                program: session.program,
                public_ip: public_ip.into(),
                lan_ip: lan_ip.into(),
                port: 22,
                version: "8.7.6".into(),
            })
            .unwrap()
            .await;
        let hash = runtime
            .channel
            .send(&AdminRegisterPassword {
                program: session.program,
                password: "standalone".into(),
            })
            .unwrap()
            .await;

        assert_eq!(added, Ok(true));
        assert_eq!(file_version, Ok(true));
        assert_eq!(port_version, Ok(true));
        assert_eq!(hash, Ok(hash_password("standalone")));

        let world = server.world().await;
        let player = world.player();

        assert!(world.net_cards(player).await.unwrap().last().unwrap().wifi);
        assert_eq!(
            world.port(player, 22).await.unwrap().unwrap().version,
            "8.7.6"
        );
        assert_eq!(
            world
                .password_of_hash(&hash_password("standalone"))
                .await
                .unwrap(),
            Some("standalone".to_owned())
        );
    });
}

#[test]
fn should_expose_sql_viewer_read_and_update_channels() {
    run_local(async {
        let mut world = World::new(&PlayerOptions::default()).await.unwrap();
        let player = world.player();

        world
            .install_service(player, ServiceId::Criminals, "1.0.0")
            .await
            .unwrap();

        let mut database = world
            .file(player, ServiceId::Criminals.config_path(), false)
            .await
            .unwrap()
            .unwrap();

        database.data_mut().unwrap().content = serde_json::to_string(&json!({
            "records": [{"kind": 2, "person_id": "person-1", "full_name": "Before"}]
        }))
        .unwrap();
        world.save_node(&database).await.unwrap();

        let public_ip = world.public_ip(player).await.unwrap();
        let lan_ip = world.computer(player).await.unwrap().unwrap().local_ip;
        let server = WorldServer::new(world, Rc::new(ManualClock::constant()));
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;

        let read = runtime
            .channel
            .send(&AdminServiceRecords {
                program: session.program,
                public_ip: public_ip.clone().into(),
                lan_ip: lan_ip.clone().into(),
                service: "criminals".into(),
            })
            .unwrap()
            .await
            .unwrap()
            .unwrap();

        assert!(read.contains("Before"));

        let updated = runtime
            .channel
            .send(&AdminUpdateServiceRecord {
                program: session.program,
                public_ip: public_ip.clone().into(),
                lan_ip: lan_ip.clone().into(),
                service: "criminals".into(),
                record: serde_json::to_string(&json!({
                    "kind": 2, "person_id": "person-1", "full_name": "After"
                }))
                .unwrap()
                .into(),
            })
            .unwrap()
            .await;

        assert_eq!(updated, Ok(Outcome::True));

        let read = runtime
            .channel
            .send(&AdminServiceRecords {
                program: session.program,
                public_ip: public_ip.into(),
                lan_ip: lan_ip.into(),
                service: "criminals".into(),
            })
            .unwrap()
            .await
            .unwrap()
            .unwrap();

        assert!(read.contains("After"));
        assert!(!read.contains("Before"));
    });
}

#[test]
fn should_expose_the_cctv_browser_channel() {
    run_local(async {
        let mut world = World::new(&PlayerOptions::default()).await.unwrap();
        let player = world.player();
        let mut computer = world.computer(player).await.unwrap().unwrap();

        computer.kind = DeviceKind::Camera;
        world.save_computer(&computer).await.unwrap();

        let public_ip = world.public_ip(player).await.unwrap();
        let lan_ip = computer.local_ip.clone();
        let password = world.users(player).await.unwrap()[0].password.clone();
        let server = WorldServer::new(world, Rc::new(ManualClock::constant()));
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;

        let prompt = runtime
            .channel
            .send(&AdminAccessCctv {
                program: session.program,
                public_ip: public_ip.clone().into(),
                lan_ip: lan_ip.clone().into(),
                password: None,
            })
            .unwrap()
            .await;

        assert_eq!(prompt, Ok(CctvAccess::Prompt));

        let access = runtime
            .channel
            .send(&AdminAccessCctv {
                program: session.program,
                public_ip: public_ip.clone().into(),
                lan_ip: lan_ip.clone().into(),
                password: Some(password.into()),
            })
            .unwrap()
            .await;

        assert!(matches!(access, Ok(CctvAccess::Data(_))));
    });
}

#[test]
fn should_expose_smartphone_adb_installation() {
    run_local(async {
        let mut world = World::new(&PlayerOptions::default()).await.unwrap();
        let player = world.player();
        let mut computer = world.computer(player).await.unwrap().unwrap();

        computer.kind = DeviceKind::Smartphone;
        world.save_computer(&computer).await.unwrap();

        let public_ip = world.public_ip(player).await.unwrap();
        let lan_ip = computer.local_ip.clone();
        let server = WorldServer::new(world, Rc::new(ManualClock::constant()));
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;

        let installed = runtime
            .channel
            .send(&AdminInstallSmartphoneAdb {
                program: session.program,
                public_ip: public_ip.into(),
                lan_ip: lan_ip.into(),
            })
            .unwrap()
            .await;

        assert_eq!(installed, Ok(true));
        assert!(
            server
                .world()
                .await
                .has_service(player, ServiceId::Adb)
                .await
                .unwrap()
        );
    });
}

/// `AdminNpcLogin` resolves the target computer and triggers the social-engineering login, returning whether it was found.
#[test]
fn should_route_an_admin_npc_login_to_its_computer() {
    run_local(async {
        let (server, public_ip, lan_ip, _) = player_server().await;
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;
        let user = server
            .change_world(async |world| {
                let player = world.player();

                world.users(player).await.unwrap()[0].name.clone()
            })
            .await;

        let found = runtime
            .channel
            .send(&AdminNpcLogin {
                program: session.program,
                address: public_ip.clone().into(),
                lan_ip: lan_ip.clone().into(),
                user: user.clone().into(),
            })
            .unwrap()
            .await;
        let missing = runtime
            .channel
            .send(&AdminNpcLogin {
                program: session.program,
                address: "203.0.113.1".into(),
                lan_ip: "192.168.0.99".into(),
                user: user.into(),
            })
            .unwrap()
            .await;

        assert_eq!(found, Ok(true));
        assert_eq!(missing, Ok(false));
    });
}
