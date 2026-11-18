//! Runs the `greybel` binary in isolated temporary projects populated by each test.

pub mod build;
pub mod world;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary test project deleted on drop.
pub struct Project {
    pub folder: PathBuf,
}

impl Project {
    /// Creates an empty project and writes each `(relative_path, contents)` file into it.
    pub fn new(files: &[(&str, &str)]) -> Self {
        static COUNT: AtomicUsize = AtomicUsize::new(0);

        let folder = std::env::temp_dir().join(format!(
            "greybel-cli-{}-{}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&folder);

        std::fs::create_dir_all(&folder).expect("creates the folder");

        for (path, content) in files {
            let path = folder.join(path);

            std::fs::create_dir_all(path.parent().expect("a folder")).expect("creates the folder");
            std::fs::write(path, content).expect("writes the file");
        }

        Self { folder }
    }

    /// Runs `greybel` with `arguments` in the project directory and writes `input` to its stdin. Programs run in
    /// worlds of their own.
    pub fn run(&self, arguments: &[&str], input: &str) -> Run {
        let mut command = Command::new(env!("CARGO_BIN_EXE_greybel"));

        command.env("GREYBEL_OWN_WORLD", "true");

        self.run_command(command, arguments, input)
    }

    /// Runs `greybel` like [`Project::run`], with programs in the shared world server, which saves its worlds in
    /// `worlds`.
    pub fn run_in_machine_world(&self, arguments: &[&str], worlds: &Path) -> Run {
        let mut command = Command::new(env!("CARGO_BIN_EXE_greybel"));

        command
            .env_remove("GREYBEL_OWN_WORLD")
            .env("GREYBEL_WORLDS", worlds);

        self.run_command(command, arguments, "")
    }

    fn run_command(&self, mut command: Command, arguments: &[&str], input: &str) -> Run {
        let mut child = command
            .args(arguments)
            .env_remove("GREYBEL_PROFILE")
            .current_dir(&self.folder)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("greybel starts");

        child
            .stdin
            .take()
            .expect("an input")
            .write_all(input.as_bytes())
            .expect("writes the input");

        Run(child.wait_with_output().expect("greybel ends"))
    }

    /// The path of `path` relative to the project directory.
    pub fn path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.folder.join(path)
    }

    /// Reads the file at `path` relative to the project directory.
    pub fn read(&self, path: impl AsRef<Path>) -> String {
        std::fs::read_to_string(self.folder.join(path)).expect("reads the file")
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// Captured exit status, stdout, and stderr from a `greybel` process.
pub struct Run(Output);

impl Run {
    pub fn success(&self) -> bool {
        self.0.status.success()
    }

    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.0.stdout).into_owned()
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.0.stderr).into_owned()
    }
}

impl std::fmt::Debug for Run {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}\nstdout:\n{}\nstderr:\n{}",
            self.0.status,
            self.stdout(),
            self.stderr()
        )
    }
}
