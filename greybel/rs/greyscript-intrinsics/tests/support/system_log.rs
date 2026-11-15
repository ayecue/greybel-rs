use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::ready;

use greybel_interpreter::{InputFuture, Key, Stdin};
use greybel_test_support::future::block_on;
use greyhack_behaviour::{LibraryKind, ROOT_USER, Reward, SYSTEM_LOG_PATH, SystemLog};
use greyhack_world::World;
use greyhack_world_core::ComputerId;
use greyhack_world_libraries::{Library, MemoryZone, Weakness};

use super::network::{SERVICE_SEED, ServiceTarget, first_remote_service};
use super::scripts::run_game_with_io;
use super::{set_library, world_options};

/// Address and values of the weakness [`world_with_shell_weakness`] installs.
pub const ADDRESS: &str = "0x1234";
pub const UNSEC_VALUE: &str = "buffer";

/// A generated world with its one other network's first forwarded SSH or FTP service.
pub fn world_with_service() -> (World, ServiceTarget) {
    let world =
        block_on(World::generate(&world_options(SERVICE_SEED, 1))).expect("a generated world");
    let target = first_remote_service(&world);

    (world, target)
}

/// [`world_with_service`], whose service library has a weakness that gives a root shell.
pub fn world_with_shell_weakness() -> (World, ServiceTarget) {
    let (world, target) = world_with_service();
    let kind = if target.service == "ssh" {
        LibraryKind::Ssh
    } else {
        LibraryKind::Ftp
    };
    let path = format!("/lib/{}", kind.file_name());

    set_library(
        &world,
        target.computer,
        &path,
        &Library {
            kind,
            version: "9.9.9".to_owned(),
            zones: vec![MemoryZone {
                address: ADDRESS.to_owned(),
                hide: false,
                weaknesses: vec![Weakness {
                    remote: true,
                    unsec_value: UNSEC_VALUE.to_owned(),
                    details: "Test weakness".to_owned(),
                    requirements: Vec::new(),
                    reward: Reward::Shell,
                    user: ROOT_USER.to_owned(),
                    required_lib: None,
                    req_lib_version: "1.0.0".to_owned(),
                    metaxploit_version: "1.0.0".to_owned(),
                    num_register_users: 0,
                    num_port_forward: 0,
                    num_conn_gateway: 0,
                    path_exist: String::new(),
                    random_path: "/root".to_owned(),
                }],
            }],
        },
    );

    (world, target)
}

/// The system log of `computer` in `world`, if it has one.
pub fn log(world: &World, computer: ComputerId) -> Option<SystemLog> {
    block_on(
        world
            .modules
            .core
            .files
            .lookup
            .content(computer, SYSTEM_LOG_PATH, false),
    )
    .expect("the world reads")
    .map(|content| SystemLog::parse(&content).expect("a system log"))
}

/// The `(action, ip, port, file, bounce address)` of each entry of the system log of `computer` in `world`.
pub fn entries(world: &World, computer: ComputerId) -> Vec<(i64, String, i64, String, String)> {
    log(world, computer)
        .map(|log| {
            log.content_log
                .into_iter()
                .map(|entry| {
                    (
                        entry.action.code(),
                        entry.ip,
                        entry.puerto,
                        entry.file,
                        entry.bounce_ip,
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The public address of `computer` in `world`.
pub fn public_ip(world: &World, computer: ComputerId) -> String {
    block_on(world.modules.core.networks.topology.public_ip(computer)).expect("the world reads")
}

/// The router of the player's network, as a computer.
pub fn router_computer(world: &World, computer: ComputerId) -> ComputerId {
    let router = block_on(world.modules.core.networks.topology.router_of(computer))
        .expect("the world reads")
        .expect("a router");

    ComputerId::new(router.id.number())
}

/// A script that connects to `address:port` as `user` and prints the type of what it gets.
pub fn connect_script(
    address: &str,
    port: u16,
    service: &str,
    user: &str,
    password: &str,
) -> String {
    format!(
        "shell = get_shell.connect_service(\"{address}\", {port}, \"{user}\", \"{password}\", \"{service}\")\nprint typeof(shell)"
    )
}

/// A stdin that types `lines` one at each line request, then nothing.
pub struct LinesStdin {
    lines: RefCell<VecDeque<String>>,
}

impl LinesStdin {
    pub fn new(lines: &[&str]) -> Self {
        Self {
            lines: RefCell::new(lines.iter().map(|line| (*line).to_owned()).collect()),
        }
    }
}

impl Stdin for LinesStdin {
    fn read_line(&self, _prompt: &str, _echo: greybel_interpreter::Echo) -> InputFuture<String> {
        let line = self.lines.borrow_mut().pop_front().unwrap_or_default();

        Box::pin(ready(Ok(line)))
    }

    fn read_key(&self, _prompt: &str) -> InputFuture<Key> {
        Box::pin(ready(Ok(Key::Enter)))
    }

    fn try_read_key(&self, _prompt: &str) -> InputFuture<Option<Key>> {
        Box::pin(ready(Ok(None)))
    }
}

/// `script` run with `lines` typed into its terminals, giving its output, error and world.
pub fn run_typing(script: &str, world: World, lines: &[&str]) -> (String, Option<String>, World) {
    run_game_with_io(
        script,
        world,
        true,
        std::rc::Rc::new(LinesStdin::new(lines)),
        greybel_test_support::interpreter::constant_clock(),
    )
}
