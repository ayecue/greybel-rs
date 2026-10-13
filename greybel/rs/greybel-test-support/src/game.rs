//! Launches headless Grey Hack through `greyhack-test-support`. Clones the installed game (`GREY_HACK_PATH`) with
//! BepInEx major `BEPINEX` (default 5) and only the message-hook and single-player test-support plugins.
//!
//! Tests normally lock one of `GREYHACK_TEST_COPIES` (default 4) shared `game start` sessions, each with a distinct
//! clone, world, and ports. `GREYHACK_TEST_REUSE=0` instead starts an isolated `game run` child stopped on drop.
//! Steam must be logged in. Tests pass `CARGO_TARGET_TMPDIR` for clone and tool storage.

use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use greyhack_message_hook_client::{Client, ClientOptions};

/// Timeout for isolated game shutdown after closing the tool's stdin.
const STOP_TIME: Duration = Duration::from_secs(10);
/// Timeout for plugin player discovery after desktop boot.
const CONNECT_TIME: Duration = Duration::from_secs(30);
/// Grey Hack program execution timeout.
pub const RUN_TIME: Duration = Duration::from_secs(60);
/// Trailing game-log lines to print on test panic.
const LOG_TAIL: usize = 80;
/// Number of concurrent game copies unless overridden by `GREYHACK_TEST_COPIES`. The `game` group in
/// `.config/nextest.toml` uses the same test concurrency.
const COPIES: usize = 4;

/// Handle for an isolated game process or locked shared session.
pub struct Game {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    port: u16,
    test_port: u16,
    log: PathBuf,
    _session_lock: Option<File>,
}

impl Game {
    /// Starts or joins a game and awaits its desktop and both protocol listeners. Uses the test's
    /// `CARGO_TARGET_TMPDIR` as `folder` for the tool build and game clone.
    #[must_use]
    pub fn start(folder: impl AsRef<Path>) -> Self {
        let folder = folder.as_ref();

        if std::env::var("GREYHACK_TEST_REUSE").is_ok_and(|value| value == "0") {
            Self::start_process(folder)
        } else {
            Self::start_session(folder)
        }
    }

    fn start_session(folder: &Path) -> Self {
        let major = bepinex_major();
        let (work_folder, session_lock) = take_copy(folder, &major);
        let mut tool = start_tool(
            folder,
            [
                OsStr::new("game"),
                OsStr::new("start"),
                OsStr::new("--bepinex"),
                OsStr::new(&major),
                OsStr::new("--work-folder"),
                work_folder.as_os_str(),
            ],
        );

        tool.stdin.take();

        let status = tool.child.wait().expect("the session command finishes");

        assert!(status.success(), "the session command failed ({status})");

        Self {
            child: None,
            stdin: None,
            port: port(&tool.ready, "port"),
            test_port: port(&tool.ready, "testPort"),
            log: log(&tool.ready),
            _session_lock: Some(session_lock),
        }
    }

    fn start_process(folder: &Path) -> Self {
        let major = bepinex_major();
        let work_folder = folder.join(format!("greyhack-game-{major}"));
        let mut tool = start_tool(
            folder,
            [
                OsStr::new("game"),
                OsStr::new("run"),
                OsStr::new("--bepinex"),
                OsStr::new(&major),
                OsStr::new("--work-folder"),
                work_folder.as_os_str(),
            ],
        );

        Self {
            stdin: tool.stdin.take(),
            port: port(&tool.ready, "port"),
            test_port: port(&tool.ready, "testPort"),
            log: log(&tool.ready),
            child: Some(tool.child),
            _session_lock: None,
        }
    }

    /// Returns the message-hook listener port.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns the setup/query test-support protocol port.
    #[must_use]
    pub fn test_port(&self) -> u16 {
        self.test_port
    }

    /// Starts or joins through [`Game::start`], awaits a player, and returns the game with a message-hook client.
    pub async fn start_with_player(folder: impl AsRef<Path>) -> (Self, Client) {
        let mut game = Self::start(folder);
        let client = game.player_client().await;

        (game, client)
    }

    /// Polls message-hook health until a player appears; panics after `CONNECT_TIME`.
    pub async fn player_client(&mut self) -> Client {
        let deadline = Instant::now() + CONNECT_TIME;

        while Instant::now() < deadline {
            self.assert_alive();

            let options = ClientOptions {
                connect_attempts: 1,
                request_timeout: Duration::from_secs(5),
                ..ClientOptions::with_port(self.port)
            };

            if let Ok(client) = Client::connect(options).await {
                if client
                    .health_check()
                    .await
                    .is_ok_and(|status| status.has_player_client)
                {
                    return client;
                }

                client.shutdown().await;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        panic!("the plugin reported no player within {CONNECT_TIME:?}");
    }

    /// Panics when an isolated tool exits. Brokers manage shared sessions instead.
    pub fn assert_alive(&mut self) {
        if let Some(child) = self.child.as_mut()
            && let Some(status) = child.try_wait().expect("the tool's status reads")
        {
            panic!("greyhack-test-support exited with {status} while the game ran");
        }
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            self.stdin.take();

            let deadline = Instant::now() + STOP_TIME;

            while child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(100));
            }

            let _ = child.kill();
            let _ = child.wait();
        }

