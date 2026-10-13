//! Runtime for a game interpreter and its launched programs.

mod child;
mod connection;
mod handle;
mod interpreter;
mod programs;

pub use child::ChildRunner;
pub(crate) use handle::Handle;
pub use connection::ServerConnection;
use greybel_documents::AsyncResourceProvider;
use greybel_interpreter::{Interpreter, Scheduler};
pub use interpreter::{GreyHackError, GreyHackInterpreter, ProgramBinary};

use crate::{GAME_API, Game};

/// Registers the game API, child runner, and required host data.
fn register_game<P, S>(
    interpreter: Interpreter<P, S>,
    runner: ChildRunner,
    game: Game,
) -> Interpreter<P, S>
where
    P: AsyncResourceProvider,
    S: Scheduler,
{
    let interpreter = interpreter.with_api(GAME_API).with_host_data(game);

    interpreter.with_host_data(runner)
}
