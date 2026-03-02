use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

pub fn next_event(timeout: Duration) -> Result<Option<AppEvent>> {
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key) => Ok(Some(AppEvent::Key(key))),
            _ => Ok(None),
        }
    } else {
        Ok(Some(AppEvent::Tick))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    TogglePlay,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    ToggleShuffle,
    ToggleRepeat,
    NavUp,
    NavDown,
    NavLeft,
    NavRight,
    Select,
    Search,
    Tab,
    Escape,
    Char(char),
    Backspace,
    Refresh,
}

pub fn key_to_action(key: &KeyEvent) -> Option<Action> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) => Some(Action::Quit),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Quit),
        (KeyCode::Char(' '), _) => Some(Action::TogglePlay),
        (KeyCode::Char('n'), _) | (KeyCode::Char('>'), _) => Some(Action::Next),
        (KeyCode::Char('p'), _) | (KeyCode::Char('<'), _) => Some(Action::Previous),
        (KeyCode::Char('+'), _) | (KeyCode::Char('='), _) => Some(Action::VolumeUp),
        (KeyCode::Char('-'), _) => Some(Action::VolumeDown),
        (KeyCode::Char('s'), _) => Some(Action::ToggleShuffle),
        (KeyCode::Char('r'), _) => Some(Action::ToggleRepeat),
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => Some(Action::NavDown),
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => Some(Action::NavUp),
        (KeyCode::Char('h'), _) | (KeyCode::Left, _) => Some(Action::NavLeft),
        (KeyCode::Char('l'), _) | (KeyCode::Right, _) => Some(Action::NavRight),
        (KeyCode::Enter, _) => Some(Action::Select),
        (KeyCode::Char('/'), _) => Some(Action::Search),
        (KeyCode::Tab, _) => Some(Action::Tab),
        (KeyCode::Esc, _) => Some(Action::Escape),
        (KeyCode::Backspace, _) => Some(Action::Backspace),
        (KeyCode::Char('R'), _) => Some(Action::Refresh),
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            Some(Action::Char(c))
        }
        _ => None,
    }
}
