//! Helix's single-selection modal editing, using the shared text engine.
use super::state::{EditorState,
                   Selection};
use crate::{app::{Command,
                  Model,
                  Screen},
            input::{KeyCode,
                    KeyEvent,
                    KeyModifiers},
            message::Msg,
            settings::Language};
use regex::RegexBuilder;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Select,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    Goto,
    Space,
    Command,
    Search,
    Find { backward: bool, till: bool },
    Replace,
}

#[derive(Default)]
pub struct HelixState {
    pub mode: Mode,
    pub prefix: Option<Prefix>,
    pub count: Option<usize>,
    pub prompt: String,
    pub prompt_cursor: usize,
    pub prompt_error: bool,
    insert_before: Option<String>,
    search_selection: Option<Selection>,
    last_find: Option<(char, bool, bool)>,
}

fn offset(state: &EditorState, line: usize, col: usize) -> usize {
    state.content.iter().take(line).map(|s| s.chars().count() + 1).sum::<usize>() + state.content.get(line).map_or(0, |s| col.min(s.chars().count()))
}

fn position(state: &EditorState, mut index: usize) -> (usize, usize) {
    for (line, text) in state.content.iter().enumerate() {
        let len = text.chars().count();
        if index <= len || line + 1 == state.content.len() {
            return (line, index.min(len));
        }
        index -= len + 1;
    }
    (0, 0)
}

fn point(state: &EditorState) -> usize { offset(state, state.cursor_line, state.cursor_col) }

fn cursor_limit(state: &EditorState) -> usize {
    let len = state.get_content().chars().count();
    if state.content.last().is_some_and(String::is_empty) {
        len
    } else {
        len.saturating_sub(1)
    }
}

fn bounds(state: &EditorState) -> (usize, usize) {
    state.get_selection_range().map_or_else(
        || {
            let p = point(state);
            (p, (p + 1).min(state.get_content().chars().count()))
        },
        |((al, ac), (hl, hc))| (offset(state, al, ac), offset(state, hl, hc)),
    )
}

fn assign(state: &mut EditorState, anchor: usize, head: usize) {
    let len = state.get_content().chars().count();
    let anchor = anchor.min(len);
    let head = head.min(len);
    let (anchor_line, anchor_col) = position(state, anchor);
    let (cursor_line, cursor_col) = position(state, head);
    state.selection = Some(Selection {
        anchor_line,
        anchor_col,
        cursor_line,
        cursor_col,
    });
    let cursor = if head > anchor { head - 1 } else { head };
    (state.cursor_line, state.cursor_col) = position(state, cursor);
}

fn collapse(state: &mut EditorState) {
    let p = point(state).min(cursor_limit(state));
    assign(state, p, p + 1);
}

fn move_to(state: &mut EditorState, target: usize) {
    let target = target.min(cursor_limit(state));
    if state.helix.mode == Mode::Select {
        let anchor = state.selection.as_ref().map_or(point(state), |s| {
            let a = offset(state, s.anchor_line, s.anchor_col);
            if (s.anchor_line, s.anchor_col) > (s.cursor_line, s.cursor_col) {
                a.saturating_sub(1)
            } else {
                a
            }
        });
        if target >= anchor {
            assign(state, anchor, target + 1);
        } else {
            assign(state, anchor + 1, target);
        }
    } else {
        assign(state, target, target + 1);
    }
}

fn prepare_edit(state: &mut EditorState) {
    let (line, col) = (state.cursor_line, state.cursor_col);
    let selection = state.selection.clone();
    let index = state.history_index;
    if let Some(snapshot) = state.edit_history.get_mut(index) {
        snapshot.cursor_line = line;
        snapshot.cursor_col = col;
        snapshot.selection = selection;
    }
}

fn finish_insert(state: &mut EditorState) {
    if let Some(before) = state.helix.insert_before.take()
        && before != state.get_content()
    {
        state.save_snapshot();
    }
}

