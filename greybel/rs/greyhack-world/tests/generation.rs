use greyhack_behaviour::{DeviceKind, NetworkKind, ServiceId};
use greyhack_world::{
    ComputerOptions, NetworkOptions, PlayerOptions, RecordKind, SmartApplianceSettings, World,
    WorldOptions,
};
use greyhack_world_assets::content_file;

use crate::support::generation::add_typed_network;

#[tokio::test]
async fn should_configure_services_on_generated_computers() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = world
        .add_network(&NetworkOptions {
            computers: Some(0),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();
    let computer = world
        .add_computer_to(
            router,
            &ComputerOptions {
                services: Some(vec![
                    "ssh".to_owned(),
                    "smtp".to_owned(),
                    "criminals".to_owned(),
                ]),
                ..ComputerOptions::default()
            },
        )
        .await
        .unwrap()
        .unwrap();

    for service in [ServiceId::Ssh, ServiceId::Smtp, ServiceId::Criminals] {
        assert!(
            world
                .file(computer, service.config_path(), false)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            world
                .file(computer, service.executable_path(), false)
                .await
                .unwrap()
                .is_some()
        );
    }

    assert_eq!(
        world
            .content(computer, "/root/PoliceRecord.exe", false)
            .await
            .unwrap(),
        Some(format!(
            "{}:3306",
            world.computer(computer).await.unwrap().unwrap().local_ip
        ))
    );
}

#[tokio::test]
async fn should_generate_police_records_and_update_one_by_person_id() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let (router, computer) =
        add_typed_network(&mut world, NetworkKind::Police, "police.test").await;
    let network = world
        .network(world.router(router).await.unwrap().unwrap().network)
        .await
        .unwrap();

    assert_eq!(network.unwrap().kind, NetworkKind::Police);
    assert!(
        world
            .has_service(computer, ServiceId::Criminals)
            .await
            .unwrap()
    );

    let records = world
        .service_records(computer, ServiceId::Criminals)
        .await
        .unwrap()
        .unwrap();

    assert!((15..=30).contains(&records.records.len()));
    assert_eq!(records.records[0].kind, RecordKind::Police);
    assert!(!records.records[0].offenses.as_ref().unwrap().is_empty());
    assert_eq!(records.records[0].grades, None);

    let mut updated = records.records[0].clone();

    updated.age = 99;
    world
        .update_service_record(computer, ServiceId::Criminals, updated.clone())
        .await
        .unwrap()
        .unwrap();

    let records = world
        .service_records(computer, ServiceId::Criminals)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(records.records[0], updated);
}

