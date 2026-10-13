use greybel_test_support::future::block_on;
use greyhack_world::{PlayerOptions, World};

use super::scripts::run_on_host;

/// Runs `parent` after building `child` in the player's home folder.
pub fn run_with_child(child: &str, parent: &str) -> (String, Option<String>) {
    let code = format!(
        "sh = get_shell\npc = sh.host_computer\npc.touch(home_dir, \"child.src\")\npc.File(home_dir + \"/child.src\").set_content(\"{}\")\nget_shell.build(home_dir + \"/child.src\", home_dir)\n{parent}",
        child.replace('"', "\"\"")
    );
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = run_on_host(&code, world, true);

    (output, error)
}
