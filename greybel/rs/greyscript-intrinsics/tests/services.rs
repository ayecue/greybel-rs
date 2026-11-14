use greybel_test_support::future::block_on;
use greyhack_world::{LoginFailure, ServiceFailure, World};

use crate::support::network::{
    SERVICE_SEED, connect_to, first_remote_service, literal, rebooted_service,
};
use crate::support::scripts::{run_code_in, run_on_host};
use crate::support::{read_computer, world_options};

#[test]
fn should_install_a_service_through_its_native_object() {
    let code = r#"
include_lib("admin.so").sudo
service = include_lib("/lib/libssh.so")
print service.install_service
print get_shell.host_computer.File("/server/conf/sshd.conf") != null
print get_shell.host_computer.get_ports[0].port_number
"#;

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "1\n1\n22");
}

#[test]
fn should_refuse_connections_while_the_computer_reboots() {
    // Arrange
    let (mut world, target) = rebooted_service();

    // Act
    let connection = connect_to(&mut world, &target);

    // Assert
    assert_eq!(
        connection,
        Err(LoginFailure::Service(ServiceFailure::HostDown))
    );
}

#[test]
fn should_accept_connections_once_the_computer_rebooted() {
    // Arrange
    let (mut world, target) = rebooted_service();

    block_on(world.tick(180)).unwrap();

    // Act
    let connection = connect_to(&mut world, &target);

    // Assert
    assert!(connection.is_ok());
}

#[test]
fn should_interrupt_the_objects_of_a_computer_that_rebooted() {
    // Arrange
    let world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = first_remote_service(&world);
    let device = block_on(world.modules.core.networks.topology.devices(target.router)).unwrap()[0];
    let computer = read_computer(&world, device);
    let code = format!(
        "admin = include_lib(\"admin.so\")\nadmin.sudo\ncomputer = admin.network({}).computer({}).shell.host_computer\nprint computer.reboot\nprint computer.get_name",
        literal(&target.address),
        literal(&computer.local_ip)
    );

    // Act
    let (output, error, _) = run_on_host(&code, world, true);

    // Assert
    assert_eq!(output, "1", "{error:?}");
    assert!(
        error
            .as_deref()
            .is_some_and(|error| error.contains("remote connection interrupted")),
        "{error:?}"
    );
}
