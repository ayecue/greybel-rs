use greyhack_behaviour::{GUEST_USER, MAX_DEBUG_LOGIN_ATTEMPTS, ROOT_USER};
use greyhack_world::{DebugFailure, DebugLibraryPatch, DebugPayload, PartialObject};

use crate::support::debug::{LIBRARY_PATH, debug_world};

#[tokio::test]
async fn should_block_debug_login_after_the_attempt_limit() {
    let (mut world, _) = debug_world().await;

    for attempt in 1..=MAX_DEBUG_LOGIN_ATTEMPTS {
        assert_eq!(
            world.login_debug("dbg", "wrong").await.unwrap(),
            Err(DebugFailure::IncorrectPassword)
        );
        assert_eq!(
            world
                .debug_account("dbg")
                .await
                .unwrap()
                .unwrap()
                .failed_logins,
            attempt
        );
    }

    assert_eq!(
        world.login_debug("dbg", "secret").await.unwrap(),
        Err(DebugFailure::LoginLockout)
    );
    assert_eq!(
        world
            .debug_account("dbg")
            .await
            .unwrap()
            .unwrap()
            .failed_logins,
        MAX_DEBUG_LOGIN_ATTEMPTS
    );

    let mut account = world.debug_account("dbg").await.unwrap().unwrap();

    account.failed_logins = 1;
    world.save_debug_account(&account).await.unwrap();
    assert_eq!(world.login_debug("dbg", "secret").await.unwrap(), Ok(()));
    assert_eq!(
        world
            .debug_account("dbg")
            .await
            .unwrap()
            .unwrap()
            .failed_logins,
        0
    );
}

#[tokio::test]
async fn should_scan_and_patch_a_debug_library() {
    let (mut world, computer) = debug_world().await;

    let expected = world
        .debug_library(computer, LIBRARY_PATH)
        .await
        .unwrap()
        .unwrap()
        .challenge();

    assert_eq!(
        world
            .scan_debug_library(computer, LIBRARY_PATH)
            .await
            .unwrap(),
        Ok(expected)
    );
    assert_eq!(
        world
            .debug_library_patch(computer, LIBRARY_PATH, true)
            .await
            .unwrap(),
        DebugLibraryPatch::NoPatch
    );
    assert_eq!(
        world
            .patch_debug_library(computer, LIBRARY_PATH, "/root/patch.src")
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        world
            .debug_library_patch(computer, LIBRARY_PATH, false)
            .await
            .unwrap(),
        DebugLibraryPatch::PatchPresent
    );
    assert_eq!(
        world
            .scan_debug_library(computer, LIBRARY_PATH)
            .await
            .unwrap(),
        Err(DebugFailure::ScanPatchPresent)
    );
}

#[tokio::test]
async fn should_lock_debug_tests_after_an_incorrect_answer() {
    let (mut world, computer) = debug_world().await;

    let error_lines = world
        .debug_library(computer, LIBRARY_PATH)
        .await
        .unwrap()
        .unwrap()
        .challenge()
        .error_lines;
    let mut duplicated = error_lines.clone();

    duplicated.push(error_lines[0]);

    let found = world
        .run_debug_library_tests(computer, LIBRARY_PATH, "dbg", &duplicated)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        found,
        world
            .debug_library(computer, LIBRARY_PATH)
            .await
            .unwrap()
            .unwrap()
            .zero_day()
    );

    assert_eq!(
        world
            .run_debug_library_tests(computer, LIBRARY_PATH, "dbg", &[])
            .await
            .unwrap(),
        Err(DebugFailure::IncorrectErrorLines)
    );
    assert!(
        world
            .debug_account("dbg")
            .await
            .unwrap()
            .unwrap()
            .unit_test_failed
    );
    assert_eq!(
        world
            .run_debug_library_tests(computer, LIBRARY_PATH, "dbg", &[1])
            .await
            .unwrap(),
        Err(DebugFailure::UnitTestLockout)
    );
}

#[tokio::test]
async fn should_return_partial_objects_and_reset_only_the_automatic_patch_schedule() {
    let (mut world, computer) = debug_world().await;
    let library = world
        .debug_library(computer, LIBRARY_PATH)
        .await
        .unwrap()
        .unwrap();
    let zero_day = library.zero_day();
    let mut patch_state = library.clone();

    patch_state.patch_seed = Some(library.seed - 1);
    patch_state.auto_patch_at = Some(world.time_seconds());
    patch_state.num_patches = 1;
    world.save_debug_library(&patch_state).await.unwrap();

    let node = world
        .file(computer, LIBRARY_PATH, false)
        .await
        .unwrap()
        .unwrap();
    let normal_user = world
        .normal_user(computer)
        .await
        .unwrap()
        .unwrap_or_else(|| ROOT_USER.to_owned());
    let expected_user = |choice: &str| {
        if choice == ROOT_USER || choice == GUEST_USER {
            choice.to_owned()
        } else {
            normal_user.clone()
        }
    };

    let payload = world
        .debug_library_payload(computer, LIBRARY_PATH, &zero_day.address, "")
        .await
        .unwrap();
    let DebugPayload::Objects(objects) = payload else {
        panic!("the payload returns objects");
    };

    assert_eq!(objects.len(), 3);
    assert_eq!(
        objects[0],
        PartialObject::Computer {
            computer,
            user: expected_user(&zero_day.objects[0].access),
            methods: zero_day.objects[0].methods.clone(),
        }
    );
    assert_eq!(
        objects[1],
        PartialObject::File {
            computer,
            node: node.id,
            path: LIBRARY_PATH.to_owned(),
            user: expected_user(&zero_day.objects[1].access),
            methods: zero_day.objects[1].methods.clone(),
        }
    );
    assert!(matches!(objects[2], PartialObject::Library { .. }));

    let library = world
        .debug_library(computer, LIBRARY_PATH)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(library.patch_seed, Some(library.seed - 1));
    assert_eq!(library.auto_patch_at, None);
    assert_eq!(library.num_patches, 0);
}
