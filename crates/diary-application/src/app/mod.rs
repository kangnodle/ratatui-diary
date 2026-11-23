use crate::{calendar::state::CalendarState,
            editor::state::EditorState};
use chrono::{Datelike,
             NaiveDate};
use diary_domain::DiaryEntry;
use std::collections::HashSet;

pub(crate) mod entry;
pub mod keymap;
mod preferences;
pub mod update;

pub use keymap::handle_key;
pub(super) use preferences::initialize as initialize_settings;
pub use update::{Command,
                 update};

pub struct Model {
    pub screen: Screen,
    pub settings: crate::settings::Settings,
    pub show_settings: bool,
    pub should_quit: bool,
    pub settings_row: usize,
    pub calendar_state: CalendarState,
    pub editor_state: EditorState,
    pub diary_entries: DiaryIndex,
    pub error_message: Option<String>,
    pub show_error_popup: bool,
    pub today: NaiveDate,
    pub preview: Option<(NaiveDate, DiaryEntry)>,
    pub search: crate::search::SearchState,
    entry_operation: entry::Operation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Calendar,
    Editor,
    Search,
}

pub struct DiaryIndex {
    pub entries: HashSet<NaiveDate>,
}

impl Model {
    fn report_error(&mut self, message: crate::i18n::Text, error: String) {
        self.error_message = Some(format!("{}: {}", self.settings.language.tr(message), error));
        self.show_error_popup = true;
    }

    pub fn new(today: NaiveDate, entries: HashSet<NaiveDate>) -> Self {
        Self {
            screen: Screen::Calendar,
            settings: Default::default(),
            show_settings: false,
            should_quit: false,
            settings_row: 0,
            calendar_state: CalendarState {
                selected_date: today,
                ..CalendarState::new(today.year(), today.month())
            },
            editor_state: EditorState::new(today),
            diary_entries: DiaryIndex {
                entries,
            },
            error_message: None,
            show_error_popup: false,
            today,
            preview: None,
            search: Default::default(),
            entry_operation: entry::Operation::Idle,
        }
    }
}