/// Preserve text and undo history, normalizing the selection for the new mode.
pub fn reset(state: &mut EditorState, active: bool) {
    finish_insert(state);
    state.helix = HelixState::default();
    state.submode = None;
    state.search_origin = None;
    state.search_pattern.clear();
    state.search_matches.clear();
    state.current_match_index = 0;
    if active {
        if let Some(s) = state.selection.clone() {
            assign(state, offset(state, s.anchor_line, s.anchor_col), offset(state, s.cursor_line, s.cursor_col));
        } else {
            collapse(state);
        }
    }
}

fn insert_at(state: &mut EditorState, index: usize) {
    prepare_edit(state);
    state.helix.insert_before = Some(state.get_content());
    state.helix.mode = Mode::Insert;
    state.selection = None;
    (state.cursor_line, state.cursor_col) = position(state, index);
}

fn replace_selection(state: &mut EditorState, replacement: &str, checkpoint: bool) {
    if checkpoint {
        prepare_edit(state);
    }
    let (start, _) = bounds(state);
    state.delete_selection();
    for ch in replacement.chars() {
        if ch == '\n' {
            state.new_line();
        } else {
            state.insert_char(ch);
        }
    }
    let end = point(state);
    assign(state, start, end);
    if checkpoint {
        state.save_snapshot();
    }
}

fn category(ch: char, long: bool) -> u8 {
    if ch.is_whitespace() {
        0
    } else if long || ch.is_alphanumeric() || ch == '_' {
        1
    } else {
        2
    }
}

fn word_motion(state: &mut EditorState, key: char, count: usize) {
    let chars: Vec<char> = state.get_content().chars().collect();
    let backward = matches!(key, 'b' | 'B');
    let long = key.is_uppercase();
    let end = matches!(key, 'e' | 'E');
    let cursor = point(state);
    let (mut a, mut h) = if backward {
        (cursor + 1, cursor)
    } else {
        (cursor, (cursor + 1).min(chars.len()))
    };
    for _ in 0 .. count.min(chars.len().max(1)) {
        if backward {
            while h > 0 && chars[h - 1] == '\n' {
                h -= 1;
                a = h;
            }
        } else {
            while h < chars.len() && chars[h] == '\n' {
                h += 1;
                a = h;
            }
        }
        let initial = h;
        while (backward && h > 0) || (!backward && h < chars.len()) {
            let (prev, next) = if backward {
                (chars.get(h).copied(), chars[h - 1])
            } else {
                (h.checked_sub(1).and_then(|i| chars.get(i)).copied(), chars[h])
            };
            let boundary = prev.is_none_or(|p| {
                category(p, long) != category(next, long)
                    && if backward || end {
                        !p.is_whitespace() || next == '\n'
                    } else {
                        !next.is_whitespace() || next == '\n'
                    }
            });
            if boundary {
                if h == initial {
                    a = h;
                } else {
                    break;
                }
            }
            if backward {
                h -= 1;
            } else {
                h += 1;
            }
        }
    }
    if state.helix.mode == Mode::Select {
        move_to(state, if h > a { h - 1 } else { h });
    } else {
        assign(state, a, h);
    }
}

fn motion(state: &mut EditorState, key: KeyCode, count: usize) {
    if let KeyCode::Char(ch @ ('w' | 'b' | 'e' | 'W' | 'B' | 'E')) = key {
        word_motion(state, ch, count);
        return;
    }
    let (line, col) = (state.cursor_line, state.cursor_col);
    let len = state.get_content().chars().count();
    let target = match key {
        | KeyCode::Char('h') | KeyCode::Left => point(state).saturating_sub(count),
        | KeyCode::Char('l') | KeyCode::Right => point(state).saturating_add(count).min(len.saturating_sub(1)),
        | KeyCode::Char('j') | KeyCode::Down => offset(state, line.saturating_add(count).min(state.content.len() - 1), col),
        | KeyCode::Char('k') | KeyCode::Up => offset(state, line.saturating_sub(count), col),
        | KeyCode::Home => offset(state, line, 0),
        | KeyCode::End => offset(state, line, state.content[line].chars().count().saturating_sub(1)),
        | _ => return,
    };
    move_to(state, target);
}

