pub use crate::command::Command;
use crate::{app::{Model,
                  Screen},
            calendar,
            editor,
            message::Msg};

pub fn update(model: &mut Model, msg: Msg) -> Option<Command> {
    match msg {
        | Msg::OpenSearch if model.screen == Screen::Calendar => crate::search::open(model),
        | Msg::SearchFinished(id, result) => {
            if model.screen == Screen::Search {
                model.search.finish(id, result);
            }
            None
        },
        | Msg::SearchWorkerStopped => {
            if model.screen == Screen::Search {
                model.search.worker_stopped();
            }
            None
        },
        | Msg::ToggleSettings | Msg::SettingsMove | Msg::SettingsMoveBack | Msg::SettingsChange(_) | Msg::SettingsSaveFailed(_) =>
            super::preferences::interact(model, msg),
        | Msg::DayObserved(date) => {
            model.today = date;
            None
        },
        | Msg::ViewportMeasured(height) => {
            model.editor_state.viewport_height = height;
            None
        },
        | Msg::Quit => {
            model.editor_state.submode = None;
            super::entry::navigate(model, super::entry::Navigation::Quit, false)
        },
        | Msg::EditorEnterCtrlXMode => {
            model.editor_state.submode = Some(editor::state::EditorSubMode::CtrlX);
            None
        },
        | Msg::EditorExitSubMode => {
            model.editor_state.submode = None;
            None
        },
        | Msg::EditorKeyboardQuit => {
            let state = &mut model.editor_state;
            if let Some((line, col)) = state.search_origin.take() {
                state.cursor_line = line;
                state.cursor_col = col;
            }
            state.submode = None;
            state.selection = None;
            state.search_matches.clear();
            state.search_pattern.clear();
            None
        },
        | Msg::EditorCtrlXBack if model.screen == Screen::Editor => {
            model.editor_state.submode = None;
            super::entry::navigate(model, super::entry::Navigation::Calendar, false)
        },
        | Msg::EditorUndo if model.editor_state.submode == Some(editor::state::EditorSubMode::CtrlX) => {
            model.editor_state.submode = None;
            model.editor_state.undo();
            None
        },

        | Msg::DismissError => {
            model.show_error_popup = false;
            model.error_message = None;
            None
        },

        | Msg::LoadDiarySuccess(date, content, format) => super::entry::opened(model, date, content, format),
        | Msg::LoadDiaryFailed(error) => super::entry::open_failed(model, error),
        | Msg::SaveDiarySuccess(date) => super::entry::saved(model, date),
        | Msg::SaveDiaryFailed(error) => super::entry::save_failed(model, error),
        | Msg::DeleteDiarySuccess(date) => super::entry::deleted(model, date),
        | Msg::RefreshIndex(entries) => {
            model.diary_entries.entries = entries;
            None
        },
        | Msg::PreviewLoaded(date, entry) => {
            if date == model.calendar_state.selected_date {
                model.preview = entry.map(|entry| (date, entry));
            }
            None
        },

        | Msg::HelixKey(key) => editor::helix::update(key, model),

        | other => match model.screen {
            | Screen::Calendar => calendar::update::update(model, other),
            | Screen::Editor => editor::update::update(model, other),
            | Screen::Search => crate::search::update(model, other),
        },
    }
}
