//! Pure application state and state-transition entry points.

use crate::{calendar::state::CalendarState,
            editor::state::EditorState};
use chrono::{Datelike,
             NaiveDate};
use diary_domain::DiaryEntry;
use std::collections::HashSet;

pub mod keymap;
pub mod update;

pub use keymap::handle_key;
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
    pub(crate) pending_navigation: Option<Navigation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Calendar,
    Editor,
}

pub struct DiaryIndex {
    pub entries: HashSet<NaiveDate>,
}

impl Model {
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
            pending_navigation: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Navigation {
    Quit,
    Calendar,
}

pub(crate) fn navigate(model: &mut Model, navigation: Navigation, force_save: bool) -> Option<Command> {
    if model.editor_state.is_modified || force_save {
        model.pending_navigation = Some(navigation);
        Some(Command::SaveDiary(model.editor_state.date, model.editor_state.get_content()))
    } else {
        finish_navigation(model, navigation)
    }
}

fn finish_navigation(model: &mut Model, navigation: Navigation) -> Option<Command> {
    if model.show_error_popup {
        return None;
    }
    match navigation {
        | Navigation::Quit => {
            model.should_quit = true;
            None
        },
        | Navigation::Calendar => {
            model.screen = Screen::Calendar;
            Some(Command::LoadPreview(model.calendar_state.selected_date))
        },
    }
}