fn find_char(state: &mut EditorState, ch: char, backward: bool, till: bool, count: usize) {
    let chars: Vec<char> = state.get_content().chars().collect();
    let origin = point(state);
    let mut target = origin;
    for _ in 0 .. count.min(chars.len().max(1)) {
        let found = if backward {
            chars[.. target].iter().rposition(|&c| c == ch)
        } else {
            chars.iter().enumerate().skip(target + 1).find(|(_, c)| **c == ch).map(|(i, _)| i)
        };
        let Some(found) = found else {
            return;
        };
        target = found;
    }
    if till {
        target = if backward { target + 1 } else { target.saturating_sub(1) };
    }
    if state.helix.mode == Mode::Select {
        move_to(state, target);
    } else if backward {
        assign(state, origin + 1, target);
    } else {
        assign(state, origin, target + 1);
    }
    state.helix.last_find = Some((ch, backward, till));
}

fn search(state: &mut EditorState, next: bool, reverse: bool) {
    // shortcut: Select n/N awaits a multi-range editor.
    if next && state.helix.mode == Mode::Select {
        return;
    }
    let pattern = &state.search_pattern;
    if pattern.is_empty() {
        if let Some((line, col)) = state.search_origin {
            state.cursor_line = line;
            state.cursor_col = col;
        }
        state.selection = state.helix.search_selection.clone();
        return;
    }
    let Ok(regex) = RegexBuilder::new(pattern).case_insensitive(!pattern.chars().any(char::is_uppercase)).build() else {
        state.helix.prompt_error = true;
        return;
    };
    state.helix.prompt_error = false;
    let text = state.get_content();
    let (mut byte, mut character) = (0, 0);
    let ranges: Vec<_> = regex
        .find_iter(&text)
        .map(|m| {
            character += text[byte .. m.start()].chars().count();
            let start = character;
            character += text[m.start() .. m.end()].chars().count();
            byte = m.end();
            (start, character)
        })
        .collect();
    let from = if next {
        bounds(state).0
    } else {
        state.search_origin.map_or(point(state), |(l, c)| offset(state, l, c))
    };
    let found = if reverse {
        ranges
            .iter()
            .rfind(|(start, _)| if next { *start < from } else { *start <= from })
            .or_else(|| ranges.last())
    } else {
        ranges
            .iter()
            .find(|(start, _)| if next { *start > from } else { *start >= from })
            .or_else(|| ranges.first())
    };
    if let Some(&(start, end)) = found {
        assign(state, start, end);
    }
}

fn error(model: &mut Model, ko: &str, en: &str) {
    model.error_message = Some(model.settings.language.text(ko, en).to_string());
    model.show_error_popup = true;
}

fn command(model: &mut Model) -> Option<Command> {
    let cmd = model.editor_state.helix.prompt.trim().to_string();
    model.editor_state.helix.prefix = None;
    model.editor_state.helix.prompt.clear();
    match cmd.as_str() {
        | "w" | "write" if model.screen == Screen::Editor => Some(Command::SaveDiary(model.editor_state.date, model.editor_state.get_content())),
        | "wq" | "write-quit" | "x" if model.screen == Screen::Editor => crate::app::navigate(model, crate::app::Navigation::Quit, true),
        | "q" | "quit" | "bc" | "buffer-close" if model.editor_state.is_modified => {
            error(
                model,
                "저장하지 않은 변경 사항이 있습니다. :w로 저장하거나 :q!로 버리세요.",
                "Unsaved changes. Use :w to save or :q! to discard.",
            );
            None
        },
        | "q" | "quit" | "q!" | "quit!" => {
            model.should_quit = true;
            None
        },
        | "bc" | "buffer-close" => {
            model.screen = Screen::Calendar;
            Some(Command::LoadPreview(model.calendar_state.selected_date))
        },
        | _ => {
            error(
                model,
                "지원하지 않는 명령입니다. :w, :q, :q!, :wq, :bc를 사용하세요.",
                "Unsupported command. Use :w, :q, :q!, :wq or :bc.",
            );
            None
        },
    }
}

fn escape(state: &mut EditorState) {
    if state.helix.prefix == Some(Prefix::Search) {
        if let Some((line, col)) = state.search_origin.take() {
            state.cursor_line = line;
            state.cursor_col = col;
        }
        state.selection = state.helix.search_selection.take();
        state.search_pattern = state.last_search_pattern.clone();
    } else if state.helix.prefix.is_none() && state.helix.mode == Mode::Insert {
        finish_insert(state);
        state.cursor_col = state.cursor_col.saturating_sub(1);
        collapse(state);
    }
    state.helix.mode = Mode::Normal;
    state.helix.prefix = None;
    state.helix.count = None;
    state.helix.prompt.clear();
    state.helix.prompt_error = false;
}

