use crossterm::event::KeyEventState;

use super::super::*;

pub fn event(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind,
        state: KeyEventState::NONE,
    }
}

pub fn key(code: KeyCode) -> Option<Key> {
    match press(event(code, KeyModifiers::NONE, KeyEventKind::Press)) {
        Some(Press::Key(key)) => Some(key),
        _ => None,
    }
}
