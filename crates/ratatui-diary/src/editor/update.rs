//! 에디터 화면의 메시지 처리.
//!
//! 호출 시점에 이미 화면이 [`Screen::Editor`]임이 보장되므로
//! (최상위 [`crate::app::update`] 디스패처가 화면별로 위임) 별도의
//! 화면 가드 없이 자신의 메시지만 처리한다.

use crate::{app::{Command,
                  Model},
            editor::state::{EditorSubMode,
                            Selection},
            message::Msg};
use text_editor::TextEditor;

/// Selection이 활성화되어 있으면 커서 이동시 selection도 업데이트
fn update_selection_on_move(state: &mut TextEditor) {
    if let Some(ref mut sel) = state.selection {
        sel.cursor_line = state.cursor_line;
        sel.cursor_col = state.cursor_col;
    }
}

/// 클립보드 내용을 커서 위치에 붙여넣기
fn paste_clipboard(state: &mut TextEditor) {
    if state.content.is_empty() {
        state.content.push(String::new());
    }
    state.selection = None;
    for ch in state.clipboard.clone().chars() {
        if ch == '\n' {
            state.new_line();
        } else {
            state.insert_char(ch);
        }
    }
}

fn kill_word(state: &mut TextEditor, backward: bool) {
    let origin = (state.cursor_line, state.cursor_col);
    if backward {
        state.move_word_prev();
    } else {
        state.move_word_next();
    }
    if origin == (state.cursor_line, state.cursor_col) {
        return;
    }
    state.selection = Some(Selection {
        anchor_line: origin.0,
        anchor_col: origin.1,
        cursor_line: state.cursor_line,
        cursor_col: state.cursor_col,
    });
    if let Some(text) = state.get_selected_text() {
        state.clipboard = text;
        state.delete_selection();
        state.save_snapshot();
    }
}

