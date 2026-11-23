use crate::{Command,
            Model,
            Msg,
            Screen,
            input::{KeyCode,
                    KeyEvent,
                    KeyModifiers}};
use chrono::Datelike;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub date: chrono::NaiveDate,
    pub line: usize,
    pub col: usize,
    pub excerpt: String,
    pub literal: bool,
    pub anchor: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchOutput {
    pub results: Vec<SearchHit>,
    pub notice: Option<String>,
}

#[derive(Default)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchHit>,
    pub selected: usize,
    pub editing: bool,
    pub busy: bool,
    pub notice: Option<String>,
    pub revision: u64,
    pub(crate) target: Option<SearchHit>,
}

pub fn handle_key(key: KeyEvent, model: &Model) -> Option<Msg> {
    let control = key.modifiers == KeyModifiers::CONTROL;
    let helix = model.settings.keybindings == crate::settings::Keybindings::Helix;
    match key.code {
        | KeyCode::Esc => Some(Msg::SearchClose),
        | KeyCode::Char('g') if control => Some(Msg::SearchClose),
        | KeyCode::Char('c') if control && helix => Some(Msg::SearchClose),
        | KeyCode::Enter => Some(Msg::SearchSubmit),
        | KeyCode::Tab => Some(Msg::SearchEdit),
        | KeyCode::Up => Some(Msg::SearchMove(false)),
        | KeyCode::Down => Some(Msg::SearchMove(true)),
        | KeyCode::Char('p') if control => Some(Msg::SearchMove(false)),
        | KeyCode::Char('n') if control => Some(Msg::SearchMove(true)),
        | KeyCode::Char('k') if helix && !model.search.editing => Some(Msg::SearchMove(false)),
        | KeyCode::Char('j') if helix && !model.search.editing => Some(Msg::SearchMove(true)),
        | KeyCode::Backspace => Some(Msg::SearchBackspace),
        | KeyCode::Char(ch) if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT => Some(Msg::SearchChar(ch)),
        | _ => None,
    }
}

fn invalidate(state: &mut SearchState) {
    state.revision += 1;
    state.busy = false;
    state.results.clear();
    state.selected = 0;
    state.notice = None;
    state.editing = true;
}

pub fn update(model: &mut Model, msg: Msg) -> Option<Command> {
    let state = &mut model.search;
    match msg {
        | Msg::SearchChar(ch) => {
            state.query.push(ch);
            invalidate(state);
        },
        | Msg::SearchBackspace => {
            state.query.pop();
            invalidate(state);
        },
        | Msg::SearchEdit => state.editing = !state.editing,
        | Msg::SearchMove(forward) if !state.results.is_empty() => {
            state.editing = false;
            state.selected = if forward {
                (state.selected + 1).min(state.results.len() - 1)
            } else {
                state.selected.saturating_sub(1)
            };
        },
        | Msg::SearchSubmit if !state.editing && !state.busy && !state.results.is_empty() => {
            let hit = &state.results[state.selected];
            state.target = Some(hit.clone());
            model.calendar_state.selected_date = hit.date;
            model.calendar_state.current_year = hit.date.year();
            model.calendar_state.current_month = hit.date.month();
            model.screen = Screen::Editor;
            return Some(Command::LoadDiary(hit.date));
        },
        | Msg::SearchSubmit if !state.busy && !state.query.trim().is_empty() => {
            invalidate(state);
            state.editing = false;
            state.busy = true;
            return Some(Command::Search(state.revision, state.query.trim().to_string()));
        },
        | Msg::SearchClose => {
            state.revision += 1;
            state.busy = false;
            model.screen = Screen::Calendar;
            return Some(Command::LoadPreview(model.calendar_state.selected_date));
        },
        | _ => {},
    }
    None
}

pub(crate) fn locate(hit: &SearchHit, content: &str) -> Option<(usize, usize)> {
    if hit.anchor.is_empty() {
        return None;
    }
    let content = content.replace("\r\n", "\n");
    let anchor = hit.anchor.replace("\r\n", "\n");
    content
        .match_indices(&anchor)
        .map(|(byte, _)| {
            let prefix = &content[.. byte];
            (
                prefix.bytes().filter(|&b| b == b'\n').count(),
                prefix.rsplit('\n').next().unwrap_or("").chars().count(),
            )
        })
        .min_by_key(|&(line, col)| (line.abs_diff(hit.line), col.abs_diff(hit.col)))
}
