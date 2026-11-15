use std::cell::OnceCell;
use std::io;
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    ModifierKeyCode, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::{execute, terminal};
use greybel_io::{Key, Side};

const ENHANCEMENT: KeyboardEnhancementFlags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
    .union(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
    .union(KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS);

pub(crate) enum Press {
    Key(Key),
    Interrupt,
}

#[derive(Default)]
pub(crate) struct Keyboard {
    enhanced: OnceCell<bool>,
}

impl Keyboard {
    pub(crate) fn raw(&self) -> io::Result<RawMode> {
        let enhanced = *self
            .enhanced
            .get_or_init(|| terminal::supports_keyboard_enhancement().unwrap_or(false));

        RawMode::enter(enhanced)
    }
}

pub(crate) struct RawMode {
    enhanced: bool,
}

impl RawMode {
    fn enter(enhanced: bool) -> io::Result<Self> {
        terminal::enable_raw_mode()?;

        let mut mode = Self { enhanced: false };

        if enhanced {
            execute!(io::stdout(), PushKeyboardEnhancementFlags(ENHANCEMENT))?;
            mode.enhanced = true;
        }

        Ok(mode)
    }

    pub(crate) fn next() -> io::Result<Press> {
        loop {
            if let Event::Key(event) = event::read()?
                && let Some(press) = press(event)
            {
                return Ok(press);
            }
        }
    }

    pub(crate) fn poll() -> io::Result<Option<Press>> {
        if !event::poll(Duration::ZERO)? {
            return Ok(None);
        }

        match event::read()? {
            Event::Key(event) => Ok(press(event)),
            _ => Ok(None),
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        if self.enhanced {
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        }

        let _ = terminal::disable_raw_mode();
    }
}

fn press(event: KeyEvent) -> Option<Press> {
    if event.kind == KeyEventKind::Release {
        return None;
    }

    let key = match event.code {
        KeyCode::Char('c' | 'C') if event.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(Press::Interrupt);
        }
        KeyCode::Char(char) => Key::Char(char),
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Tab | KeyCode::BackTab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::F(number) => Key::F(number),
        KeyCode::Modifier(modifier) => modifier_key(modifier)?,
        _ => return None,
    };

    Some(Press::Key(key))
}

fn modifier_key(modifier: ModifierKeyCode) -> Option<Key> {
    Some(match modifier {
        ModifierKeyCode::LeftShift => Key::Shift(Side::Left),
        ModifierKeyCode::RightShift => Key::Shift(Side::Right),
        ModifierKeyCode::LeftControl => Key::Control(Side::Left),
        ModifierKeyCode::RightControl => Key::Control(Side::Right),
        ModifierKeyCode::LeftAlt => Key::Alt(Side::Left),
        ModifierKeyCode::RightAlt => Key::Alt(Side::Right),
        _ => return None,
    })
}

#[cfg(test)]
#[path = "tests/keyboard.rs"]
mod tests;