pub fn update(model: &mut Model, msg: Msg) -> Option<Command> {
    match msg {
        // ===== 네비게이션 =====
        | Msg::EditorMoveLeft =>
            if model.editor_state.cursor_col > 0 {
                model.editor_state.cursor_col -= 1;
                update_selection_on_move(&mut model.editor_state);
            } else if model.editor_state.cursor_line > 0 {
                model.editor_state.cursor_line -= 1;
                model.editor_state.cursor_col = model.editor_state.content[model.editor_state.cursor_line].chars().count();
                update_selection_on_move(&mut model.editor_state);
            },
        | Msg::EditorMoveRight => {
            let line_len = if model.editor_state.cursor_line < model.editor_state.content.len() {
                model.editor_state.content[model.editor_state.cursor_line].chars().count()
            } else {
                0
            };
            if model.editor_state.cursor_col < line_len {
                model.editor_state.cursor_col += 1;
                update_selection_on_move(&mut model.editor_state);
            } else if model.editor_state.cursor_line + 1 < model.editor_state.content.len() {
                model.editor_state.cursor_line += 1;
                model.editor_state.cursor_col = 0;
                update_selection_on_move(&mut model.editor_state);
            }
        },
        | Msg::EditorMoveUp =>
            if model.editor_state.cursor_line > 0 {
                model.editor_state.cursor_line -= 1;
                let line_len = model.editor_state.content[model.editor_state.cursor_line].chars().count();
                model.editor_state.cursor_col = model.editor_state.cursor_col.min(line_len);
                update_selection_on_move(&mut model.editor_state);
            },
        | Msg::EditorMoveDown =>
            if model.editor_state.cursor_line + 1 < model.editor_state.content.len() {
                model.editor_state.cursor_line += 1;
                let line_len = model.editor_state.content[model.editor_state.cursor_line].chars().count();
                model.editor_state.cursor_col = model.editor_state.cursor_col.min(line_len);
                update_selection_on_move(&mut model.editor_state);
            },
        | Msg::EditorWordNext => {
            model.editor_state.move_word_next();
            update_selection_on_move(&mut model.editor_state);
        },
        | Msg::EditorWordPrev => {
            model.editor_state.move_word_prev();
            update_selection_on_move(&mut model.editor_state);
        },

        // ===== 점프 =====
        | Msg::EditorGotoDocStart => {
            model.editor_state.cursor_line = 0;
            model.editor_state.cursor_col = 0;
            update_selection_on_move(&mut model.editor_state);
        },
        | Msg::EditorGotoDocEnd =>
            if !model.editor_state.content.is_empty() {
                model.editor_state.cursor_line = model.editor_state.content.len() - 1;
                model.editor_state.cursor_col = model.editor_state.content[model.editor_state.cursor_line].chars().count();
                update_selection_on_move(&mut model.editor_state);
            },
        | Msg::EditorGotoLineStart => {
            model.editor_state.cursor_col = 0;
            update_selection_on_move(&mut model.editor_state);
        },
        | Msg::EditorGotoLineEnd => {
            let line_len = if model.editor_state.cursor_line < model.editor_state.content.len() {
                model.editor_state.content[model.editor_state.cursor_line].chars().count()
            } else {
                0
            };
            model.editor_state.cursor_col = line_len;
            update_selection_on_move(&mut model.editor_state);
        },
        // ===== 문자 입력 (항상 활성) =====
        | msg @ (Msg::EditorInsertChar(_) | Msg::EditorBackspace | Msg::EditorNewLine | Msg::EditorOpenLine) => {
            let state = &mut model.editor_state;
            state.selection = None;
            match msg {
                | Msg::EditorInsertChar(c) => state.insert_char(c),
                | Msg::EditorBackspace => state.backspace(),
                | Msg::EditorNewLine => state.new_line(),
                | Msg::EditorOpenLine => state.open_line(),
                | _ => unreachable!(),
            }
            state.save_snapshot();
        },

        // ===== Selection =====
        | Msg::EditorToggleSelection => {
            let state = &mut model.editor_state;
            if state.selection.is_some() {
                state.selection = None;
            } else {
                state.selection = Some(Selection {
                    anchor_line: state.cursor_line,
                    anchor_col: state.cursor_col,
                    cursor_line: state.cursor_line,
                    cursor_col: state.cursor_col,
                });
            }
        },
        | Msg::EditorSelectLine => {
            let state = &mut model.editor_state;
            let line_len = if state.cursor_line < state.content.len() {
                state.content[state.cursor_line].chars().count()
            } else {
                0
            };
            state.selection = Some(Selection {
                anchor_line: state.cursor_line,
                anchor_col: 0,
                cursor_line: state.cursor_line,
                cursor_col: line_len,
            });
        },

        // ===== 편집 기능 =====
        | Msg::EditorDelete =>
        // Selection이 있을 때만 삭제
            if model.editor_state.selection.is_some() {
                if let Some(text) = model.editor_state.get_selected_text() {
                    model.editor_state.clipboard = text;
                }
                model.editor_state.delete_selection();
                model.editor_state.save_snapshot();
            },
        | Msg::EditorDeleteForward => {
            model.editor_state.delete_forward();
            model.editor_state.save_snapshot();
        },
        | Msg::EditorKillLine => {
            let killed = model.editor_state.kill_line();
            if !killed.is_empty() {
                model.editor_state.clipboard = killed;
            }
            model.editor_state.save_snapshot();
        },
        | Msg::EditorKillWord => kill_word(&mut model.editor_state, false),
        | Msg::EditorBackwardKillWord => kill_word(&mut model.editor_state, true),
        | Msg::EditorYank =>
            if let Some(text) = model.editor_state.get_selected_text() {
                model.editor_state.clipboard = text;
                model.editor_state.selection = None;
            },
        | Msg::EditorPaste => {
            paste_clipboard(&mut model.editor_state);
            model.editor_state.save_snapshot();
        },

        // ===== Undo/Redo =====
        | Msg::EditorUndo => model.editor_state.undo(),
        | Msg::EditorRedo => model.editor_state.redo(),

        // ===== 검색 =====
        | Msg::EditorEnterSearchMode => model.editor_state.start_search(false),
        | Msg::EditorEnterReverseSearchMode => model.editor_state.start_search(true),
        | Msg::EditorSearchChar(c) =>
            if model.editor_state.submode == Some(EditorSubMode::Search) {
                model.editor_state.search_pattern.push(c);
                model.editor_state.refresh_search();
            },
        | Msg::EditorSearchBackspace =>
            if model.editor_state.submode == Some(EditorSubMode::Search) {
                model.editor_state.search_pattern.pop();
                model.editor_state.refresh_search();
            },
        | Msg::EditorExecuteSearch =>
            if model.editor_state.submode == Some(EditorSubMode::Search) {
                let state = &mut model.editor_state;
                state.refresh_search();
                if !state.search_backward && !state.search_matches.is_empty() {
                    state.cursor_col += state.search_pattern.chars().count();
                }
                state.last_search_pattern = state.search_pattern.clone();
                state.submode = None;
                state.search_origin = None;
                state.selection = None;
            },
        | msg @ (Msg::EditorSearchNext | Msg::EditorSearchPrev) => {
            let state = &mut model.editor_state;
            state.search_backward = matches!(msg, Msg::EditorSearchPrev);
            if state.search_pattern.is_empty() {
                state.search_pattern = state.last_search_pattern.clone();
                state.refresh_search();
            } else if state.search_backward {
                state.search_prev();
            } else {
                state.search_next();
            }
        },

        // ===== Ctrl+X 프리픽스 =====
        | Msg::EditorCtrlXSave =>
            if model.editor_state.submode == Some(EditorSubMode::CtrlX) {
                let date = model.editor_state.date;
                let content = model.editor_state.get_content();
                model.editor_state.submode = None;
                return Some(Command::SaveDiary(date, content));
            },

        | _ => {},
    }

    None
}
