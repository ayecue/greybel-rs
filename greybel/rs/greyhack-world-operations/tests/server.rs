use std::rc::Rc;

use duplex_core::{Error, ErrorCode};
use duplex_test_support::local::run_local;
use futures_util::FutureExt;
use greyhack_world::prelude::*;
use greyhack_world::{Database, PlayerOptions, World};
use greyhack_world_operations::WorldServer;
use scheduler::ManualClock;

use crate::support::program::{
    ClientEnd, EconomyFailure, EndProgram, EndReason, Failure, FileContent, FileGetContent,
    FileSetContent, LaunchFailure, LaunchOutcome, MetaxploitSniffer, MineJobFailure, ObjectId,
    Outcome, PrintLine, ProgramEnd, ProgramPrint, ReleaseObjects, ScriptFailure, StartProgram,
    SubWalletMining,
};
use crate::support::server::{launch, metaxploit, notes, sub_wallets};
use crate::support::{
    Notice, connect, end_process, player, process, server, server_with_ether, settle,
};

/// Tests that program operations mutate the server world and are visible to a second runtime.
#[test]
fn should_share_the_world_between_two_runtimes() {
    run_local(async {
        // Arrange
        let server = server();
        let (first, second) = (connect(&server), connect(&server));
        let (writer, reader) = (
            first.start("/bin/writer").await,
            second.start("/bin/reader").await,
        );

        // Act
        let written = first
            .channel
            .send(&FileSetContent {
                program: writer.program,
                file: notes(&first, &writer).await,
                content: "hello".into(),
            })
            .expect("the change is sent")
            .await;
        let read = second
            .channel
            .send(&FileGetContent {
                program: reader.program,
                file: notes(&second, &reader).await,
            })
            .expect("the read is sent")
            .await;

        // Assert
        assert_eq!(written, Ok(Outcome::True));
        assert_eq!(read, Ok(FileContent::Content("hello".to_owned())));
    });
}

/// Tests connection-local object lookup and message dropping for unknown or wrong-class object IDs.
#[test]
fn should_drop_an_operation_on_an_object_the_connection_does_not_hold() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;

        // Act
        let unknown = runtime
            .channel
            .send(&FileGetContent {
                program: session.program,
                file: ObjectId { id: 999 },
            })
            .expect("the read is sent")
            .await;
        let shell_as_file = runtime
            .channel
            .send(&FileGetContent {
                program: session.program,
                file: session.shell,
            })
            .expect("the read is sent")
            .await;

        // Assert
        assert_eq!(unknown, Err(Error::Rejection(ErrorCode::DroppedMessage)));
        assert_eq!(
            shell_as_file,
            Err(Error::Rejection(ErrorCode::DroppedMessage))
        );
    });
}

/// Tests that released object IDs no longer resolve on the server.
#[test]
fn should_forget_the_objects_a_program_released() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;
        let file = notes(&runtime, &session).await;

        // Act
        runtime
            .channel
            .send(&ReleaseObjects {
                objects: vec![file].into(),
            })
            .expect("the release is sent");

        let read = runtime
            .channel
            .send(&FileGetContent {
                program: session.program,
                file,
            })
            .expect("the read is sent")
            .await;

        // Assert
        assert_eq!(read, Err(Error::Rejection(ErrorCode::DroppedMessage)));
    });
}

/// Tests that a started program remains a player-computer process until it ends.
#[test]
fn should_run_a_started_program_as_a_process() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let user = player();

        // Act
        let session = runtime.start("/bin/tool").await;

        // Assert
        assert_eq!(session.user, user);
        assert_eq!(session.home, format!("/home/{user}"));
        assert_eq!(
            process(&server, "tool").map(|process| process.user),
            Some(user.clone())
        );

        let written = runtime
            .channel
            .send(&EndProgram {
                program: session.program,
                reason: ClientEnd::Finished,
            })
            .expect("the end is sent")
            .await;

        assert_eq!(written, Ok(false));
        assert_eq!(process(&server, "tool"), None);
    });
}

/// Tests that ending a program's process tells it why it ended.
#[test]
fn should_end_a_program_whose_process_ended() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;

        // Act
        end_process(&server, "tool").await;
        settle().await;

        // Assert
        assert_eq!(
            runtime.ended(),
            [ProgramEnd {
                program: session.program,
                reason: EndReason::Closed,
            }]
        );
    });
}

