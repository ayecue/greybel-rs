//! Player globals (`PlayerIntrinsics`), one file per global.
//! `GAME_API` registers each native as `player::<name>`.

use greybel_interpreter::{Key, Side};
use unicode_general_category::{GeneralCategory, get_general_category};

pub(crate) mod active_user;
pub(crate) mod cd;
pub(crate) mod clear_screen;
pub(crate) mod command_info;
pub(crate) mod current_date;
pub(crate) mod current_path;
pub(crate) mod exit;
pub(crate) mod format_columns;
pub(crate) mod get_abs_path;
pub(crate) mod get_ctf;
pub(crate) mod get_custom_object;
pub(crate) mod get_router;
pub(crate) mod get_shell;
pub(crate) mod get_switch;
pub(crate) mod home_dir;
pub(crate) mod include_lib;
pub(crate) mod is_lan_ip;
pub(crate) mod is_valid_ip;
pub(crate) mod launch_path;
pub(crate) mod mail_login;
pub(crate) mod nslookup;
pub(crate) mod parent_path;
pub(crate) mod poll_input;
pub(crate) mod program_path;
pub(crate) mod reset_ctf_password;
pub(crate) mod user_bank_number;
pub(crate) mod user_input;
pub(crate) mod user_mail_address;
pub(crate) mod whois;

/// Additional terminal-accepted symbols beyond letters, digits, punctuation, and separators.
const SYMBOLS: &str = "=+|^$<>";

/// Converts `key` to Grey Hack input text, or `None` if the terminal ignores it and continues waiting.
/// Enter returns the empty line.
pub(crate) fn input(key: Key) -> Option<String> {
    let name = match key {
        Key::Char(char) => return accepts(char).then(|| char.to_string()),
        Key::Enter => "",
        Key::Escape => "Escape",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Delete => "Delete",
        Key::Insert => "Insert",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Up => "UpArrow",
        Key::Down => "DownArrow",
        Key::Left => "LeftArrow",
        Key::Right => "RightArrow",
        Key::F(number) => return Some(format!("F{number}")),
        Key::Shift(side) => return Some(format!("{}Shift", side_name(side))),
        Key::Control(side) => return Some(format!("{}Control", side_name(side))),
        Key::Alt(side) => return Some(format!("{}Alt", side_name(side))),
    };

    Some(name.to_owned())
}

/// Returns terminal echo for `key`: the character itself, an empty string for Enter, or `None` for other keys.
pub(crate) fn echo(key: Key) -> Option<String> {
    match key {
        Key::Char(char) => Some(char.to_string()),
        Key::Enter => Some(String::new()),
        _ => None,
    }
}

fn side_name(side: Side) -> &'static str {
    match side {
        Side::Left => "Left",
        Side::Right => "Right",
    }
}

/// Validates input characters by Unicode category and [`SYMBOLS`].
fn accepts(char: char) -> bool {
    use GeneralCategory as Category;

    SYMBOLS.contains(char)
        || matches!(
            get_general_category(char),
            Category::UppercaseLetter
                | Category::LowercaseLetter
                | Category::TitlecaseLetter
                | Category::ModifierLetter
                | Category::OtherLetter
                | Category::DecimalNumber
                | Category::ConnectorPunctuation
                | Category::DashPunctuation
                | Category::OpenPunctuation
                | Category::ClosePunctuation
                | Category::InitialPunctuation
                | Category::FinalPunctuation
                | Category::OtherPunctuation
                | Category::SpaceSeparator
                | Category::LineSeparator
                | Category::ParagraphSeparator
        )
}

const COLUMN_GAP: &str = "  ";

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