#[tokio::test]
async fn should_generate_student_records_and_viewers() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let (_, computer) =
        add_typed_network(&mut world, NetworkKind::University, "university.test").await;
    let records = world
        .service_records(computer, ServiceId::Students)
        .await
        .unwrap()
        .unwrap();

    assert!((15..=30).contains(&records.records.len()));
    assert_eq!(records.records[0].kind, RecordKind::Student);
    assert!(!records.records[0].grades.as_ref().unwrap().is_empty());
    assert_eq!(records.records[0].offenses, None);
    assert!(
        world
            .file(computer, "/root/StudentsViewer.exe", false)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn should_generate_bank_mail_and_shop_services() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let (_, bank) = add_typed_network(&mut world, NetworkKind::Bank, "bank.test").await;
    let (_, mail) = add_typed_network(&mut world, NetworkKind::MailService, "mail.test").await;
    let (_, shop) = add_typed_network(&mut world, NetworkKind::HackShop, "shop.test").await;

    assert!(
        world
            .has_service(bank, ServiceId::BankAccount)
            .await
            .unwrap()
    );
    assert!(
        world
            .file(bank, ServiceId::BankAccount.config_path(), false)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        world
            .content(mail, ServiceId::Smtp.config_path(), false)
            .await
            .unwrap(),
        Some("mail.test".to_owned())
    );
    assert!(
        world
            .has_service(shop, ServiceId::Repository)
            .await
            .unwrap()
    );
    assert!(
        world
            .file(shop, "/Public/htdocs/downloads/decipher", false)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn should_create_device_kinds_and_npc_account_files() {
    let world = World::generate(&WorldOptions {
        seed: 7,
        networks: 16,
        ..WorldOptions::default()
    })
    .await
    .unwrap();
    let routers = world.public_addresses().await.unwrap();
    let mut kinds = std::collections::HashSet::new();
    let mut checked_npc = false;

    for address in routers {
        let Some(router) = world.router_at(&address).await.unwrap() else {
            continue;
        };

        for computer in world.devices(router.id).await.unwrap() {
            let row = world.computer(computer).await.unwrap().unwrap();

            kinds.insert(row.kind);

            match row.kind {
                DeviceKind::Camera => {
                    assert!(world.has_service(computer, ServiceId::Cam).await.unwrap());
                    assert_eq!(
                        world
                            .content(computer, "/Public/htdocs/website.html", false)
                            .await
                            .unwrap(),
                        Some(content_file("cctv").to_owned())
                    );
                    assert!(
                        world
                            .users(computer)
                            .await
                            .unwrap()
                            .iter()
                            .any(|user| user.name == "cctv")
                    );
                }
                DeviceKind::Microwave | DeviceKind::Fridge => {
                    assert!(world.smart(computer).await.unwrap().is_some());
                }
                DeviceKind::Smartphone => {
                    assert!(
                        world
                            .file(computer, "/usr/bin/Clock.exe", false)
                            .await
                            .unwrap()
                            .is_some()
                    );
                }
                _ => {}
            }

            if !checked_npc {
                let npcs = world.npcs_on(computer).await.unwrap();

                if let Some(npc) = npcs.first() {
                    let person = world.person(npc.person).await.unwrap().unwrap();
                    let mail = person.mail_address.unwrap();
                    let bank = person.bank_number.unwrap();
                    let mail_account = world.mail_account(&mail).await.unwrap().unwrap();
                    let bank_account = world.bank_account(&bank).await.unwrap().unwrap();
                    let prefix = format!("/home/{}/Config", npc.user);

                    assert_eq!(
                        world
                            .content(computer, &format!("{prefix}/Mail.txt"), false)
                            .await
                            .unwrap(),
                        Some(format!("{mail}:{}", mail_account.password_hash))
                    );
                    assert_eq!(
                        world
                            .content(computer, &format!("{prefix}/Bank.txt"), false)
                            .await
                            .unwrap(),
                        Some(format!("{bank}:{}", bank_account.password_hash))
                    );

                    let mut phone_mail = false;

                    for device in world.devices(router.id).await.unwrap() {
                        if world.computer(device).await.unwrap().unwrap().kind
                            != DeviceKind::Smartphone
                            || !world
                                .users(device)
                                .await
                                .unwrap()
                                .iter()
                                .any(|user| user.name == npc.user)
                        {
                            continue;
                        }

                        assert_eq!(
                            world
                                .content(device, &format!("{prefix}/Mail.txt"), false)
                                .await
                                .unwrap(),
                            Some(format!("{mail}:{}", mail_account.password_hash))
                        );
                        phone_mail = true;
                    }

                    assert!(phone_mail);
                    checked_npc = true;
                }
            }
        }
    }

    for kind in [
        DeviceKind::Camera,
        DeviceKind::Microwave,
        DeviceKind::Fridge,
        DeviceKind::Smartphone,
    ] {
        assert!(kinds.contains(&kind), "missing generated {kind:?}");
    }

    assert!(checked_npc);
}

#[tokio::test]
async fn should_authenticate_cctv_and_install_adb_only_on_smartphones() {
    let mut world = World::generate(&WorldOptions {
        seed: 7,
        networks: 8,
        ..WorldOptions::default()
    })
    .await
    .unwrap();
    let mut camera = None;
    let mut phone = None;

    for address in world.public_addresses().await.unwrap() {
        let Some(router) = world.router_at(&address).await.unwrap() else {
            continue;
        };

        for device in world.devices(router.id).await.unwrap() {
            let kind = world.computer(device).await.unwrap().unwrap().kind;

            if kind == DeviceKind::Camera && camera.is_none() {
                camera = Some(device);
            }

            if kind == DeviceKind::Smartphone && phone.is_none() {
                phone = Some(device);
            }
        }
    }

    let camera = camera.expect("a generated camera");
    let phone = phone.expect("a generated smartphone");

    assert_eq!(
        world.access_cctv(camera, None).await.unwrap().unwrap(),
        None
    );
    assert_eq!(
        world.access_cctv(camera, Some("wrong")).await.unwrap(),
        Err(greyhack_world::CctvFailure::WrongPassword)
    );

    let password = world
        .users(camera)
        .await
        .unwrap()
        .into_iter()
        .find(|user| user.name == "cctv")
        .unwrap()
        .password;
    let access = world
        .access_cctv(camera, Some(&password))
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert_eq!(
        access.local_ip,
        world.computer(camera).await.unwrap().unwrap().local_ip
    );

    let router = world.router_of(camera).await.unwrap().unwrap();
    let appliance = world.devices(router.id).await.unwrap()[0];

    world
        .save_smart(
            appliance,
            Some(&SmartApplianceSettings::new("test/FR1", 600, 20)),
        )
        .await
        .unwrap();

    let mut broken = world.smart(appliance).await.unwrap().unwrap();

    broken.recovery_at = world.time_seconds() + 1;
    world.save_smart(appliance, Some(&broken)).await.unwrap();
    assert_eq!(
        world.access_cctv(camera, Some(&password)).await.unwrap(),
        Err(greyhack_world::CctvFailure::NoExternalAccess)
    );
    world.save_smart(appliance, None).await.unwrap();

    assert!(!world.install_smartphone_adb(camera).await.unwrap());
    assert!(world.install_smartphone_adb(phone).await.unwrap());
    assert!(world.has_service(phone, ServiceId::Adb).await.unwrap());
    assert!(
        world
            .file(phone, ServiceId::Adb.config_path(), false)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        world
            .file(phone, ServiceId::Adb.executable_path(), false)
            .await
            .unwrap()
            .is_some()
    );
}