/// Tests that disconnecting a runtime ends its programs.
#[test]
fn should_end_the_programs_of_a_runtime_that_disconnects() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);

        runtime.start("/bin/tool").await;

        // Act
        drop(runtime);
        settle().await;

        // Assert
        assert_eq!(process(&server, "tool"), None);
    });
}

/// A refused launch queues its typed output before responding without a program value.
#[test]
fn should_print_why_a_launch_is_refused_before_its_response() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;

        // Act
        let outcome = launch(&runtime, &session, "/bin/missing").await;
        let notices = runtime.notices.borrow().clone();

        // Assert
        assert_eq!(outcome, LaunchOutcome::Refusal);
        assert_eq!(
            notices,
            [Notice::Print(ProgramPrint {
                program: session.program,
                line: PrintLine::LaunchFailure(Failure::Launch(LaunchFailure::CommandNotFound(
                    "/bin/missing".to_owned()
                ))),
            })]
        );
    });
}

/// Tests that a program another program launched ends with it: its process too.
#[test]
fn should_end_the_programs_an_ended_program_launched() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let parent = runtime.start("/bin/tool").await;
        let LaunchOutcome::Program { session: child, .. } =
            launch(&runtime, &parent, "/bin/airmon").await
        else {
            panic!("the launch starts /bin/airmon");
        };

        // Act
        end_process(&server, "tool").await;
        settle().await;

        // Assert
        assert_eq!(
            runtime.ended(),
            [
                ProgramEnd {
                    program: parent.program,
                    reason: EndReason::Closed,
                },
                ProgramEnd {
                    program: child.program,
                    reason: EndReason::ParentEnd,
                },
            ]
        );
        assert!(process(&server, "airmon").is_none());
    });
}

/// Tests that a runtime ending a program also ends the programs it launched, telling the runtime.
#[test]
fn should_end_the_children_of_a_program_its_runtime_ended() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let parent = runtime.start("/bin/tool").await;
        let LaunchOutcome::Program { session: child, .. } =
            launch(&runtime, &parent, "/bin/airmon").await
        else {
            panic!("the launch starts /bin/airmon");
        };

        // Act
        runtime
            .channel
            .send(&EndProgram {
                program: parent.program,
                reason: ClientEnd::Stopped,
            })
            .expect("the end is sent")
            .await
            .expect("the game server responds");
        settle().await;

        // Assert
        assert_eq!(
            runtime.ended(),
            [ProgramEnd {
                program: child.program,
                reason: EndReason::ParentEnd,
            }]
        );
        assert!(process(&server, "airmon").is_none());
    });
}

/// Tests that an operation of a program that ended fails, and that its end may be sent again.
#[test]
fn should_fail_the_operations_of_an_ended_program() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;
        let file = notes(&runtime, &session).await;
        let end = EndProgram {
            program: session.program,
            reason: ClientEnd::Finished,
        };

        runtime
            .channel
            .send(&end)
            .expect("the end is sent")
            .await
            .expect("the game server responds");

        // Act
        let read = runtime
            .channel
            .send(&FileGetContent {
                program: session.program,
                file,
            })
            .expect("the read is sent")
            .await;
        let again = runtime.channel.send(&end).expect("the end is sent").await;

        // Assert
        assert_eq!(read, Err(Error::Failure(ScriptFailure::ProgramEnded)));
        assert_eq!(again, Ok(false));
    });
}

/// Tests that a program the game server ended acknowledges its end without a second notification.
#[test]
fn should_accept_the_acknowledgement_of_a_program_the_game_server_ended() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;

        end_process(&server, "tool").await;
        settle().await;

        // Act
        let acknowledged = runtime
            .channel
            .send(&EndProgram {
                program: session.program,
                reason: ClientEnd::Acknowledged,
            })
            .expect("the end is sent")
            .await;

        // Assert
        assert_eq!(acknowledged, Ok(false));
        assert_eq!(runtime.ended().len(), 1);
    });
}

