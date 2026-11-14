use std::panic::{AssertUnwindSafe, catch_unwind};

use duplex_test_support::local::run_local;
use greybel_test_support::future::block_on;
use greyhack_behaviour::SYSTEM_LOG_PATH;

use crate::support::program::{
    AdminAddComputer, AdminShell, ClientEnd, ComputerCreation, ComputerOptions, EndProgram,
    LaunchOutcome, MiningHardware, ShellStartTerminal,
};
use crate::support::{connect, process, server, settle};

/// Exiting a terminal removes its shell from the launch tree, allowing `CloseRemotePrograms` to log the
/// remote close while the launcher continues running (`HelperScript.CloseRemotePrograms`).
#[test]
fn should_pop_the_last_shell_when_a_terminal_exits() {
    run_local(async {
        // Arrange
        let server = server();
        let (public_ip, remote_ip) = {
            let world = server.world().await;
            let player = world.handle.player();
            let public_ip = world
                .modules
                .core
                .networks
                .topology
                .public_ip(player)
                .await
                .expect("the player has an address");
            let local_ip = world
                .modules
                .core
                .networks
                .topology
                .computer(player)
                .await
                .expect("the player computer reads")
                .expect("the player computer exists")
                .local_ip;
            let (network, _) = local_ip
                .rsplit_once('.')
                .expect("the LAN address has octets");

            (public_ip, format!("{network}.99"))
        };
        let runtime = connect(&server);
        let launcher = runtime.start("/bin/tool").await;
        let created = runtime
            .channel
            .send(&AdminAddComputer {
                program: launcher.program,
                public_ip: public_ip.clone().into(),
                computer: ComputerOptions {
                    name: Some("remote".into()),
                    lan_ip: Some(remote_ip.clone()),
                    root_password: Some("root".into()),
                    users: None,
                    services: None,
                    is_switch: false,
                    mining_hardware: MiningHardware {
                        cpu_speed: 1.0,
                        cpu_cores: 1,
                        ram_model: 0,
                        gpu_hashrate: 1.0,
                    },
                },
            })
            .expect("the computer is sent")
            .await
            .expect("the computer is created");
        let ComputerCreation::Computer(_computer) = created else {
            panic!("the remote computer is created: {created:?}");
        };
        let shell = runtime
            .channel
            .send(&AdminShell {
                program: launcher.program,
                public_ip: public_ip.clone().into(),
                lan_ip: remote_ip.clone().into(),
                user: "root".into(),
            })
            .expect("the shell is sent")
            .await
            .expect("the shell responds")
            .expect("the remote shell exists");
        let LaunchOutcome::Program {
            session: terminal, ..
        } = runtime
            .channel
            .send(&ShellStartTerminal {
                program: launcher.program,
                shell,
            })
            .expect("the terminal is sent")
            .await
            .expect("the terminal starts")
        else {
            panic!("the remote terminal starts");
        };

        // Act
        runtime
            .channel
            .send(&EndProgram {
                program: terminal.program,
                reason: ClientEnd::Exited,
            })
            .expect("the terminal exit is sent")
            .await
            .expect("the terminal exit responds");
        settle().await;

        let content = {
            let world = server.world().await;
            let computer = world
                .modules
                .core
                .networks
                .routers
                .computer_at(&public_ip, &remote_ip)
                .await
                .expect("the remote computer reads")
                .expect("the remote computer exists");

            world
                .modules
                .core
                .files
                .lookup
                .content(computer, SYSTEM_LOG_PATH, false)
                .await
                .expect("the remote system log reads")
                .expect("the remote system log exists")
        };

        // Assert
        assert!(
            content.contains("\"action\":1"),
            "the terminal close is logged: {content}"
        );
        assert!(
            process(&server, "tool").is_some(),
            "the launcher still runs"
        );
    });
}

#[test]
fn read_routes_refuse_mutations_and_roll_back_their_transaction() {
    let server = server();
    let before = block_on(server.world()).handle.time_seconds();

    let panic = catch_unwind(AssertUnwindSafe(|| {
        block_on(server.read_world(async |world| {
            world.handle.world_mut().time_seconds += 1;
        }));
    }));

    assert!(panic.is_err(), "world_mut panics during a read route");
    assert_eq!(block_on(server.world()).handle.time_seconds(), before);
}
