use greybel_io::{Key, Side};

/// Converts keys to names expected by Grey Hack's terminal.
pub(super) trait GameKey {
    /// Returns the character or Unity `KeyCode` name for game any-key input.
    fn game_name(self) -> String;
}

impl GameKey for Key {
    fn game_name(self) -> String {
        let name = match self {
            Key::Char(char) => return char.to_string(),
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
            Key::F(number) => return format!("F{number}"),
            Key::Shift(side) => return format!("{}Shift", side.game_name()),
            Key::Control(side) => return format!("{}Control", side.game_name()),
            Key::Alt(side) => return format!("{}Alt", side.game_name()),
        };

        name.to_owned()
    }
}

impl GameKey for Side {
    fn game_name(self) -> String {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
        }
        .to_owned()
    }
}
