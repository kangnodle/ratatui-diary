//! 최상위 키 입력 디스패처.
//!
//! 전역 키(에러 팝업 닫기)를 먼저 처리하고, 그 외에는 현재 화면의
//! 기능 모듈 키맵으로 위임한다.

use crate::{app::{Model,
                  Screen},
            calendar,
            editor,
            input::{KeyCode,
                    KeyEvent,
                    KeyModifiers},
            message::Msg};

pub fn handle_key(key: KeyEvent, model: &Model) -> Option<Msg> {
    let cancel = key.code == KeyCode::Esc
        || (key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::CONTROL)
        || (model.settings.keybindings == crate::settings::Keybindings::Helix && key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL);
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
            return Some(if key.code == KeyCode::Char('p') {
                Msg::SettingsMoveBack
            } else {
                Msg::SettingsMove
            });
        }
        return match key.code {
            | KeyCode::Esc => Some(Msg::ToggleSettings),
            | KeyCode::Down | KeyCode::Tab => Some(Msg::SettingsMove),
            | KeyCode::Up | KeyCode::BackTab => Some(Msg::SettingsMoveBack),
            | KeyCode::Char('j') if model.settings.keybindings == crate::settings::Keybindings::Helix => Some(Msg::SettingsMove),
            | KeyCode::Char('k') if model.settings.keybindings == crate::settings::Keybindings::Helix => Some(Msg::SettingsMoveBack),
            | KeyCode::Left => Some(Msg::SettingsChange(false)),
            | KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => Some(Msg::SettingsChange(true)),
            | _ => None,
        };
    }
    if model.screen == Screen::Search {
        return crate::search::handle_key(key, model);
    }
    if model.screen == Screen::Calendar
        && key.code == KeyCode::Char('/')
        && key.modifiers == KeyModifiers::NONE
        && model.editor_state.submode.is_none()
        && model.editor_state.helix.prefix.is_none()
    {
        return Some(Msg::OpenSearch);
    }
    if model.settings.keybindings == crate::settings::Keybindings::Helix {
        return Some(Msg::HelixKey(key));
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
        | Screen::Search => unreachable!(),
    }
}
