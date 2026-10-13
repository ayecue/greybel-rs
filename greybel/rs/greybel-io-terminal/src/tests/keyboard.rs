#[path = "support/keyboard.rs"]
mod support;

use crossterm::event::ModifierKeyCode;
use support::{event, key};

use super::*;

#[test]
fn should_read_presses_as_keys() {
    let codes = [
        KeyCode::Char('a'),
        KeyCode::Up,
        KeyCode::BackTab,
        KeyCode::F(7),
        KeyCode::Modifier(ModifierKeyCode::RightAlt),
        KeyCode::Modifier(ModifierKeyCode::LeftSuper),
        KeyCode::CapsLock,
    ];
    let [char, up, back_tab, f7, right_alt, left_super, caps_lock] = codes.map(key);

    assert_eq!(char, Some(Key::Char('a')));
    assert_eq!(up, Some(Key::Up));
    assert_eq!(back_tab, Some(Key::Tab));
    assert_eq!(f7, Some(Key::F(7)));
    assert_eq!(right_alt, Some(Key::Alt(Side::Right)));
    assert_eq!(left_super, None);
    assert_eq!(caps_lock, None);
}

#[test]
fn should_skip_releases() {
    let release = event(
        KeyCode::Char('a'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    );
    let pressed = press(release);

    assert!(pressed.is_none());
}

#[test]
fn should_interrupt_on_ctrl_c() {
    let ctrl_c = event(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
        KeyEventKind::Press,
    );
    let pressed = press(ctrl_c);

    assert!(matches!(pressed, Some(Press::Interrupt)));
}