        if std::thread::panicking() {
            print_log(&self.log);
        }
    }
}

/// Reserves an unused game copy and returns its work folder and lock, waiting when all copies are occupied.
fn take_copy(folder: &Path, major: &str) -> (PathBuf, File) {
    let copies = std::env::var("GREYHACK_TEST_COPIES")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&copies: &usize| copies > 0)
        .unwrap_or(COPIES);
    let lock = |index: usize| {
        let work_folder = folder.join(format!("greyhack-game-{major}-{index}"));

        std::fs::create_dir_all(&work_folder).expect("the copy's work folder is created");

        let lock = File::create(work_folder.join("test.lock")).expect("the copy's test lock opens");

        (work_folder, lock)
    };

    for index in 0..copies {
        let (work_folder, file) = lock(index);

        if file.try_lock().is_ok() {
            eprintln!("the test takes {}", work_folder.display());

            return (work_folder, file);
        }
    }

    let (work_folder, file) = lock(std::process::id() as usize % copies);

    file.lock().expect("the copy's test lock is taken");
    eprintln!("the test takes {} after waiting", work_folder.display());

    (work_folder, file)
}

fn bepinex_major() -> String {
    let major = std::env::var("BEPINEX").unwrap_or_else(|_| "5".to_owned());

    assert!(
        major == "5" || major == "6",
        "BEPINEX names BepInEx 5 or 6, not {major}"
    );

    major
}

/// Extracts port `name` from the tool's ready JSON.
pub fn port(ready: &serde_json::Value, name: &str) -> u16 {
    u16::try_from(ready[name].as_u64().expect("a port")).expect("a port")
}

fn log(ready: &serde_json::Value) -> PathBuf {
    PathBuf::from(ready["log"].as_str().expect("the game's log"))
}

/// Started tool process, optional stdin, ready JSON, and remaining stdout lines.
pub struct Tool {
    pub child: Child,
    pub stdin: Option<ChildStdin>,
    pub ready: serde_json::Value,
    pub output: Lines<BufReader<ChildStdout>>,
}

/// Starts the tool with `arguments` and piped stdin/stdout, then awaits its first ready JSON line.
pub fn start_tool<I, S>(folder: &Path, arguments: I) -> Tool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = tool_command(folder, arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("greyhack-test-support starts");
    let stdin = child.stdin.take();
    let mut output = BufReader::new(child.stdout.take().expect("the tool's output")).lines();
    let Some(Ok(line)) = output.next() else {
        let status = child.wait().expect("the tool's status reads");

        panic!("greyhack-test-support didn't start ({status}); its error is above");
    };
    let ready = serde_json::from_str(&line).expect("the tool writes JSON");

    Tool {
        child,
        stdin,
        ready,
        output,
    }
}

/// Builds a tool `Command` with `arguments` and inherited stderr. Compiles once per test process and uses a file
/// lock to serialize cross-process builds.
pub fn tool_command<I, S>(folder: &Path, arguments: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("dotnet");

    command.arg(tool(folder)).args(arguments);

    command
}

fn tool(folder: &Path) -> &'static Path {
    static TOOL: OnceLock<PathBuf> = OnceLock::new();

    TOOL.get_or_init(|| {
        let output = folder.join("greyhack-test-support");

        std::fs::create_dir_all(&output).expect("the tool's output folder is created");

        let lock = File::create(output.join(".lock")).expect("the tool's lock file opens");

        lock.lock().expect("the tool's lock is taken");

        let project = Path::new(concat!(
            env!("GREYBEL_DOTNET"),
            "/greyhack-test-support/src"
        ));
        let status = Command::new("dotnet")
            .args(["build", "--nologo", "-v", "q", "-o"])
            .arg(&output)
            .arg(project)
            .stdout(Stdio::null())
            .status()
            .expect("dotnet runs");

        assert!(
            status.success(),
            "dotnet build of {} failed",
            project.display()
        );

        output.join("greyhack-test-support.dll")
    })
}

fn print_log(log: &Path) {
    let Ok(file) = File::open(log) else {
        eprintln!("the game wrote no log at {}", log.display());

        return;
    };
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|line| {
            !line.contains("Shader") && !line.contains("#pragma") && !line.contains("subshaders")
        })
        .collect();

    eprintln!("the end of the game's log ({}):", log.display());
    for line in &lines[lines.len().saturating_sub(LOG_TAIL)..] {
        eprintln!("{line}");
    }
}