fn prompt(key: KeyEvent, model: &mut Model) -> Option<Command> {
    if key.code == KeyCode::Enter {
        if model.editor_state.helix.prefix == Some(Prefix::Command) {
            return command(model);
        }
        if model.editor_state.helix.prompt_error {
            error(model, "올바르지 않은 정규식입니다.", "Invalid regular expression.");
            return None;
        }
        model.editor_state.helix.prefix = None;
        model.editor_state.search_origin = None;
        model.editor_state.last_search_pattern = model.editor_state.search_pattern.clone();
        return None;
    }
    let state = &mut model.editor_state;
    let mut chars: Vec<char> = state.helix.prompt.chars().collect();
    let mut pos = state.helix.prompt_cursor.min(chars.len());
    let control = key.modifiers == KeyModifiers::CONTROL;
    match key.code {
        | KeyCode::Char('a') if control => pos = 0,
        | KeyCode::Char('e') if control => pos = chars.len(),
        | KeyCode::Char('b') if control => pos = pos.saturating_sub(1),
        | KeyCode::Char('f') if control => pos = (pos + 1).min(chars.len()),
        | KeyCode::Left => pos = pos.saturating_sub(1),
        | KeyCode::Right => pos = (pos + 1).min(chars.len()),
        | KeyCode::Home => pos = 0,
        | KeyCode::End => pos = chars.len(),
        | KeyCode::Backspace | KeyCode::Char('h') if key.code == KeyCode::Backspace || control =>
            if pos > 0 {
                pos -= 1;
                chars.remove(pos);
            },
        | KeyCode::Delete | KeyCode::Char('d') if key.code == KeyCode::Delete || control =>
            if pos < chars.len() {
                chars.remove(pos);
            },
        | KeyCode::Char('u') if control => {
            chars.drain(.. pos);
            pos = 0;
        },
        | KeyCode::Char('k') if control => {
            chars.truncate(pos);
        },
        | KeyCode::Char('w') if control => {
            let end = pos;
            while pos > 0 && chars[pos - 1].is_whitespace() {
                pos -= 1;
            }
            while pos > 0 && !chars[pos - 1].is_whitespace() {
                pos -= 1;
            }
            chars.drain(pos .. end);
        },
        | KeyCode::Char(ch) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            chars.insert(pos, ch);
            pos += 1;
        },
        | _ => return None,
    }
    state.helix.prompt = chars.into_iter().collect();
    state.helix.prompt_cursor = pos;
    if state.helix.prefix == Some(Prefix::Search) {
        state.search_pattern = state.helix.prompt.clone();
        search(state, false, state.search_backward);
    }
    None
}

fn insert_newline(state: &mut EditorState) {
    let indent: String = state.content[state.cursor_line]
        .chars()
        .take(state.cursor_col)
        .take_while(|c| matches!(c, ' ' | '\t'))
        .collect();
    state.new_line();
    for ch in indent.chars() {
        state.insert_char(ch);
    }
}

