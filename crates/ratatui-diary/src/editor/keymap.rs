//! 에디터 화면의 키 입력 → 메시지 매핑 (Emacs 스타일).

use crate::{editor::state::{EditorState,
                            EditorSubMode},
            message::Msg};
use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyModifiers};

pub fn handle_key(key: KeyEvent, state: &EditorState) -> Option<Msg> {
    let mods = key.modifiers;
    let control = mods == KeyModifiers::CONTROL;
    let alt = mods == KeyModifiers::ALT;
    let text = mods.is_empty() || mods == KeyModifiers::SHIFT;
    match &state.submode {
        | Some(EditorSubMode::CtrlX) => return None, // App dispatcher handles the shared prefix.
        | Some(EditorSubMode::Search) => {
            return match key.code {
                | KeyCode::Char('s') if control => Some(Msg::EditorSearchNext),
                | KeyCode::Char('r') if control => Some(Msg::EditorSearchPrev),
                | KeyCode::Char('h') if control => Some(Msg::EditorSearchBackspace),
                | KeyCode::Char(c) if text => Some(Msg::EditorSearchChar(c)),
                | KeyCode::Enter => Some(Msg::EditorExecuteSearch),
                | KeyCode::Backspace => Some(Msg::EditorSearchBackspace),
                | _ => None,
            };
        },
        | None => {},
    }
    match key.code {
        | KeyCode::Char('f') if control => Some(Msg::EditorMoveRight),
        | KeyCode::Char('b') if control => Some(Msg::EditorMoveLeft),
        | KeyCode::Char('n') if control => Some(Msg::EditorMoveDown),
        | KeyCode::Char('p') if control => Some(Msg::EditorMoveUp),
        | KeyCode::Char('a') if control => Some(Msg::EditorGotoLineStart),
        | KeyCode::Char('e') if control => Some(Msg::EditorGotoLineEnd),
        | KeyCode::Char('h') if control => Some(Msg::EditorBackspace),
        | KeyCode::Char('o') if control => Some(Msg::EditorOpenLine),
        | KeyCode::Char('d') if control => Some(Msg::EditorDeleteForward),
        | KeyCode::Char('k') if control => Some(Msg::EditorKillLine),
        | KeyCode::Char(' ' | '@') | KeyCode::Null if control => Some(Msg::EditorToggleSelection),
        | KeyCode::Char('@') if mods == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) => Some(Msg::EditorToggleSelection),
        | KeyCode::Null if mods.is_empty() => Some(Msg::EditorToggleSelection),
        | KeyCode::Char('w') if control => Some(Msg::EditorDelete),
        | KeyCode::Char('y') if control => Some(Msg::EditorPaste),
        | KeyCode::Char('/' | '_' | 'z') if control => Some(Msg::EditorUndo),
        | KeyCode::Char('_' | '/') if mods == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) => Some(Msg::EditorUndo),
        | KeyCode::Char('Z' | 'z' | '?') if mods == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) => Some(Msg::EditorRedo),
        | KeyCode::Char('Z' | '?') if control => Some(Msg::EditorRedo),
        | KeyCode::Char('s') if control => Some(Msg::EditorEnterSearchMode),
        | KeyCode::Char('r') if control => Some(Msg::EditorEnterReverseSearchMode),
        | KeyCode::Char('x') if control => Some(Msg::EditorEnterCtrlXMode),
        | KeyCode::Char('q') if control => Some(Msg::Quit),
        | KeyCode::Char('j' | 'm') if control => Some(Msg::EditorNewLine),
        | KeyCode::Char('f') if alt => Some(Msg::EditorWordNext),
        | KeyCode::Char('b') if alt => Some(Msg::EditorWordPrev),
        | KeyCode::Char('d') if alt => Some(Msg::EditorKillWord),
        | KeyCode::Backspace | KeyCode::Char('\u{7f}') if alt => Some(Msg::EditorBackwardKillWord),
        | KeyCode::Char('<') if alt || mods == (KeyModifiers::ALT | KeyModifiers::SHIFT) => Some(Msg::EditorGotoDocStart),
        | KeyCode::Char('>') if alt || mods == (KeyModifiers::ALT | KeyModifiers::SHIFT) => Some(Msg::EditorGotoDocEnd),
        | KeyCode::Char('w') if alt => Some(Msg::EditorYank),
        | KeyCode::Left if mods.is_empty() => Some(Msg::EditorMoveLeft),
        | KeyCode::Right if mods.is_empty() => Some(Msg::EditorMoveRight),
        | KeyCode::Up if mods.is_empty() => Some(Msg::EditorMoveUp),
        | KeyCode::Down if mods.is_empty() => Some(Msg::EditorMoveDown),
        | KeyCode::Home if control => Some(Msg::EditorGotoDocStart),
        | KeyCode::End if control => Some(Msg::EditorGotoDocEnd),
        | KeyCode::Home if mods.is_empty() => Some(Msg::EditorGotoLineStart),
        | KeyCode::End if mods.is_empty() => Some(Msg::EditorGotoLineEnd),
        | KeyCode::Delete if mods.is_empty() => Some(Msg::EditorDeleteForward),
        | KeyCode::Backspace if mods.is_empty() => Some(Msg::EditorBackspace),
        | KeyCode::Enter if mods.is_empty() => Some(Msg::EditorNewLine),
        | KeyCode::Char(c) if text => Some(Msg::EditorInsertChar(c)),
        | _ => None,
    }
}
