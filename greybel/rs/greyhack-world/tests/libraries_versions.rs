use greyhack_behaviour::{
    LibraryKind, MAX_WEAKNESS_USES, ROOT_USER, Reward, library_check_seconds,
};
use greyhack_world::{PlayerOptions, WeaknessReportFailure, World};

use crate::support::exploit_shop::{VERSION, offers, shop_world};
use crate::support::exploits::{ADDRESS, LIBRARY_PATH, UNSEC_VALUE, install_weakness, weakness};
use crate::support::library_versions::{SSH, hidden_world, newest, use_up};

#[tokio::test]
async fn should_release_nothing_while_no_weakness_is_used_up() {
    let (world, _) = shop_world().await;

    for _ in 0..MAX_WEAKNESS_USES - 1 {
        world
            .modules
            .libraries
            .versions
            .count_weakness_use(SSH, "0x10", "buffer")
            .await
            .unwrap();
    }

    assert!(
        world
            .modules
            .libraries
            .versions
            .check_library_versions()
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(newest(&world, SSH).await.version, VERSION);
}

#[tokio::test]
async fn should_release_the_next_version_when_a_weakness_is_used_up() {
    let (mut world, _) = shop_world().await;
    let metaxploit = newest(&world, LibraryKind::Metaxploit).await.version;

    use_up(&mut world, "buffer").await;

    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .check_library_versions()
            .await
            .unwrap(),
        [SSH]
    );

    let library = newest(&world, SSH).await;

    assert_eq!(library.version, "9.0.1");
    assert!(library.weakness("0x10", "buffer").is_none());
    assert!(library.weakness("0x10", "loop").is_some());
    assert!(
        library.zones.iter().filter(|zone| zone.hide).count() > 1,
        "new hidden zones take the place of the old"
    );
    assert_eq!(
        newest(&world, LibraryKind::Metaxploit).await.version,
        greyhack_behaviour::next_version(&metaxploit)
    );
}

#[tokio::test]
async fn should_make_the_weaknesses_of_a_release_need_the_metaxploit_of_the_time() {
    let (mut world, _) = shop_world().await;
    let metaxploit = newest(&world, LibraryKind::Metaxploit).await.version;
    let next = greyhack_behaviour::next_version(&metaxploit);

    use_up(&mut world, "buffer").await;
    world
        .modules
        .libraries
        .versions
        .check_library_versions()
        .await
        .unwrap();

    let library = newest(&world, SSH).await;
    let renewed = library
        .zones
        .iter()
        .find(|zone| zone.address == "0x10")
        .unwrap()
        .weaknesses
        .iter()
        .find(|weakness| weakness.unsec_value != "loop")
        .unwrap();

    assert_eq!(renewed.metaxploit_version, metaxploit);
    for zone in library.zones.iter().filter(|zone| zone.hide) {
        for weakness in &zone.weaknesses {
            assert_eq!(weakness.metaxploit_version, next);
        }
    }
}

#[tokio::test]
async fn should_target_only_their_version_by_the_exploits_of_a_patched_weakness() {
    let (mut world, _) = shop_world().await;

    for offer in offers(&mut world).await {
        assert!(offer.description.contains("libssh.so >= v9.0.0"));
    }

    use_up(&mut world, "buffer").await;
    world
        .modules
        .libraries
        .versions
        .check_library_versions()
        .await
        .unwrap();

    let old = offers(&mut world).await;
    let shell = old
        .iter()
        .find(|offer| offer.description.starts_with("Get access to a shell."))
        .expect("the exploit of the patched weakness");
    let other = old
        .iter()
        .find(|offer| !offer.description.starts_with("Get access to a shell."))
        .expect("the exploit of the other weakness");

    assert!(shell.description.contains("libssh.so = v9.0.0"));
    assert!(other.description.contains("libssh.so >= v9.0.0"));

    let next = world
        .modules
        .libraries
        .shop
        .exploit_offers(SSH, "9.0.1")
        .await
        .unwrap()
        .expect("the new version has exploits");

    assert_eq!(next.len(), 2);
    for offer in &next {
        assert!(offer.description.contains("libssh.so >= v9.0.1"));
    }
}

