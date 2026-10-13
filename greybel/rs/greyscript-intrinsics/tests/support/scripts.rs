use std::rc::Rc;

use duplex_test_support::local::run_local;
use greybel_interpreter::{Clock, Interpreter, InterpreterOptions, Stdin};
use greybel_test_support::greyhack;
use greybel_test_support::interpreter::{
    constant_clock, constant_clock_options, inline, run_to_end, runtime_error_of, snapshot_options,
};
use greyhack_world::{Database, PlayerOptions, World, WorldOptions};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::API;
use mono_random::Random;

pub fn run_code(code: &str) -> (String, Option<String>) {
    run_code_in(code, false)
}

/// Runs `code`, with the game intrinsics and the player's world when `game`.
pub fn run_code_in(code: &str, game: bool) -> (String, Option<String>) {
    if game {
        let world = greybel_test_support::future::block_on(World::new(&PlayerOptions::default()))
            .expect("a world");
        let (output, error, _) = run_on_host(code, world, true);

        return (output, error);
    }

    let (lines, options) = snapshot_options();
    let mut interpreter = Interpreter::new(options, inline(code))
        .with_host_data(Random::new(0))
        .with_api(API);
    let error = runtime_error_of(run_to_end(&mut interpreter));

    (lines.text(), error.map(|error| error.to_string()))
}

/// Runs GreyScript in a real host as the default player and returns its output, error, and resulting world.
pub fn run_on_host(code: &str, world: World, admin: bool) -> (String, Option<String>, World) {
    let user = PlayerOptions::default().user;
    let run = greyhack::run_on_host(code, world, &user, admin, constant_clock_options());

    (run.output, run.error, run.world)
}

/// Runs GreyScript with a controlled input source and clock.
pub fn run_game_with_io(
    code: &str,
    world: World,
    admin: bool,
    stdin: Rc<dyn Stdin>,
    clock: Rc<dyn Clock>,
) -> (String, Option<String>, World) {
    let options = InterpreterOptions {
        stdin,
        clock,
        ..InterpreterOptions::new(constant_clock())
    };
    let user = PlayerOptions::default().user;
    let run = greyhack::run_on_host(code, world, &user, admin, options);

    (run.output, run.error, run.world)
}

/// What `code` prints, followed by the runtime error that ends it, if any.
pub fn printed(code: &str, game: bool) -> String {
    let (mut output, error) = run_code_in(code, game);

    if let Some(error) = error {
        output.push_str(&format!("\nerror: {error}"));
    }

    output
}

/// Each program and its error: `raises` and the message, or `returns` and what printing it shows, for a program that is
/// an expression whose value is the error.
pub fn reported_errors(programs: &[&str], game: bool) -> String {
    let mut snapshot = String::new();

    for code in programs {
        let report = match run_code_in(code, game) {
            (_, Some(error)) => format!("raises {error}"),
            _ => match run_code_in(&format!("print({code})"), game) {
                (output, None) => format!("returns {}", output.trim_end()),
                (_, Some(error)) => panic!("`{code}` doesn't fail, and printing it raises {error}"),
            },
        };

        snapshot.push_str(&format!("{code}\n  {report}\n"));
    }

    snapshot
}

/// Each line is a program ending in an error, raised or returned; the snapshot pairs them.
pub const ERRORS: &[&str] = &[
    "char(65536)",
    "char(55296)",
    "code(\"\")",
    "\"😀\".code",
    "[1].join(\"x\" * 129)",
    "l = []; l.push(l)",
    "{}.push(null)",
    "range(null, 2)",
    "range(1, null)",
    "replace(\"abc\", \"a\", \"b\")",
    "wait(0)",
    "wait(301)",
    "md5(1)",
    "\"a\".split(\"a\", \"q\")",
    "\"a\".replace(\"\", \"b\")",
    "\"a\".replace(\"a\", 1)",
    "\"a\".replace(\"a\", \"b\", \"q\")",
    "\"a\".is_match(1)",
    "\"a\".is_match(\"\")",
    "\"a\".is_match(\"a\", \"ix\")",
    "(\"a\" * 40 + \"b\").is_match(\"(a|aa)+(?=b)c\")",
    "\"a\".matches(\"\")",
    "\"a\".matches(1)",
    "\"a\".matches(\"a\", \"z\")",
    "\"a\".matches(\"(\")",
    "[1].remove(5)",
    "refEquals(1, 1)",
    "stackTrace",
];

