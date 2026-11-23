pub mod app;
pub mod calendar;
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