fn insert_key(key: KeyEvent, state: &mut EditorState) {
    let control = key.modifiers == KeyModifiers::CONTROL;
    let alt = key.modifiers == KeyModifiers::ALT;
    match key.code {
        | KeyCode::Char('s') if control => {
            finish_insert(state);
            state.helix.insert_before = Some(state.get_content());
        },
        | KeyCode::Char('h') if control => state.backspace(),
        | KeyCode::Char('d') if control => state.delete_forward(),
        | KeyCode::Char('j') if control => insert_newline(state),
        | KeyCode::Char('w') if control => {
            let end = point(state);
            state.move_word_prev();
            let start = point(state);
            assign(state, start, end);
            state.delete_selection();
        },
        | KeyCode::Backspace if alt => {
            let end = point(state);
            state.move_word_prev();
            let start = point(state);
            assign(state, start, end);
            state.delete_selection();
        },
        | KeyCode::Char('d') | KeyCode::Delete if alt => {
            let start = point(state);
            state.move_word_next();
            let end = point(state);
            assign(state, start, end);
            state.delete_selection();
        },
        | KeyCode::Char('u') if control => {
            let end = point(state);
            let start = offset(state, state.cursor_line, 0);
            assign(state, start, end);
            state.delete_selection();
        },
        | KeyCode::Char('k') if control => {
            state.kill_line();
        },
        | KeyCode::Backspace => state.backspace(),
        | KeyCode::Delete => state.delete_forward(),
        | KeyCode::Enter => insert_newline(state),
        | KeyCode::Tab =>
            for _ in 0 .. 4 {
                state.insert_char(' ');
            },
        | KeyCode::Left => state.cursor_col = state.cursor_col.saturating_sub(1),
        | KeyCode::Right => state.cursor_col = (state.cursor_col + 1).min(state.content[state.cursor_line].chars().count()),
        | KeyCode::Home => state.cursor_col = 0,
        | KeyCode::End => state.cursor_col = state.content[state.cursor_line].chars().count(),
        | KeyCode::Up | KeyCode::Down => {
            state.cursor_line = if key.code == KeyCode::Up {
                state.cursor_line.saturating_sub(1)
            } else {
                (state.cursor_line + 1).min(state.content.len() - 1)
            };
            state.cursor_col = state.cursor_col.min(state.content[state.cursor_line].chars().count());
        },
        | KeyCode::Char(ch) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => state.insert_char(ch),
        | _ => {},
    }
    state.selection = None;
}

