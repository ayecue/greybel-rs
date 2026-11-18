//! Tests the world server that the runtimes of one machine share on a port.

use std::time::Duration;

use duplex_test_support::local::run_local;
use greybel_runtime::{SharedWorld, SharedWorldFailure};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

use crate::support::{IDLE, run, shared_world, worlds_folder};

#[test]
fn should_start_the_shared_server_once_and_share_its_world() {
    let world = shared_world(
        IDLE,
        &worlds_folder("should_start_the_shared_server_once_and_share_its_world"),
    );

    run_local(async {
        let (writer, reader) = tokio::join!(world.connect("test"), world.connect("test"));
        let (writer, reader) = (
            writer.expect("the server starts"),
            reader.expect("the server is found"),
        );

        run(
            &writer,
            "computer = get_shell.host_computer\ncomputer.touch(home_dir, \"shared.txt\")\ncomputer.File(home_dir + \"/shared.txt\").set_content(\"from the writer\")",
        )
        .await;

        let read = run(
            &reader,
            "print get_shell.host_computer.File(home_dir + \"/shared.txt\").get_content",
        )
        .await;

        assert_eq!(read, "from the writer");
    });
}

#[test]
fn should_refuse_a_shared_server_of_another_seed() {
    let world = shared_world(
        IDLE,
        &worlds_folder("should_refuse_a_shared_server_of_another_seed"),
    );

    run_local(async {
        let _connection = world.connect("test").await.expect("the server starts");
        let other = world.connect("other").await.err();

        assert!(
            matches!(&other, Some(SharedWorldFailure::Seed { seed, .. }) if seed == "test"),
            "{other:?}"
        );
    });
}

#[test]
fn should_shut_down_only_after_the_idle_timeout_without_connections() {
    let world = shared_world(
        IDLE,
        &worlds_folder("should_shut_down_only_after_the_idle_timeout_without_connections"),
    );

    run_local(async {
        let connection = world.connect("test").await.expect("the server starts");

        sleep(IDLE * 2).await;
        assert_eq!(
            run(&connection, "print 1").await,
            "1",
            "a connected server keeps running"
        );

        connection.close();
        sleep(IDLE * 2).await;
        assert!(
            TcpStream::connect(world.address()).await.is_err(),
            "the server shuts down when idle"
        );
    });
}

#[test]
fn should_interrupt_its_connections_and_generate_the_world_again_on_reset() {
    let world = shared_world(
        IDLE,
        &worlds_folder("should_interrupt_its_connections_and_generate_the_world_again_on_reset"),
    );

    run_local(async {
        let port = world
            .address()
            .rsplit(':')
            .next()
            .and_then(|port| port.parse().ok())
            .expect("a port");
        let before = world.connect("test").await.expect("the server starts");

        run(
            &before,
            "get_shell.host_computer.touch(home_dir, \"old.txt\")",
        )
        .await;

        assert_eq!(
            SharedWorld::reset(port).await.expect("the world resets"),
            "test"
        );
        assert!(
            timeout(Duration::from_secs(5), before.on_disconnect())
                .await
                .is_ok(),
            "the reset closes earlier connections"
        );

        let after = world.connect("test").await.expect("the server runs");

        assert_eq!(
            run(
                &after,
                "print get_shell.host_computer.File(home_dir + \"/old.txt\") == null"
            )
            .await,
            "1"
        );
    });
}

#[test]
fn should_report_that_no_server_runs_on_reset() {
    let world = shared_world(
        IDLE,
        &worlds_folder("should_report_that_no_server_runs_on_reset"),
    );

    run_local(async {
        let port = world
            .address()
            .rsplit(':')
            .next()
            .and_then(|port| port.parse().ok())
            .expect("a port");

        assert!(matches!(
            SharedWorld::reset(port).await,
            Err(SharedWorldFailure::Absent { .. })
        ));
    });
}