/// Tests that acknowledging the end of a program the game server ended also acknowledges the programs it launched,
/// which the runtime ends without acknowledging each: the connection has no programs left, so the world is saved.
#[test]
fn should_acknowledge_the_ended_programs_an_acknowledged_program_launched() {
    run_local(async {
        // Arrange
        let database = Database::in_memory().await.expect("the database opens");
        let server = WorldServer::with_database(
            World::new(&PlayerOptions::default())
                .await
                .expect("the world generates"),
            database,
            Rc::new(ManualClock::constant()),
        )
        .await
        .expect("the world is written");
        let runtime = connect(&server);
        let parent = runtime.start("/bin/tool").await;
        let LaunchOutcome::Program { session: child, .. } =
            launch(&runtime, &parent, "/bin/airmon").await
        else {
            panic!("the launch starts /bin/airmon");
        };
        let file = notes(&runtime, &child).await;

        end_process(&server, "tool").await;
        settle().await;

        // Act
        let acknowledged = runtime
            .channel
            .send(&EndProgram {
                program: parent.program,
                reason: ClientEnd::Acknowledged,
            })
            .expect("the end is sent")
            .await;
        let read = runtime
            .channel
            .send(&FileGetContent {
                program: child.program,
                file,
            })
            .expect("the read is sent")
            .await;

        // Assert
        assert_eq!(acknowledged, Ok(true));
        assert_eq!(read, Err(Error::Failure(ScriptFailure::ProgramEnded)));
    });
}

/// Tests that a program whose process ends stops waiting on its work: its sniffer ends, failing the operation.
#[test]
fn should_stop_the_work_of_a_program_whose_process_ended() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;
        let metaxploit = metaxploit(&runtime, &session).await;
        let sniffer = runtime
            .channel
            .send(&MetaxploitSniffer {
                program: session.program,
                metaxploit,
                encode_source: false,
                program_path: "/bin/tool".into(),
            })
            .expect("the sniffer is sent");

        settle().await;

        // Act
        end_process(&server, "tool").await;

        // Assert
        assert_eq!(
            sniffer.await,
            Err(Error::Failure(ScriptFailure::ProgramEnded))
        );
    });
}

/// A program tree mines once at a time (`HelperScript`'s one mining process per terminal): a program launched by a
/// mining program is refused, while a program of another tree mines.
#[test]
fn should_refuse_a_second_mining_process_of_one_program_tree() {
    run_local(async {
        // Arrange
        let server = server_with_ether();
        let runtime = connect(&server);
        let miner = runtime.start("/bin/miner").await;
        let [alice, bob, carol] = sub_wallets(&runtime, &miner, &["alice", "bob", "carol"])
            .await
            .try_into()
            .expect("three sub-wallets");
        let LaunchOutcome::Program { session: child, .. } =
            launch(&runtime, &miner, "/bin/ls").await
        else {
            panic!("ls launches");
        };
        let other = runtime.start("/bin/other").await;
        let mut mining = runtime
            .channel
            .send(&SubWalletMining {
                program: miner.program,
                sub_wallet: alice,
            })
            .expect("the mining is sent");

        settle().await;

        // Act
        let refusal = runtime
            .channel
            .send(&SubWalletMining {
                program: child.program,
                sub_wallet: bob,
            })
            .expect("the mining is sent");
        let mut other_mining = runtime
            .channel
            .send(&SubWalletMining {
                program: other.program,
                sub_wallet: carol,
            })
            .expect("the mining is sent");

        settle().await;

        // Assert
        assert_eq!(
            refusal.now_or_never(),
            Some(Ok(Outcome::Failure(Failure::Economy(
                EconomyFailure::MineJob(MineJobFailure::SecondMineJob)
            ))))
        );
        assert!(
            (&mut other_mining).now_or_never().is_none(),
            "another tree's mining waits for its round"
        );
        assert!(
            (&mut mining).now_or_never().is_none(),
            "the first mining waits for its round"
        );
    });
}

/// Tests that the third strike deletes the player's computer, which ends the player's programs, and that no program
/// starts without one afterwards (`Database.GameOver`).
#[test]
fn should_end_the_programs_and_refuse_new_ones_when_the_game_is_over() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;

        // Act
        server
            .change_world(async |world| {
                let computer = world.handle.player();

                world
                    .modules
                    .guilds
                    .traces
                    .set_strikes(2)
                    .await
                    .expect("the world writes");
                world
                    .modules
                    .guilds
                    .traces
                    .strike_player(computer, greyhack_world::TraceKind::Active)
                    .await
                    .expect("the world writes");
            })
            .await;
        settle().await;

        let refused = runtime
            .channel
            .send(&StartProgram {
                user: player().into(),
                path: Some("/bin/again".into()),
                safe_terminal: false,
            })
            .expect("the program start is sent")
            .await;

        // Assert
        assert_eq!(
            runtime.ended(),
            [ProgramEnd {
                program: session.program,
                reason: EndReason::ComputerRemoval,
            }]
        );
        assert!(refused.is_err());
        assert!(!server.world().await.has_player());
    });
}
