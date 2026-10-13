//! The game server applies the rules a program checks before it sends an operation, so a program that skips them
//! gets the same answer.

use duplex_test_support::local::run_local;

use crate::support::program::{
    ComputerCreateFolder, ComputerTouch, Failure, FileSetContent, FileSetOwner, NameKind, Outcome,
    Refusal, ScriptFailure,
};
use crate::support::server::notes;
use crate::support::{connect, player, server};

#[test]
fn should_refuse_a_new_name_the_program_did_not_check() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;
        let home = format!("/home/{}", player());

        // Act
        let touched = runtime
            .channel
            .send(&ComputerTouch {
                program: session.program,
                computer: session.shell,
                folder: home.as_str().into(),
                name: "bad name".into(),
            })
            .unwrap()
            .await;
        let created = runtime
            .channel
            .send(&ComputerCreateFolder {
                program: session.program,
                computer: session.shell,
                folder: home.as_str().into(),
                name: "a".repeat(129).into(),
            })
            .unwrap()
            .await;

        // Assert
        assert_eq!(
            (touched, created),
            (
                Ok(Outcome::Failure(Failure::Refusal(Refusal::InvalidName(
                    NameKind::File
                )))),
                Ok(Outcome::Failure(Failure::Refusal(Refusal::LongName))),
            )
        );
    });
}

#[test]
fn should_refuse_content_over_the_limit_the_program_did_not_check() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;
        let file = notes(&runtime, &session).await;

        // Act
        let written = runtime
            .channel
            .send(&FileSetContent {
                program: session.program,
                file,
                content: "a".repeat(160_001).into(),
            })
            .unwrap()
            .await;

        // Assert
        assert_eq!(
            written,
            Ok(Outcome::Failure(Failure::Refusal(Refusal::ContentTooLong)))
        );
    });
}

#[test]
fn should_fail_an_operation_whose_refusal_the_program_raises() {
    run_local(async {
        // Arrange
        let server = server();
        let runtime = connect(&server);
        let session = runtime.start("/bin/tool").await;
        let file = notes(&runtime, &session).await;

        // Act
        let changed = runtime
            .channel
            .send(&FileSetOwner {
                program: session.program,
                file,
                owner: "not valid".into(),
                recursive: false,
            })
            .unwrap()
            .await;

        // Assert
        assert!(matches!(
            changed,
            Err(duplex_core::Error::Failure(ScriptFailure::InvalidArguments))
        ));
    });
}
