use crate::{Command,
            Model,
            Msg,
            Screen,
            input::{KeyCode,
                    KeyEvent,
                    KeyModifiers}};
pub use diary_domain::search::{SearchHit,
                               SearchOutput};

#[derive(Default)]
enum Request {
    #[default]
    Idle,
    Pending(u64),
}

#[derive(Default)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchHit>,
    pub selected: usize,
    pub editing: bool,
    request: Request,
    pub notice: Option<String>,
    revision: u64,
}

impl SearchState {
    pub fn is_busy(&self) -> bool { matches!(self.request, Request::Pending(_)) }

    pub fn pending_revision(&self) -> Option<u64> {
        match self.request {
            | Request::Pending(id) => Some(id),
            | Request::Idle => None,
        }
    }

    pub(crate) fn finish(&mut self, revision: u64, result: Result<SearchOutput, String>) {
        if self.pending_revision() != Some(revision) {
            return;
        }
        self.request = Request::Idle;
        match result {
            | Ok(output) => {
                self.results = output.results;
                self.notice = output.notice;
            },
            | Err(error) => self.notice = Some(error),
        }
    }

    pub(crate) fn worker_stopped(&mut self) {
        if let Some(revision) = self.pending_revision() {
            self.finish(revision, Err("검색 작업이 종료되었습니다 / Search worker stopped".into()));
        }
    }
}

pub(crate) fn open(model: &mut Model) -> Option<Command> {
    model.screen = Screen::Search;
    model.search.editing = true;
    model.editor_state.submode = None;
    model.editor_state.helix.prefix = None;
    None
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
    state.request = Request::Idle;
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
        | Msg::SearchSubmit if !state.editing && !state.is_busy() && !state.results.is_empty() => {
            let hit = state.results[state.selected].clone();
            return Some(crate::app::entry::open(model, hit.date, Some(hit)));
        },
        | Msg::SearchSubmit if !state.is_busy() && !state.query.trim().is_empty() => {
            invalidate(state);
            state.editing = false;
            state.request = Request::Pending(state.revision);
            return Some(Command::Search(state.revision, state.query.trim().to_string()));
        },
        | Msg::SearchClose => {
            state.revision += 1;
            state.request = Request::Idle;
            return Some(crate::app::entry::return_to_calendar(model));
        },
        | _ => {},
    }
    None
}
