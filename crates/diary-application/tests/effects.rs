use chrono::NaiveDate;
use diary_application::{Command,
                        DiaryEntry,
                        EntryFormat,
                        Model,
                        Msg,
                        Screen,
                        app::handle_key,
                        input::{KeyCode,
                                KeyEvent,
                                KeyModifiers},
                        settings::Keybindings,
                        update};

fn date() -> NaiveDate { NaiveDate::from_ymd_opt(2026, 10, 19).unwrap() }
fn model() -> Model { Model::new(date(), Default::default()) }

#[test]
fn startup_and_calendar_preview_depend_only_on_supplied_values() {
    let mut model = model();
    assert_eq!(model.today, date());
    assert_eq!(model.calendar_state.selected_date, date());
    assert_eq!(model.editor_state.date, date());
    let command = update(&mut model, Msg::CalendarMoveRight);
    let selected = model.calendar_state.selected_date;
    assert!(matches!(command, Some(Command::LoadPreview(day)) if day == selected));
    let entry = DiaryEntry {
        content: "* Today".into(),
        format: EntryFormat::Org,
    };
    update(&mut model, Msg::PreviewLoaded(date(), Some(entry.clone())));
    assert!(model.preview.is_none());
    update(&mut model, Msg::PreviewLoaded(selected, Some(entry.clone())));
    assert_eq!(model.preview, Some((selected, entry)));
    update(&mut model, Msg::PreviewLoaded(selected, None));
    assert!(model.preview.is_none());
}

#[test]
fn failed_quit_save_preserves_text_and_cancels_the_pending_exit() {
    let mut model = model();
    model.screen = Screen::Editor;
    model.editor_state.insert_char('한');
    assert!(matches!(update(&mut model, Msg::Quit), Some(Command::SaveDiary(day, text)) if day == date() && text == "한"));
    assert!(!model.should_quit);
    assert!(model.editor_state.is_modified);
    update(&mut model, Msg::SaveDiaryFailed("disk full".into()));
    assert_eq!(model.editor_state.get_content(), "한");
    assert_eq!(model.screen, Screen::Editor);
    update(&mut model, Msg::DismissError);
    update(&mut model, Msg::SaveDiarySuccess(date()));
    assert!(!model.should_quit);
    assert!(model.diary_entries.entries.contains(&date()));
    update(&mut model, Msg::Quit);
    assert!(model.should_quit);
}

#[test]
fn calendar_return_waits_for_save_and_requests_a_fresh_preview() {
    let mut model = model();
    model.screen = Screen::Editor;
    model.editor_state.insert_char('a');
    let command = update(&mut model, Msg::EditorCtrlXBack);
    assert!(matches!(command, Some(Command::SaveDiary(_, _))));
    assert_eq!(model.screen, Screen::Editor);
    let unrelated = date().succ_opt().unwrap();
    update(&mut model, Msg::SaveDiarySuccess(unrelated));
    assert!(model.editor_state.is_modified);
    assert_eq!(model.screen, Screen::Editor);
    let command = update(&mut model, Msg::SaveDiarySuccess(date()));
    assert!(!model.editor_state.is_modified);
    assert_eq!(model.screen, Screen::Calendar);
    assert!(matches!(command, Some(Command::LoadPreview(day)) if day == date()));
}

#[test]
fn loaded_format_is_explicit_and_not_inferred_by_the_view() {
    let mut model = model();
    model.screen = Screen::Editor;
    update(&mut model, Msg::LoadDiarySuccess(date(), "# legacy".into(), EntryFormat::Markdown));
    assert_eq!(model.editor_state.format, EntryFormat::Markdown);
    assert_eq!(model.editor_state.get_content(), "# legacy");
}

#[test]
fn helix_write_quit_requests_a_save_even_when_the_document_is_clean() {
    let mut model = model();
    model.screen = Screen::Editor;
    model.settings.keybindings = Keybindings::Helix;
    for ch in ":wq".chars() {
        let message = handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE), &model).unwrap();
        assert!(update(&mut model, message).is_none());
    }
    let message = handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &model).unwrap();
    assert!(matches!(update(&mut model, message), Some(Command::SaveDiary(_, _))));
    assert!(!model.should_quit);
    update(&mut model, Msg::SaveDiarySuccess(date()));
    assert!(model.should_quit);
}
