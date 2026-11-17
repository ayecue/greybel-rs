use std::io::{self, IsTerminal};

use greybel_io::{Output, Screen};
use greybel_io_terminal::TerminalScreen;
use greybel_project::LineLimits as ProjectLineLimits;
use greyhack_terminal_text::{LineLimits, LineSplitRule, ansi_text, plain_text};

/// Program-output stdout renderer. Uses game-style ANSI rich text at terminal width for terminals and plain text otherwise,
/// with long lines split by `split_rule`.
pub struct RichTextScreen {
    screen: TerminalScreen,
    styled: bool,
    split_rule: LineSplitRule,
}

impl RichTextScreen {
    pub fn new(split_rule: LineSplitRule) -> Self {
        Self {
            screen: TerminalScreen::default(),
            styled: io::stdout().is_terminal(),
            split_rule,
        }
    }

    /// The split_rule `run.lineLimits` of `greybel.json` sets.
    pub fn split_rule_of(limits: Option<&ProjectLineLimits>) -> LineSplitRule {
        limits.map_or(LineSplitRule::Game, |limits| {
            LineSplitRule::Limits(LineLimits::new(
                limits.text_per_line as usize,
                limits.tags_per_line.map(|limit| limit as usize),
            ))
        })
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
                        ansi_text(text, columns, self.split_rule)
                    } else {
                        plain_text(text, self.split_rule)
                    },
                    options: *options,
                },
                Output::Clear => Output::Clear,
            })
            .collect();

        self.screen.show(&batch);
    }
}
