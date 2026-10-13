use std::io::{self, IsTerminal};

use greybel_io::{Output, Screen};
use greybel_io_terminal::TerminalScreen;
use greyhack_terminal_text::{ansi_text, plain_text};

/// Program-output stdout renderer. Uses game-style ANSI rich text at terminal width for terminals and plain text otherwise.
pub struct RichTextScreen {
    screen: TerminalScreen,
    styled: bool,
}

impl Default for RichTextScreen {
    fn default() -> Self {
        Self {
            screen: TerminalScreen::default(),
            styled: io::stdout().is_terminal(),
        }
    }
}

impl Screen for RichTextScreen {
    fn show(&self, batch: &[Output]) {
        let columns = if self.styled {
            crossterm::terminal::size().map_or(0, |(columns, _)| columns)
        } else {
            0
        };
        let batch: Vec<Output> = batch
            .iter()
            .map(|output| match output {
                Output::Print { text, options } => Output::Print {
                    text: if self.styled {
                        ansi_text(text, columns)
                    } else {
                        plain_text(text)
                    },
                    options: *options,
                },
                Output::Clear => Output::Clear,
            })
            .collect();

        self.screen.show(&batch);
    }
}