#[tokio::test]
async fn should_offer_the_hidden_exploits_of_a_version_once_a_newer_one_replaces_it() {
    let mut world = hidden_world().await;

    assert_eq!(offers(&mut world).await.len(), 2);

    use_up(&mut world, "buffer").await;
    world
        .modules
        .libraries
        .versions
        .check_library_versions()
        .await
        .unwrap();

    let old = offers(&mut world).await;

    assert_eq!(old.len(), 3);
    assert!(
        old.iter()
            .any(|offer| offer.description.contains("libssh.so = v9.0.0")
                && offer.description.contains("Permissions obtained: </b>root"))
    );
}

#[tokio::test]
async fn should_not_make_hidden_zones_for_a_router_kernel() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = LibraryKind::KernelRouter;
    let old = newest(&world, router).await;

    assert!(
        world
            .modules
            .libraries
            .versions
            .release_library_version(router)
            .await
            .unwrap()
            .is_some()
    );

    let library = newest(&world, router).await;

    assert!(library.zones.iter().all(|zone| !zone.hide));
    assert!(library.version != old.version);
    assert!(library.zones.iter().any(|zone| {
        zone.weaknesses.iter().any(|weakness| {
            matches!(weakness.reward, Reward::Shell | Reward::Computer)
                && weakness.user == "guest"
                && weakness.requirements.is_empty()
        })
    }));
}

#[tokio::test]
async fn should_give_metaxploit_two_versions_for_its_own_release() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let before = newest(&world, LibraryKind::Metaxploit).await.version;
    let released = world
        .modules
        .libraries
        .versions
        .release_library_version(LibraryKind::Metaxploit)
        .await
        .unwrap();

    assert_eq!(
        released,
        Some(greyhack_behaviour::next_version(
            &greyhack_behaviour::next_version(&before)
        ))
    );
}

#[tokio::test]
async fn should_run_the_check_every_fifteen_real_minutes_of_the_world_clock() {
    let (mut world, _) = shop_world().await;

    use_up(&mut world, "buffer").await;
    world.tick(library_check_seconds() - 1).await.unwrap();

    assert_eq!(newest(&world, SSH).await.version, VERSION);

    world.tick(1).await.unwrap();

    assert_eq!(newest(&world, SSH).await.version, "9.0.1");
    assert_eq!(
        world.modules.routines.next_due_at().await.unwrap(),
        Some(world.handle.time_seconds() + library_check_seconds())
    );
}

#[tokio::test]
async fn should_count_a_successful_overflow_as_a_use() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();

    install_weakness(&world, computer, weakness(Reward::Shell, ROOT_USER)).await;

    for attempt in 1..=3 {
        world
            .modules
            .libraries
            .overflows
            .overflow_file(computer, LIBRARY_PATH, ADDRESS, UNSEC_VALUE, "")
            .await
            .unwrap();

        assert_eq!(
            world
                .modules
                .libraries
                .versions
                .weakness_use_count(SSH, ADDRESS, UNSEC_VALUE)
                .await
                .unwrap(),
            attempt
        );
    }
}

#[tokio::test]
async fn should_not_count_an_overflow_that_gave_nothing() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();

    install_weakness(&world, computer, weakness(Reward::ChangePass, "guest")).await;
    world
        .modules
        .libraries
        .overflows
        .overflow_file(computer, LIBRARY_PATH, ADDRESS, UNSEC_VALUE, "new")
        .await
        .unwrap();
    world
        .modules
        .libraries
        .overflows
        .overflow_file(computer, LIBRARY_PATH, ADDRESS, "missing", "new")
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .weakness_use_count(SSH, ADDRESS, UNSEC_VALUE)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn should_count_uses_up_to_one_past_the_limit() {
    let (world, _) = shop_world().await;

    for _ in 0..MAX_WEAKNESS_USES + 5 {
        world
            .modules
            .libraries
            .versions
            .count_weakness_use(SSH, "0x10", "buffer")
            .await
            .unwrap();
    }

    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .weakness_use_count(SSH, "0x10", "buffer")
            .await
            .unwrap(),
        MAX_WEAKNESS_USES + 1
    );
}

