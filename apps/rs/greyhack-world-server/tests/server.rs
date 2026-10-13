//! Verifies that separate runtime connections share writes through the game-server process.

use duplex_test_support::local::run_local;

use crate::support::{GameServerProcess, run};

#[test]
fn should_share_the_world_between_the_programs_of_separate_runtimes() {
    let process = GameServerProcess::start();

    run_local(async {
        let writer = process.connect().await;
        let reader = process.connect().await;

        let written = run(
            &writer,
            "computer = get_shell.host_computer\nprint computer.touch(home_dir, \"shared.txt\")\nprint computer.File(home_dir + \"/shared.txt\").set_content(\"from the writer\")",
        )
        .await;
        let read = run(
            &reader,
            "print get_shell.host_computer.File(home_dir + \"/shared.txt\").get_content",
        )
        .await;

        assert_eq!(written, "1\n1");
        assert_eq!(read, "from the writer");
    });
}
