//! Programs for `launch` in the safe-mode terminal, and runs of a script in a world.

use greybel_test_support::future::block_on;
use greyhack_world::{PlayerOptions, World};

use super::scripts::{run_in_safe_terminal, run_on_host};

/// Gives `other` a program in its home that others can't execute, and one in a folder others can't enter.
pub const SETUP: &str = r#"
root = get_shell("root", "test")
pc = root.host_computer
print pc.create_user("other", "pw")
print pc.create_folder("/home/other", "locked")
pc.touch(home_dir, "child.src")
pc.File(home_dir + "/child.src").set_content("print(""ran"")")
print "build=" + root.build(home_dir + "/child.src", "/home/other")
print "build=" + root.build(home_dir + "/child.src", "/home/other/locked")
print pc.File("/home/other/child").chmod("o-x", 0)
print pc.File("/home/other/locked").chmod("o-x", 0)
print pc.File("/home/other/locked/child").chmod("o-x", 0)
"#;

pub const LAUNCH: &str = "sh = get_shell(\"other\", \"pw\")\nprint sh.launch(\"/home/other/child\")\nprint sh.launch(\"/home/other/locked/child\")";

/// Builds a program that launches the one `SETUP` made, as `other`, in `other`'s home.
pub const SETUP_PARENT: &str = r#"
root = get_shell("root", "test")
pc = root.host_computer
pc.touch(home_dir, "parent.src")
pc.File(home_dir + "/parent.src").set_content("sh = get_shell(""other"", ""pw"")" + char(10) + "print sh.launch(""/home/other/child"")")
print "build=" + root.build(home_dir + "/parent.src", "/home/other")
"#;

pub const SETUP_PARENT_OUTPUT: &str = "build=";

pub const LAUNCH_PARENT: &str =
    "sh = get_shell(\"other\", \"pw\")\nprint sh.launch(\"/home/other/parent\")";

pub const SETUP_OUTPUT: &str = "1\n1\nbuild=\nbuild=\n\n\n";

pub fn world() -> World {
    block_on(World::new(&PlayerOptions::default())).expect("a world")
}

pub fn run(code: &str, world: World) -> (String, World) {
    let (output, error, world) = run_on_host(code, world, true);

    assert_eq!(error, None, "{output}");

    (output, world)
}

/// Like [`run`], with the program typed into the safe-mode terminal the world runs.
pub fn run_safe(code: &str, world: World) -> (String, World) {
    let (output, error, world) = run_in_safe_terminal(code, world, true);

    assert_eq!(error, None, "{output}");

    (output, world)
}