#[tokio::test]
async fn should_ignore_the_use_of_a_weakness_the_newest_version_does_not_have() {
    let (world, _) = shop_world().await;

    world
        .modules
        .libraries
        .versions
        .count_weakness_use(SSH, "0x10", "missing")
        .await
        .unwrap();
    world
        .modules
        .libraries
        .versions
        .count_weakness_use(SSH, "0x99", "buffer")
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .weakness_use_count(SSH, "0x10", "missing")
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn should_accept_one_report_of_a_weakness_for_each_check() {
    let (world, _) = shop_world().await;

    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .report_weakness("libssh.so", "0x10", "buffer")
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .report_weakness("libssh.so", "0x10", "loop")
            .await
            .unwrap(),
        Err(WeaknessReportFailure::Pending),
        "the second report counts the weakness and waits"
    );
    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .weakness_use_count(SSH, "0x10", "loop")
            .await
            .unwrap(),
        MAX_WEAKNESS_USES
    );

    world.tick(library_check_seconds()).await.unwrap();

    let library = newest(&world, SSH).await;
    let unsec_value = library.zones[0].weaknesses[0].unsec_value.clone();

    assert_eq!(library.version, "9.0.1");
    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .report_weakness("libssh.so", &library.zones[0].address, &unsec_value)
            .await
            .unwrap(),
        Ok(())
    );
}

#[tokio::test]
async fn should_refuse_reports_the_game_refuses_before_counting_them() {
    let (world, _) = shop_world().await;

    for library in ["metaxploit.so", "crypto.so", "librshell.so"] {
        assert_eq!(
            world
                .modules
                .libraries
                .versions
                .report_weakness(library, "0x10", "buffer")
                .await
                .unwrap(),
            Err(WeaknessReportFailure::Unreportable)
        );
    }

    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .report_weakness("libnothing.so", "0x10", "buffer")
            .await
            .unwrap(),
        Err(WeaknessReportFailure::UnknownLibrary(
            "libnothing.so".to_owned()
        ))
    );
    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .report_weakness("libssh.so", "0x99", "buffer")
            .await
            .unwrap(),
        Err(WeaknessReportFailure::UnknownZone)
    );
    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .report_weakness("libssh.so", "0x10", "missing")
            .await
            .unwrap(),
        Err(WeaknessReportFailure::UnknownValue {
            value: "missing".to_owned(),
            zone: "0x10".to_owned(),
        })
    );
    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .report_weakness("libssh.so", "0x10", "buffer")
            .await
            .unwrap(),
        Ok(()),
        "the refused reports left the player's report free"
    );
}

#[tokio::test]
async fn should_replace_a_zone_whose_weaknesses_were_patched_three_times() {
    let (world, _) = shop_world().await;

    for _ in 0..3 {
        assert!(
            world
                .modules
                .libraries
                .versions
                .release_library_version(SSH)
                .await
                .unwrap()
                .is_some()
        );
    }

    let library = newest(&world, SSH).await;

    assert_eq!(library.version, "9.0.3");
    assert!(library.zones.iter().all(|zone| zone.address != "0x10"));
    assert!(library.zones.iter().any(|zone| !zone.hide));
}

#[tokio::test]
async fn should_keep_the_saved_versions_and_forget_the_oldest_beyond_a_hundred() {
    let (world, _) = shop_world().await;

    assert!(
        world
            .modules
            .libraries
            .versions
            .saved_library_versions(SSH)
            .await
            .unwrap()
            .is_empty()
    );

    world
        .modules
        .libraries
        .versions
        .release_library_version(SSH)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .libraries
            .versions
            .saved_library_versions(SSH)
            .await
            .unwrap(),
        [VERSION, "9.0.1"]
    );

    for _ in 1..100 {
        world
            .modules
            .libraries
            .versions
            .release_library_version(SSH)
            .await
            .unwrap();
    }

    let saved = world
        .modules
        .libraries
        .versions
        .saved_library_versions(SSH)
        .await
        .unwrap();

    assert_eq!(saved.len(), 100);
    assert_eq!(saved.first().map(String::as_str), Some("9.0.1"));
    assert_eq!(saved.last().map(String::as_str), Some("10.0.0"));
}
