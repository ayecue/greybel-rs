//! Tests deployment and imports into headless Grey Hack through the builder protocol.

use crate::support::editor::{AutoCompile, Build, Import};
use crate::support::game::{in_game, run_in_game};
use crate::support::requests::build;
use crate::support::{Harness, LIBRARY, MAIN};

#[test]
fn should_deploy_a_build_into_the_game_and_run_it_there() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let folder = format!("/root/builder{}", std::process::id());
        let main = "import_code(\"/lib.src\")\nprint(double(21))";
        let library = "double = function(n)\n  return n * 2\nend function";
        let mut harness = Harness::new(&[(MAIN, main), (LIBRARY, library)]);
        let deployment = Build {
            root: Some("file:///project".into()),
            ingame_directory: folder.clone().into(),
            auto_compile: Some(AutoCompile {
                allow_import: false,
            }),
            ..build(&[MAIN], None, Some(port))
        };

        let built = harness.request_on_io(&deployment).await;
        let check = format!(
            "computer = get_shell.host_computer\nprint(typeof(computer.File(\"{folder}/main\")))\nprint(typeof(computer.File(\"{folder}/main.src\")))"
        );
        let (completed, printed) = run_in_game(port, &check).await;

        assert!(built.is_ok(), "{built:?}");
        assert!(completed);
        assert_eq!(printed.join("\n"), "file\nnull");
    });
}

#[test]
fn should_import_files_into_the_game_as_they_are() {
    in_game(env!("CARGO_TARGET_TMPDIR"), |port| async move {
        let folder = format!("/root/import{}", std::process::id());
        let mut harness = Harness::new(&[(MAIN, "print(1)"), (LIBRARY, "x = 2")]);
        let import = Import {
            root: "file:///project".into(),
            files: vec![MAIN.to_owned(), LIBRARY.to_owned()].into(),
            ingame_directory: folder.clone().into(),
            port,
        };

        let imported = harness.request_on_io(&import).await;
        let check =
            format!("print(get_shell.host_computer.File(\"{folder}/lib.src\").get_content)");
        let (completed, printed) = run_in_game(port, &check).await;

        assert_eq!(imported, Ok(2));
        assert!(completed);
        assert_eq!(printed, ["x = 2"]);
    });
}
