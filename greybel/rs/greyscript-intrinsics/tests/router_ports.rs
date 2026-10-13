use greybel_test_support::future::block_on;
use greyhack_behaviour::ServiceId;
use greyhack_world::World;

use crate::support::network::{SERVICE_SEED, first_remote_service};
use crate::support::scripts::run_on_host;
use crate::support::world_options;

/// `Router.GetPortVersion` reads the service library of the computer a port forwards to. The port stays in the
/// router's table without it, so `ping_port` still finds it and `port_info` answers `unknown unknown`.
#[test]
fn should_report_a_forwarded_port_whose_service_library_was_renamed_as_unknown() {
    let world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = first_remote_service(&world);
    let code = format!(
        "router = get_router(\"{}\")\nprint router.port_info(router.ping_port({}))",
        target.address, target.port
    );
    let (before, error, mut world) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert!(
        before.starts_with(&format!("{} ", target.service)),
        "{before}"
    );

    let service = ServiceId::from_name(&target.service).expect("a service");
    let library = block_on(world.file(
        target.computer,
        &format!("/lib/{}", service.library()),
        false,
    ))
    .unwrap()
    .unwrap();
    let folder = block_on(world.folder(target.computer, "/lib", false))
        .unwrap()
        .unwrap();

    block_on(world.move_node(&library, &folder, "renamed.so")).unwrap();

    let (after, error, _) = run_on_host(&code, world, true);

    assert_eq!(error, None);
    assert_eq!(after.trim_end(), "unknown unknown");
}