pub fn update(key: KeyEvent, model: &mut Model) -> Option<Command> {
    let control = key.modifiers == KeyModifiers::CONTROL;
    if key.code == KeyCode::Esc || (control && key.code == KeyCode::Char('c') && model.editor_state.helix.prefix.is_some()) {
        escape(&mut model.editor_state);
        return None;
    }
    if matches!(model.editor_state.helix.prefix, Some(Prefix::Command | Prefix::Search)) {
        return prompt(key, model);
    }
    if model.screen == Screen::Calendar {
        let msg = match key.code {
            | KeyCode::Char(':') => {
                model.editor_state.helix.prefix = Some(Prefix::Command);
                model.editor_state.helix.prompt.clear();
                model.editor_state.helix.prompt_cursor = 0;
                return None;
            },
            | KeyCode::Char('h') | KeyCode::Left => Msg::CalendarMoveLeft,
            | KeyCode::Char('l') | KeyCode::Right => Msg::CalendarMoveRight,
            | KeyCode::Char('j') | KeyCode::Down => Msg::CalendarMoveDown,
            | KeyCode::Char('k') | KeyCode::Up => Msg::CalendarMoveUp,
            | KeyCode::Char('H') => Msg::CalendarPrevMonth,
            | KeyCode::Char('L') => Msg::CalendarNextMonth,
            | KeyCode::Char('{') => Msg::CalendarPrevYear,
            | KeyCode::Char('}') => Msg::CalendarNextYear,
            | KeyCode::Enter => Msg::CalendarSelectDate,
            | _ => return None,
        };
        return crate::calendar::update::update(model, msg);
    }
    let state = &mut model.editor_state;
    if state.helix.mode == Mode::Insert {
        insert_key(key, state);
        return None;
    }
    if state.selection.is_none() {
        collapse(state);
    }
    if key.modifiers == KeyModifiers::ALT && key.code == KeyCode::Char('.') {
        if let Some((ch, backward, till)) = state.helix.last_find {
            let count = state.helix.count.take().unwrap_or(1);
            find_char(state, ch, backward, till, count);
        }
        return None;
    }
    if key.modifiers == KeyModifiers::ALT && key.code == KeyCode::Char(';') {
        if let Some(s) = state.selection.clone() {
            assign(state, offset(state, s.cursor_line, s.cursor_col), offset(state, s.anchor_line, s.anchor_col));
        }
        return None;
    }
    if control || matches!(key.code, KeyCode::PageDown | KeyCode::PageUp) {
        let rows = state.viewport_height.max(1);
        let (down, half) = match key.code {
            | KeyCode::Char('f') | KeyCode::PageDown => (true, false),
            | KeyCode::Char('b') | KeyCode::PageUp => (false, false),
            | KeyCode::Char('d') => (true, true),
            | KeyCode::Char('u') => (false, true),
            | _ => return None,
        };
        let count = state
            .helix
            .count
            .take()
            .unwrap_or(1)
            .saturating_mul(if half { (rows / 2).max(1) } else { rows });
        motion(state, KeyCode::Char(if down { 'j' } else { 'k' }), count);
        return None;
    }
    if !key.modifiers.is_empty() && key.modifiers != KeyModifiers::SHIFT {
        return None;
    }
    let prefix = state.helix.prefix.take();
    if let Some(Prefix::Find {
        backward,
        till,
    }) = prefix
    {
        if let KeyCode::Char(ch) = key.code {
            let count = state.helix.count.take().unwrap_or(1);
            find_char(state, ch, backward, till, count);
        }
        return None;
    }
    if prefix == Some(Prefix::Replace) {
        if let KeyCode::Char(ch) = key.code {
            let replacement: String = state
                .get_selected_text()
                .unwrap_or_default()
                .chars()
                .map(|c| if c == '\n' { c } else { ch })
                .collect();
            replace_selection(state, &replacement, true);
        }
        return None;
    }
    if prefix == Some(Prefix::Space) {
        if key.code == KeyCode::Char('f') {
            state.helix.mode = Mode::Normal;
            return crate::app::navigate(model, crate::app::Navigation::Calendar, false);
        }
        return None;
    }
    if prefix == Some(Prefix::Goto) {
        let count = state.helix.count.take();
        let target = match key.code {
            | KeyCode::Char('g') => offset(state, count.unwrap_or(1).saturating_sub(1).min(state.content.len() - 1), 0),
            | KeyCode::Char('e') => offset(state, state.content.len() - 1, 0),
            | KeyCode::Char('h') => offset(state, state.cursor_line, 0),
            | KeyCode::Char('l') => offset(state, state.cursor_line, state.content[state.cursor_line].chars().count().saturating_sub(1)),
            | KeyCode::Char('s') => offset(
                state,
                state.cursor_line,
                state.content[state.cursor_line].chars().take_while(|c| c.is_whitespace()).count(),
            ),
            | _ => return None,
        };
        move_to(state, target);
        return None;
    }
    if let KeyCode::Char(ch @ '0' ..= '9') = key.code {
        let digit = match ch.to_digit(10) {
            | Some(digit) => digit as usize,
            | None => return None,
        };
        // shortcut: counts cap at 10,000; revisit for large batch edits.
        state.helix.count = Some(state.helix.count.unwrap_or(0).saturating_mul(10).saturating_add(digit).min(10_000));
        return None;
    }
    let count_given = state.helix.count.is_some();
    let count = state.helix.count.take().unwrap_or(1).max(1);
    match key.code {
        | KeyCode::Char(ch @ ('h' | 'j' | 'k' | 'l' | 'w' | 'b' | 'e' | 'W' | 'B' | 'E')) => motion(state, KeyCode::Char(ch), count),
        | code @ (KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End) => motion(state, code, count),
        | KeyCode::Char('g') => {
            state.helix.prefix = Some(Prefix::Goto);
            if count_given {
                state.helix.count = Some(count);
            }
        },
        | KeyCode::Char('G') => {
            let line = if count_given { count - 1 } else { state.content.len() - 1 };
            let p = offset(state, line.min(state.content.len() - 1), 0);
            move_to(state, p);
        },
        | KeyCode::Char(' ') => state.helix.prefix = Some(Prefix::Space),
        | KeyCode::Char(':') => {
            state.helix.prefix = Some(Prefix::Command);
            state.helix.prompt.clear();
            state.helix.prompt_cursor = 0;
        },
        | KeyCode::Char('/' | '?') => {
            state.helix.prefix = Some(Prefix::Search);
            state.search_backward = key.code == KeyCode::Char('?');
            state.search_origin = Some((state.cursor_line, state.cursor_col));
            state.last_search_pattern = state.search_pattern.clone();
            state.helix.search_selection = state.selection.clone();
            state.helix.prompt.clear();
            state.helix.prompt_cursor = 0;
        },
        | KeyCode::Char('n' | 'N') =>
            for _ in 0 .. count {
                search(state, true, state.search_backward ^ (key.code == KeyCode::Char('N')));
            },
        | KeyCode::Char('*') => {
            let selected = state.get_selected_text().unwrap_or_default();
            state.search_pattern = regex::escape(&selected);
            if selected.chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                state.search_pattern.insert_str(0, "\\b");
            }
            if selected.chars().last().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                state.search_pattern.push_str("\\b");
            }
            state.search_backward = false;
            search(state, true, false);
        },
        | KeyCode::Char(ch @ ('f' | 'F' | 't' | 'T')) => {
            state.helix.prefix = Some(Prefix::Find {
                backward: ch.is_uppercase(),
                till: matches!(ch, 't' | 'T'),
            });
            state.helix.count = Some(count);
        },
        | KeyCode::Char('r') => state.helix.prefix = Some(Prefix::Replace),
        | KeyCode::Char('v') => state.helix.mode = if state.helix.mode == Mode::Select { Mode::Normal } else { Mode::Select },
        | KeyCode::Char(';') => collapse(state),
        | KeyCode::Char('%') => {
            let len = state.get_content().chars().count();
            assign(state, 0, len);
        },
        | KeyCode::Char('x' | 'X') => {
            let (start, end) = bounds(state);
            let (first, _) = position(state, start);
            let (last, _) = position(state, end.saturating_sub(1));
            let mut last = last.saturating_add(count - 1).min(state.content.len() - 1);
            if key.code == KeyCode::Char('x') && start == offset(state, first, 0) && end == offset(state, last + 1, 0).min(state.get_content().chars().count())
            {
                last = (last + 1).min(state.content.len() - 1);
            }
            let finish = if last + 1 < state.content.len() {
                offset(state, last + 1, 0)
            } else {
                state.get_content().chars().count()
            };
            assign(state, offset(state, first, 0), finish);
        },
        | KeyCode::Char('y') => {
            state.clipboard = state.get_selected_text().unwrap_or_default();
            state.helix.mode = Mode::Normal;
        },
        | KeyCode::Char('d' | 'c') => {
            let change = key.code == KeyCode::Char('c');
            if change {
                let start = bounds(state).0;
                let selection = state.selection.clone();
                insert_at(state, start);
                state.selection = selection;
            } else {
                prepare_edit(state);
            }
            state.clipboard = state.get_selected_text().unwrap_or_default();
            state.delete_selection();
            if !change {
                state.save_snapshot();
                collapse(state);
                state.helix.mode = Mode::Normal;
            }
        },
        | KeyCode::Char('i' | 'a' | 'I' | 'A' | 'o' | 'O') => {
            let (start, end) = bounds(state);
            let first_line = position(state, start).0;
            let last_line = position(state, end.saturating_sub(1)).0;
            let target = match key.code {
                | KeyCode::Char('i') => start,
                | KeyCode::Char('a') => end,
                | KeyCode::Char('I') => offset(state, state.cursor_line, 0),
                | KeyCode::Char('O') => offset(state, first_line, 0),
                | KeyCode::Char('o') => offset(state, last_line, state.content[last_line].chars().count()),
                | _ => offset(state, state.cursor_line, state.content[state.cursor_line].chars().count()),
            };
            let indent: String = state.content[first_line].chars().take_while(|c| matches!(c, ' ' | '\t')).collect();
            insert_at(state, target);
            if key.code == KeyCode::Char('o') {
                insert_newline(state);
            }
            if key.code == KeyCode::Char('O') {
                state.open_line();
                for ch in indent.chars() {
                    state.insert_char(ch);
                }
            }
        },
        | KeyCode::Char('p' | 'P' | 'R') => {
            if state.clipboard.is_empty() {
                return None;
            }
            let mut text = state.clipboard.repeat(count);
            if key.code == KeyCode::Char('R') {
                replace_selection(state, &text, true);
            } else {
                prepare_edit(state);
                let (start, end) = bounds(state);
                let before = key.code == KeyCode::Char('P');
                let mut at = if before { start } else { end };
                if text.ends_with('\n') {
                    let (line, _) = position(state, if before { start } else { end.saturating_sub(1) });
                    if before {
                        at = offset(state, line, 0);
                    } else if line + 1 < state.content.len() {
                        at = offset(state, line + 1, 0);
                    } else {
                        at = state.get_content().chars().count();
                        text.insert(0, '\n');
                    }
                }
                assign(state, at, at);
                replace_selection(state, &text, false);
                state.save_snapshot();
            }
        },
        | KeyCode::Char('u' | 'U') => {
            for _ in 0 .. count.min(state.edit_history.len()) {
                if key.code == KeyCode::Char('u') {
                    state.undo();
                } else {
                    state.redo();
                }
            }
            if let Some(s) = state.selection.clone() {
                assign(state, offset(state, s.anchor_line, s.anchor_col), offset(state, s.cursor_line, s.cursor_col));
            } else {
                collapse(state);
            }
            state.helix.mode = Mode::Normal;
        },
        | KeyCode::Char('~' | '`') => {
            let selected = state.get_selected_text().unwrap_or_default();
            let text: String = if key.code == KeyCode::Char('`') {
                selected.to_lowercase()
            } else {
                selected
                    .chars()
                    .flat_map(|c| {
                        if c.is_lowercase() {
                            c.to_uppercase().collect::<Vec<_>>()
                        } else {
                            c.to_lowercase().collect()
                        }
                    })
                    .collect()
            };
            replace_selection(state, &text, true);
        },
        | _ => {},
    }
    None
}

