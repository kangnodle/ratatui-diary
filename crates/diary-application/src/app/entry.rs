//! Coordinates the calendar, editor and entry index during an entry operation.
use super::{Command,
            Model,
            Screen};
use crate::{DiaryEntry,
            EntryFormat,
            i18n::Text};
use chrono::NaiveDate;
use diary_domain::search::SearchHit;

#[derive(Default)]
pub(super) enum Operation {
    #[default]
    Idle,
    Opening {
        date: NaiveDate,
        hit: Option<SearchHit>,
    },
    SavingFor {
        date: NaiveDate,
        navigation: Navigation,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum Navigation {
    Quit,
    Calendar,
}

pub(crate) fn open(model: &mut Model, date: NaiveDate, hit: Option<SearchHit>) -> Command {
    model.calendar_state.focus(date);
    model.screen = Screen::Editor;
    model.entry_operation = Operation::Opening {
        date,
        hit,
    };
    Command::LoadDiary(date)
}

pub(super) fn opened(model: &mut Model, date: NaiveDate, content: String, format: EntryFormat) -> Option<Command> {
    if model.screen != Screen::Editor {
        return None;
    }
    let hit = match &mut model.entry_operation {
        | Operation::Opening {
            date: requested,
            hit,
        } if *requested == date => {
            let hit = hit.take();
            model.entry_operation = Operation::Idle;
            hit
        },
        | Operation::Idle => None,
        | _ => return None,
    };
    let position = match hit {
        | Some(hit) => match hit.locate_in(&content) {
            | Some(position) => Some(position),
            | None =>
                return open_failed(
                    model,
                    model
                        .settings
                        .language
                        .text(
                            "검색한 구절이 변경되었거나 삭제되었습니다. 다시 검색하세요.",
                            "The passage changed or was deleted. Please search again.",
                        )
                        .to_owned(),
                ),
        },
        | None => None,
    };

    model.editor_state.load_entry(
        date,
        DiaryEntry {
            content,
            format,
        },
        position,
        model.settings.keybindings,
    );
    None
}

pub(super) fn open_failed(model: &mut Model, error: String) -> Option<Command> {
    if matches!(model.entry_operation, Operation::Opening { .. }) {
        model.entry_operation = Operation::Idle;
    }
    model.screen = Screen::Calendar;
    model.report_error(Text::LoadFailed, error);
    None
}

pub(crate) fn save(editor: &crate::editor::state::EditorState) -> Command { Command::SaveDiary(editor.date, editor.get_content()) }

pub(crate) fn navigate(model: &mut Model, navigation: Navigation, force_save: bool) -> Option<Command> {
    if model.editor_state.is_modified || force_save {
        model.entry_operation = Operation::SavingFor {
            date: model.editor_state.date,
            navigation,
        };
        Some(save(&model.editor_state))
    } else {
        finish_navigation(model, navigation)
    }
}

pub(super) fn saved(model: &mut Model, date: NaiveDate) -> Option<Command> {
    model.diary_entries.entries.insert(date);
    if date == model.editor_state.date {
        model.editor_state.is_modified = false;
        if let Operation::SavingFor {
            date: requested,
            navigation,
        } = model.entry_operation
            && date == requested
        {
            model.entry_operation = Operation::Idle;
            return finish_navigation(model, navigation);
        }
    }
    None
}

pub(super) fn save_failed(model: &mut Model, error: String) -> Option<Command> {
    if matches!(model.entry_operation, Operation::SavingFor { .. }) {
        model.entry_operation = Operation::Idle;
    }
    model.report_error(Text::SaveFailed, error);
    None
}

pub(super) fn deleted(model: &mut Model, date: NaiveDate) -> Option<Command> {
    model.diary_entries.entries.remove(&date);
    Some(return_to_calendar(model))
}

pub(crate) fn return_to_calendar(model: &mut Model) -> Command {
    model.entry_operation = Operation::Idle;
    model.screen = Screen::Calendar;
    Command::LoadPreview(model.calendar_state.selected_date)
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
        | Navigation::Calendar => Some(return_to_calendar(model)),
    }
}
