//! 최상위 키 입력 디스패처.
//!
//! 전역 키(에러 팝업 닫기)를 먼저 처리하고, 그 외에는 현재 화면의
//! 기능 모듈 키맵으로 위임한다.

use crate::{app::{Model,
                  Screen},
            calendar,
            editor,
            message::Msg};
use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyEventKind,
                       KeyModifiers};

pub fn handle_key(key: KeyEvent, model: &Model) -> Option<Msg> {
    // Windows reports both key press and key release events. Handling the
    // release as input would insert every character twice.
    if key.kind == KeyEventKind::Release {
        return None;
    }

    let cancel = key.code == KeyCode::Esc || (key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::CONTROL);
    if model.show_error_popup && cancel {
        return Some(Msg::DismissError);
    }

    if model.show_error_popup {
        return None;
    }
    if key.code == KeyCode::F(2) {
        return Some(Msg::ToggleSettings);
    }
    if model.show_settings {
        if cancel {
            return Some(Msg::ToggleSettings);
        }
        if key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char('n' | 'p')) {
            return Some(Msg::SettingsMove);
        }
        return match key.code {
            | KeyCode::Esc => Some(Msg::ToggleSettings),
            | KeyCode::Up | KeyCode::Down | KeyCode::Tab | KeyCode::BackTab => Some(Msg::SettingsMove),
            | KeyCode::Left => Some(Msg::SettingsChange(false)),
            | KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => Some(Msg::SettingsChange(true)),
            | _ => None,
        };
    }
    if cancel {
        return Some(Msg::EditorKeyboardQuit);
    }
    if model.editor_state.submode == Some(editor::state::EditorSubMode::CtrlX) {
        return Some(match (key.modifiers, key.code) {
            | (KeyModifiers::CONTROL, KeyCode::Char('c')) => Msg::Quit,
            | (KeyModifiers::CONTROL, KeyCode::Char('s')) | (KeyModifiers::NONE, KeyCode::Char('s')) if model.screen == Screen::Editor => Msg::EditorCtrlXSave,
            | (KeyModifiers::NONE, KeyCode::Char('k')) if model.screen == Screen::Editor => Msg::EditorCtrlXBack,
            | (KeyModifiers::NONE, KeyCode::Char('u')) if model.screen == Screen::Editor => Msg::EditorUndo,
            | _ => Msg::EditorExitSubMode,
        });
    }
    match model.screen {
        | Screen::Calendar => calendar::keymap::handle_key(key),
        | Screen::Editor => editor::keymap::handle_key(key, &model.editor_state),
    }
}
