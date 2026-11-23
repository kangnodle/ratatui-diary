mod common;
use crate::common::{TestApp,
                    execute_command};
use chrono::{Datelike,
             NaiveDate};
use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyEventKind,
                       KeyModifiers};
use ratatui::{Terminal,
              backend::TestBackend};
use ratatui_diary::{Msg,
                    app::{Command,
                          handle_key},
                    model::Screen,
                    storage::Storage,
                    update,
                    view};
use std::collections::HashSet;
use tempfile::TempDir;

#[test]
fn test_application_startup() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = match storage.scan_entries() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not list the diary entries: {error:?}"),
    };

    let before = chrono::Local::now().date_naive();
    let model = TestApp::new(entries, storage);
    let after = chrono::Local::now().date_naive();

    assert_eq!(model.screen, Screen::Calendar);
    let selected = model.calendar_state.selected_date;
    assert!((before ..= after).contains(&selected));
    assert_eq!(model.calendar_state.current_year, selected.year());
    assert_eq!(model.calendar_state.current_month, selected.month());
    assert_eq!(model.editor_state.date, selected);
    assert!(!model.show_error_popup);
    assert!(model.error_message.is_none());
}

#[test]
fn test_opening_missing_diary_starts_with_empty_content() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    let date = match NaiveDate::from_ymd_opt(2026, 7, 1) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    model.screen = Screen::Editor;

    execute_command(Command::LoadDiary(date), &mut model);

    assert_eq!(model.editor_state.date, date);
    assert_eq!(model.editor_state.content, vec![String::new()]);
    assert!(!model.show_error_popup);
    assert!(model.error_message.is_none());
}

#[test]
fn test_key_release_does_not_insert_character_twice() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    model.screen = Screen::Editor;

    let press = KeyEvent::new_with_kind(KeyCode::Char('#'), KeyModifiers::NONE, KeyEventKind::Press);
    let release = KeyEvent::new_with_kind(KeyCode::Char('#'), KeyModifiers::NONE, KeyEventKind::Release);

    if let Some(msg) = handle_key(press, &model) {
        update::update(&mut model, msg);
    }
    if let Some(msg) = handle_key(release, &model) {
        update::update(&mut model, msg);
    }

    assert_eq!(model.editor_state.content, vec!["#".to_string()]);
}

#[test]
fn test_full_diary_workflow() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = match storage.scan_entries() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not list the diary entries: {error:?}"),
    };
    let mut model = TestApp::new(entries, storage);

    let date = match NaiveDate::from_ymd_opt(2026, 2, 15) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    model.calendar_state.selected_date = date;
    let cmd = update::update(&mut model, Msg::CalendarSelectDate);

    assert!(cmd.is_some());
    assert_eq!(model.screen, Screen::Editor);
    update::update(&mut model, Msg::LoadDiarySuccess(date, "test".to_string(), diary_domain::EntryFormat::Org));
    assert_eq!(model.editor_state.date, date);

    assert_eq!(model.editor_state.content, vec!["test".to_string()]);

    update::update(&mut model, Msg::EditorInsertChar('H'));
    update::update(&mut model, Msg::EditorInsertChar('i'));

    assert!(model.editor_state.is_modified);

    update::update(&mut model, Msg::EditorEnterCtrlXMode);
    let save_cmd = update::update(&mut model, Msg::EditorCtrlXSave);

    assert!(save_cmd.is_some());

    let saved_date = model.editor_state.date;
    update::update(&mut model, Msg::SaveDiarySuccess(saved_date));

    assert!(!model.editor_state.is_modified);
}

#[test]
fn test_error_handling_workflow() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);

    update::update(&mut model, Msg::LoadDiaryFailed("파일을 찾을 수 없음".to_string()));

    assert!(model.show_error_popup);
    assert_eq!(model.error_message, Some("로드 실패: 파일을 찾을 수 없음".to_string()));

    update::update(&mut model, Msg::DismissError);

    assert!(!model.show_error_popup);
    assert!(model.error_message.is_none());
}

#[test]
fn test_save_error_handling() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);

    update::update(&mut model, Msg::SaveDiaryFailed("쓰기 권한 없음".to_string()));

    assert!(model.show_error_popup);
    assert_eq!(model.error_message, Some("저장 실패: 쓰기 권한 없음".to_string()));

    update::update(&mut model, Msg::DismissError);

    assert!(!model.show_error_popup);
}

