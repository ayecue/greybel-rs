use std::io;
use std::rc::Rc;

use greybel_interpreter::{Echo, Stdin, Value, native};

use crate::error::IntrinsicError;
use crate::game::Terminal;
use crate::game::client::login_encryption::LoginEncryption;

/// Reads a line, or one Grey Hack key name when `anyKey` is true.
/// `addToHistory` has no effect.
#[allow(
    clippy::too_many_arguments,
    reason = "the game's five parameters and three host values"
)]
#[native]
pub async fn user_input(
    #[host] stdin: Rc<dyn Stdin>,
    #[host] encryption: LoginEncryption,
    #[host] terminal: Terminal,
    #[default("")]
    #[name = "inputMsg"]
    message: Value,
    #[default(0)]
    #[name = "isPassword"]
    password: Value,
    #[default(0)]
    #[name = "anyKey"]
    any_key: Value,
    #[default(0)]
    #[name = "addToHistory"]
    history: Value,
    #[default(1)] echo: Value,
) -> Result<String, IntrinsicError> {
    encryption.deny("user_input")?;

    let (Some(message), Some(password), Some(any_key), Some(_), Some(echo)) = (
        message.as_str(),
        password.as_number(),
        any_key.as_number(),
        history.as_number(),
        echo.as_number(),
    ) else {
        return Err(IntrinsicError::InvalidArguments("user_input"));
    };
    // Echo typed text unless `echo` is false; mask passwords with `*` (`TerminalListAdapter.GetPasswordBuildLine`).
    let echo = match (echo != 0.0, password != 0.0) {
        (false, _) => Echo::Hidden,
        (true, true) => Echo::Masked,
        (true, false) => Echo::Visible,
    };

    terminal.send();

    let read = if any_key != 0.0 {
        read_key(&*stdin, &terminal, message).await
    } else {
        stdin.read_line(message, echo).await
    };

    Ok(read.unwrap_or_default())
}

/// Reads until the terminal accepts a key, reproducing its echo behavior.
async fn read_key(stdin: &dyn Stdin, terminal: &Terminal, prompt: &str) -> io::Result<String> {
    let mut prompt = prompt;

    loop {
        let key = stdin.read_key(prompt).await?;

        if let Some(input) = super::input(key) {
            if let Some(echo) = super::echo(key) {
                terminal.show(&echo);
            }

            return Ok(input);
        }

        prompt = "";
    }
}
