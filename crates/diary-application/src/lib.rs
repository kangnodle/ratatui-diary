pub mod app;
pub mod calendar;
mod command;
pub mod editor;
pub mod input;
pub mod message;
pub mod search;

pub use app::{Command,
              Model,
              Screen,
              update::update};
pub use diary_domain::{DiaryEntry,
                       EntryFormat,
                       i18n,
                       settings};
pub use message::Msg;

/// The next application state and the effect it requests, if any.
pub struct Transition {
    pub model: Model,
    pub command: Option<Command>,
}

pub fn transition(mut model: Model, message: Msg) -> Transition {
    let command = update(&mut model, message);
    Transition {
        model,
        command,
    }
}

pub fn initialize(today: chrono::NaiveDate, entries: std::collections::HashSet<chrono::NaiveDate>, settings: Result<settings::Settings, String>) -> Transition {
    let mut model = Model::new(today, entries);
    app::initialize_settings(&mut model, settings);
    Transition {
        model,
        command: Some(Command::LoadPreview(today)),
    }
}

pub mod model {
    pub use crate::{app::{DiaryIndex,
                          Model,
                          Screen},
                    calendar::state::CalendarState,
                    editor::state::{EditorState,
                                    EditorSubMode,
                                    Selection}};
}

pub mod update {
    pub use crate::app::update::{Command,
                                 update};
}