/// Errors the game intrinsics raise, and assignments into game objects.
pub const GAME_ERRORS: &[&str] = &[
    "command_info(\"\")",
    "command_info(1)",
    "get_shell(1)",
    "get_shell.x = 1",
    "get_shell.__isa.x = 1",
    "s = get_shell; s[\"x\"] = 1",
    "get_shell.pop",
    "get_shell.push(1)",
    "get_shell.remove(\"classID\")",
    "get_shell.shuffle",
    "get_shell.host_computer.File(\"\")",
    "get_shell.host_computer.create_folder(1)",
    "get_shell.host_computer.create_user(\"\", \"x\")",
    "get_shell.host_computer.create_user(\"a\" * 16, \"x\")",
    "get_shell.host_computer.groups(\"\")",
    "get_shell.host_computer.File(\"/home\").chmod(\"u+r\", \"x\")",
    "get_shell.host_computer.File(\"/home\").set_owner(\"bad name\")",
    "get_shell.host_computer.File(\"/home\").set_group(\"a\" * 16)",
    "get_shell.host_computer.File(\"/home\").copy(1)",
    "get_shell.host_computer.File(\"/home\").copy(\"/tmp\", \"a\" * 129)",
    "get_abs_path(1)",
    "get_abs_path(\"a\" * 1025)",
    "cd(1)",
    "parent_path(\"\")",
    "clear_screen; clear_screen",
    "include_lib(\"\")",
    "include_lib(\"admin.so\").create_network(1)",
    "include_lib(\"admin.so\").create_network({\"essid\": 1})",
    "include_lib(\"admin.so\").network(1)",
    "include_lib(\"admin.so\").network(get_router.public_ip).add_computer(1)",
    "include_lib(\"admin.so\").network(get_router.public_ip).add_computer({\"users\": [1]})",
    "include_lib(\"admin.so\").network(get_router.public_ip).computer(1)",
    "include_lib(\"admin.so\").network(get_router.public_ip).computer(get_shell.host_computer.local_ip).shell(1)",
    "include_lib(\"admin.so\").set_date(1)",
    "get_router.device_ports(\"\")",
    "nslookup(\"\")",
    "whois(1)",
    "get_shell.ping(1)",
    "get_shell.connect_service(\"\", 22, \"a\", \"b\")",
    "get_shell.connect_service(\"1.1.1.1\", \"22\", \"a\", \"b\")",
    "get_shell.scp(\"\", \"/\")",
];

/// Changes the world: a file in the home folder and a network with one
/// computer.
pub const CHANGES: &str = r#"
            get_shell.host_computer.touch(home_dir, "kept.txt")
            include_lib("admin.so").create_network({"public_ip": "5.5.5.5", "computers": 1})
        "#;

/// Prints whether the world has the `CHANGES`.
pub const CHECKS: &str = r#"
            print get_shell.host_computer.File(home_dir + "/kept.txt") != null
            print get_router("5.5.5.5") != null and get_router("5.5.5.5").devices_lan_ip.len >= 1
            print include_lib("admin.so").networks.len
        "#;

/// Runs `code` on the world `options` generates, written to `database` at the end; responds with what it printed.
pub fn run_generated(options: &WorldOptions, database: &Database, code: &str) -> String {
    run_local(async {
        let server = WorldServer::with_database(
            World::generate(options).await.expect("a generated world"),
            database.clone(),
            constant_clock(),
        )
        .await
        .expect("the world is written to the database");

        run_and_save(&server, code).await
    })
}

/// Runs `code` on the world opened from `database`, written back at the end; responds with what it printed.
pub fn run_opened(database: &Database, code: &str) -> String {
    run_local(async {
        let server = WorldServer::open(database.clone(), constant_clock())
            .await
            .expect("open");

        run_and_save(&server, code).await
    })
}

/// Runs `code` as the player, an admin, then writes the world; responds with what it printed.
async fn run_and_save(server: &WorldServer, code: &str) -> String {
    let (lines, options) = snapshot_options();
    let user = PlayerOptions::default().user;

    if let Some(error) = greyhack::run_in(server, code, &user, true, options).await {
        panic!("{error}");
    }

    server.save().await.expect("the world is written");

    lines.text()
}

/// Runs the programs `first` and `second` at once as the player on one world, returning what each printed and the
/// world after both ended.
pub fn run_both_on_host(first: &str, second: &str, world: World) -> (String, String, World) {
    let user = PlayerOptions::default().user;

    run_local(async {
        let server = WorldServer::with_database(
            world,
            Database::in_memory().await.expect("an in-memory database"),
            constant_clock(),
        )
        .await
        .expect("the world is written to the database");
        let (first_lines, first_options) = snapshot_options();
        let (second_lines, second_options) = snapshot_options();
        let (first_error, second_error) = futures_util::future::join(
            greyhack::run_in(&server, first, &user, true, first_options),
            greyhack::run_in(&server, second, &user, true, second_options),
        )
        .await;

        assert_eq!((first_error, second_error), (None, None));

        server.save().await.expect("the world is written");

        let world = World::load(server.database().expect("a database"))
            .await
            .expect("the world loads");

        (first_lines.text(), second_lines.text(), world)
    })
}
