use greybel_io::{Output, Screen};
use greybel_io_terminal::TerminalScreen;

pub const ERASE_LINE: &str = "\x1b[2K\x1b[1G";
pub const PREVIOUS_LINE: &str = "\x1b[1F";
pub const CLEAR: &str = "\x1b[2J\x1b[1;1H";

pub fn screen_text(batch: &[Output]) -> String {
    let screen = TerminalScreen::new(Vec::new());

    screen.show(batch);

    String::from_utf8(screen.writer().clone()).expect("utf-8")
}