pub fn status(state: &EditorState) -> String {
    let mode = match state.helix.mode {
        | Mode::Normal => "NORMAL",
        | Mode::Insert => "INSERT",
        | Mode::Select => "SELECT",
    };
    match state.helix.prefix {
        | Some(Prefix::Command) => format!(":{}", state.helix.prompt),
        | Some(Prefix::Search) => format!(
            "{}{}{}",
            if state.search_backward { "?" } else { "/" },
            state.helix.prompt,
            if state.helix.prompt_error { " [invalid regex]" } else { "" }
        ),
        | Some(Prefix::Goto) => format!("-- HELIX {mode} g --"),
        | Some(Prefix::Space) => format!("-- HELIX {mode} Space --"),
        | Some(
            Prefix::Find {
                ..
            }
            | Prefix::Replace,
        ) => format!("-- HELIX {mode} … --"),
        | None => format!("-- HELIX {mode} --{}", state.helix.count.map_or(String::new(), |n| format!(" {n}"))),
    }
}

pub fn help(state: &EditorState, language: Language) -> &'static str {
    match state.helix.prefix {
        | Some(Prefix::Command) => language.text(
            ":w:저장 | :q:종료 | :wq:저장 후 종료 | :bc:달력 | Esc:취소",
            ":w:Save | :q:Quit | :wq:Save & quit | :bc:Calendar | Esc:Cancel",
        ),
        | Some(Prefix::Search) => language.text("정규식 검색 | Enter:확정 | Esc:복원", "Regex search | Enter:Accept | Esc:Restore"),
        | Some(Prefix::Goto) => language.text(
            "g:문서 시작 | e:마지막 줄 | h/l:줄 시작/끝 | s:첫 글자",
            "g:File start | e:Last line | h/l:Line start/end | s:First nonblank",
        ),
        | Some(Prefix::Space) => language.text("f:일기 선택(달력) | Esc:취소", "f:Diary picker (calendar) | Esc:Cancel"),
        | Some(
            Prefix::Find {
                ..
            }
            | Prefix::Replace,
        ) => language.text("문자 입력 | Esc:취소", "Type a character | Esc:Cancel"),
        | None if state.helix.mode == Mode::Insert =>
            language.text("입력 모드 | Esc:Normal | C-s:실행취소 구분", "Insert mode | Esc:Normal | C-s:Undo checkpoint"),
        | None => language.text(
            "h/j/k/l:이동 | w/b/e:단어 | v:선택 | i/a:입력 | d/c/y/p:편집 | /:검색 | ::명령",
            "h/j/k/l:Move | w/b/e:Word | v:Select | i/a:Insert | d/c/y/p:Edit | /:Search | ::Command",
        ),
    }
}
