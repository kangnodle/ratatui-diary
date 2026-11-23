use crossterm::event::{KeyCode as TerminalCode,
                       KeyEvent as TerminalKey,
                       KeyEventKind,
                       KeyModifiers as TerminalModifiers};
use diary_application::{Model,
                        Msg,
                        input::{KeyCode,
                                KeyEvent,
                                KeyModifiers}};

pub fn handle_key(key: TerminalKey, model: &Model) -> Option<Msg> { translate(key).and_then(|key| diary_application::app::handle_key(key, model)) }

pub fn translate(key: TerminalKey) -> Option<KeyEvent> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    let code = match key.code {
        | TerminalCode::Backspace => KeyCode::Backspace,
        | TerminalCode::Enter => KeyCode::Enter,
        | TerminalCode::Left => KeyCode::Left,
        | TerminalCode::Right => KeyCode::Right,
        | TerminalCode::Up => KeyCode::Up,
        | TerminalCode::Down => KeyCode::Down,
        | TerminalCode::Home => KeyCode::Home,
        | TerminalCode::End => KeyCode::End,
        | TerminalCode::PageUp => KeyCode::PageUp,
        | TerminalCode::PageDown => KeyCode::PageDown,
        | TerminalCode::Tab => KeyCode::Tab,
        | TerminalCode::BackTab => KeyCode::BackTab,
        | TerminalCode::Delete => KeyCode::Delete,
        | TerminalCode::F(number) => KeyCode::F(number),
        | TerminalCode::Char(ch) => KeyCode::Char(ch),
        | TerminalCode::Null => KeyCode::Null,
        | TerminalCode::Esc => KeyCode::Esc,
        | _ => KeyCode::Unknown,
    };
    let mut modifiers = KeyModifiers::NONE;
    for (terminal, logical) in [
        (TerminalModifiers::CONTROL, KeyModifiers::CONTROL),
        (TerminalModifiers::ALT, KeyModifiers::ALT),
        (TerminalModifiers::SHIFT, KeyModifiers::SHIFT),
    ] {
        if key.modifiers.contains(terminal) {
            modifiers = modifiers | logical;
        }
    }
    if !(key.modifiers - (TerminalModifiers::CONTROL | TerminalModifiers::ALT | TerminalModifiers::SHIFT)).is_empty() {
        modifiers = modifiers | KeyModifiers::OTHER;
    }
    Some(KeyEvent::new(code, modifiers))
}
