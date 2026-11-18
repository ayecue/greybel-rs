//! Serve one editor runtime over stdin/stdout using the editor terminal renderer. Exit when the editor closes the
//! runtime protocol or its input. Use `--world <address>` to connect programs to a shared TCP world server.
//!
//! `greybel-runtime world locate` prints the shared server's address, starting it if absent;
//! `greybel-runtime world reset` regenerates its world.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use duplex_driver_stdio::StreamDriver;
use greybel_runtime::{
    DEFAULT_IDLE_TIMEOUT, DEFAULT_PORT, Runtime, ServerCommand, SharedWorld, connect_over_tcp,
};
use greyhack_terminal_text::Renderer;
use tokio::task::LocalSet;

/// Exit code for an unusable shared world server; editors respond by starting their own server.
const UNUSABLE: u8 = 2;

#[derive(Parser)]
#[command(name = "greybel-runtime")]
struct Options {
    /// Address of the world server to run programs in.
    #[arg(long, value_name = "ADDRESS")]
    world: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Finds or resets the world server shared on this machine.
    World {
        #[command(subcommand)]
        action: WorldAction,
    },
}

#[derive(Subcommand)]
enum WorldAction {
    /// Prints the shared server's address for `--seed`, starting it with `--server` if absent.
    /// Exits with 2 for a different seed or an unusable server.
    Locate {
        #[arg(long, value_name = "PORT", default_value_t = DEFAULT_PORT)]
        port: u16,
        #[arg(long)]
        seed: String,
        /// Seconds without connections before a started server stops; 0 disables the timeout.
        #[arg(long, value_name = "SECONDS", default_value_t = DEFAULT_IDLE_TIMEOUT.as_secs())]
        idle_timeout: u64,
        /// `greyhack-world-server` executable used to start a server.
        #[arg(long, value_name = "PROGRAM")]
        server: PathBuf,
        /// Folder where a started server saves each seed's world [default: `greybel/worlds` in the user's data
        /// folder].
        #[arg(long, value_name = "FOLDER")]
        worlds: Option<PathBuf>,
    },
    /// Regenerates the shared server's world, interrupting programs running in it. Exits with 2 when no usable
    /// server runs.
    Reset {
        #[arg(long, value_name = "PORT", default_value_t = DEFAULT_PORT)]
        port: u16,
    },
}

fn main() -> ExitCode {
    let options = Options::parse();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, async {
        match options.command {
            Some(Command::World { action }) => world(action).await,
            None => serve(options.world).await,
        }
    })
}

async fn serve(world: Option<String>) -> ExitCode {
    let transport = StreamDriver::stdio();

    tokio::task::spawn_local(Renderer::new(&transport).serve());

    let runtime = match world {
        Some(address) => match connect_over_tcp(&address).await {
            Ok(world) => Runtime::with_world(&transport, world),
            Err(error) => {
                eprintln!("the world server at {address} is unreachable: {error}");

                return ExitCode::FAILURE;
            }
        },
        None => Runtime::new(&transport),
    };

    runtime.serve().await;

    ExitCode::SUCCESS
}

async fn world(action: WorldAction) -> ExitCode {
    let outcome = match action {
        WorldAction::Locate {
            port,
            seed,
            idle_timeout,
            server,
            worlds,
        } => {
            let world = SharedWorld::new(
                port,
                Duration::from_secs(idle_timeout),
                ServerCommand::new(
                    server,
                    worlds
                        .into_iter()
                        .flat_map(|worlds| ["--worlds".into(), worlds.into_os_string()]),
                ),
            );

            world.connect(&seed).await.map(|_| world.address())
        }
        WorldAction::Reset { port } => SharedWorld::reset(port).await,
    };

    match outcome {
        Ok(text) => {
            println!("{text}");

            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("{failure}");

            ExitCode::from(UNUSABLE)
        }
    }
}
