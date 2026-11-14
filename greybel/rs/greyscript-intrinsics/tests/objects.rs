use duplex_test_support::local::run_local;
use greybel_interpreter::RunResult;
use greybel_test_support::greyhack::connect;
use greybel_test_support::interpreter::{constant_clock, inline, snapshot_options};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::GreyHackInterpreter;

/// The game server keeps the objects a program holds; once the program is gone, its runtime releases them with the
/// connection's next operation.
#[test]
fn should_release_the_objects_of_a_finished_program() {
    run_local(async {
        let server = WorldServer::new(
            World::new(&PlayerOptions::default()).await.unwrap(),
            constant_clock(),
        )
        .await
        .unwrap();
        let connection = connect(&server);
        let user = PlayerOptions::default().user;
        let (lines, options) = snapshot_options();
        let mut program = GreyHackInterpreter::start(
            &connection,
            &user,
            options,
            inline("files = get_shell.host_computer.File(\"/bin\").get_files\nprint files.len > 0"),
        )
        .await
        .expect("the program starts");
        let run = program.run().await;

        assert!(matches!(run, Ok(RunResult::Complete)), "{run:?}");
        assert_eq!(lines.text(), "1");
        assert!(
            server.object_count().await > 2,
            "{}",
            server.object_count().await
        );

        drop(program);

        let (_, options) = snapshot_options();
        let _next = GreyHackInterpreter::start(&connection, &user, options, inline(""))
            .await
            .expect("the next program starts");

        assert_eq!(server.object_count().await, 1);
    });
}