#[test]
fn test_render_all_states() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    let backend = TestBackend::new(80, 24);
    let mut terminal = match Terminal::new(backend) {
        | Ok(value) => value,
        | Err(never) => match never {},
    };

    match terminal.draw(|f| view::view(f, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };

    model.screen = Screen::Editor;

    match terminal.draw(|f| view::view(f, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };

    model.show_error_popup = true;
    model.error_message = Some("테스트 에러".to_string());

    match terminal.draw(|f| view::view(f, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };

    model.show_error_popup = false;

    match terminal.draw(|f| view::view(f, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };
}

#[test]
fn test_keyboard_navigation_simulation() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    let original_date = model.calendar_state.selected_date;

    update::update(&mut model, Msg::CalendarMoveRight);
    let first_move = model.calendar_state.selected_date;

    assert_ne!(first_move, original_date);

    update::update(&mut model, Msg::CalendarMoveRight);
    let second_move = model.calendar_state.selected_date;

    assert_ne!(second_move, first_move);

    update::update(&mut model, Msg::CalendarMoveLeft);
    let after_left = model.calendar_state.selected_date;

    assert_ne!(after_left, second_move);
    assert_eq!(after_left, first_move);
}

#[test]
fn test_editor_navigation() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    model.screen = Screen::Editor;
    model.editor_state.content = vec!["Hello World".to_string()];

    let original_col = model.editor_state.cursor_col;
    update::update(&mut model, Msg::EditorMoveRight);

    assert_eq!(model.editor_state.cursor_col, original_col + 1);

    update::update(&mut model, Msg::EditorMoveLeft);

    assert_eq!(model.editor_state.cursor_col, original_col);
}

#[test]
fn test_quit_message() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    let initial_screen = model.screen;

    let _ = update::update(&mut model, Msg::Quit);

    assert_eq!(model.screen, initial_screen);
}

#[test]
fn test_complex_editor_workflow() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    model.screen = Screen::Editor;

    for ch in "테스트".chars() {
        update::update(&mut model, Msg::EditorInsertChar(ch));
    }

    assert!(!model.editor_state.content.is_empty());
    assert!(model.editor_state.is_modified);

    update::update(&mut model, Msg::EditorNewLine);

    let line_count = model.editor_state.content.len();
    assert!(line_count >= 1);
}

#[test]
fn test_calendar_month_navigation() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    let initial_month = model.calendar_state.current_month;

    update::update(&mut model, Msg::CalendarNextMonth);

    let new_month = model.calendar_state.current_month;
    if initial_month == 12 {
        assert_eq!(new_month, 1);
    } else {
        assert_eq!(new_month, initial_month + 1);
    }
}

#[test]
fn test_backspace_handling() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    model.screen = Screen::Editor;
    model.editor_state.content = vec!["Hello".to_string()];
    model.editor_state.cursor_col = 5;

    update::update(&mut model, Msg::EditorBackspace);

    assert!(model.editor_state.is_modified);
}

#[test]
fn test_screen_transition() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    assert_eq!(model.screen, Screen::Calendar);

    model.calendar_state.selected_date = match NaiveDate::from_ymd_opt(2026, 2, 15) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    update::update(&mut model, Msg::CalendarSelectDate);

    assert_eq!(model.screen, Screen::Editor);

    update::update(&mut model, Msg::EditorEnterCtrlXMode);
    let command = match update::update(&mut model, Msg::EditorCtrlXBack) {
        | Some(value) => value,
        | None => panic!("Expected the application update to produce a command"),
    };
    execute_command(command, &mut model);

    assert_eq!(model.screen, Screen::Calendar);
}

#[test]
fn test_undo_redo() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    model.screen = Screen::Editor;
    model.editor_state.content = vec!["AB".to_string()];
    model.editor_state.cursor_col = 0;
    model.editor_state.save_snapshot();

    let before_delete = model.editor_state.content.clone();

    update::update(&mut model, Msg::EditorKillLine);

    let after_kill = model.editor_state.content.clone();
    assert_ne!(before_delete, after_kill);

    update::update(&mut model, Msg::EditorUndo);

    let after_undo = model.editor_state.content.clone();
    assert_eq!(before_delete, after_undo);

    update::update(&mut model, Msg::EditorRedo);

    let after_redo = model.editor_state.content.clone();
    assert_eq!(after_kill, after_redo);
}

#[test]
fn test_calendar_year_navigation() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut model = TestApp::new(HashSet::new(), storage);
    let initial_year = model.calendar_state.current_year;

    update::update(&mut model, Msg::CalendarNextYear);

    assert_eq!(model.calendar_state.current_year, initial_year + 1);

    update::update(&mut model, Msg::CalendarPrevYear);

    assert_eq!(model.calendar_state.current_year, initial_year);
}

#[test]
fn test_delete_diary_success() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 2, 15) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };

    let _ = storage.save(date, "테스트 내용");
    let entries = match storage.scan_entries() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not list the diary entries: {error:?}"),
    };

    let mut model = TestApp::new(entries, storage);

    assert!(model.diary_entries.entries.contains(&date));

    update::update(&mut model, Msg::DeleteDiarySuccess(date));

    assert!(!model.diary_entries.entries.contains(&date));
    assert_eq!(model.screen, Screen::Calendar);
}
