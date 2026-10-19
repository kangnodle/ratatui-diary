use chrono::NaiveDate;
use diary_application::{Command,
                        EntryFormat,
                        Model,
                        Msg,
                        Screen,
                        update};
use diary_runtime::Runtime;
use diary_storage::Storage;
use tempfile::TempDir;

fn fixture() -> (TempDir, Runtime, Model) {
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let model = Model::new(NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(), Default::default());
    (directory, Runtime::new(storage, None), model)
}

#[test]
fn preview_and_editor_receive_the_same_legacy_format() {
    let (directory, mut runtime, mut model) = fixture();
    let date = model.calendar_state.selected_date;
    std::fs::write(directory.path().join(format!("entries/{date}.md")), "# legacy").unwrap();
    runtime.execute(Command::LoadPreview(date), &mut model);
    assert_eq!(model.preview.as_ref().unwrap().1.format, EntryFormat::Markdown);
    let command = update(&mut model, Msg::CalendarSelectDate).unwrap();
    runtime.execute(command, &mut model);
    assert_eq!(model.editor_state.format, EntryFormat::Markdown);
    assert_eq!(model.editor_state.get_content(), "# legacy");
    model.editor_state.insert_char('a');
    let command = update(&mut model, Msg::EditorCtrlXBack).unwrap();
    runtime.execute(command, &mut model);
    assert_eq!(model.screen, Screen::Calendar);
    assert_eq!(model.preview.as_ref().unwrap().1.content, "a# legacy");
    assert!(!directory.path().join(format!("entries/{date}.org")).exists());
}

#[test]
fn failed_save_keeps_the_editor_open_and_retry_can_exit() {
    let (directory, mut runtime, mut model) = fixture();
    model.screen = Screen::Editor;
    model.editor_state.insert_char('한');
    let path = directory.path().join(format!("entries/{}.org", model.editor_state.date));
    std::fs::create_dir(&path).unwrap();
    let command = update(&mut model, Msg::Quit).unwrap();
    runtime.execute(command, &mut model);
    assert!(!model.should_quit);
    assert!(model.show_error_popup);
    assert!(model.editor_state.is_modified);
    assert_eq!(model.editor_state.get_content(), "한");
    std::fs::remove_dir(&path).unwrap();
    update(&mut model, Msg::DismissError);
    let command = update(&mut model, Msg::Quit).unwrap();
    runtime.execute(command, &mut model);
    assert!(model.should_quit);
    assert_eq!(std::fs::read_to_string(path).unwrap(), "한");
}

#[test]
fn missing_diaries_are_empty_but_other_read_errors_are_visible() {
    let (directory, mut runtime, mut model) = fixture();
    let date = model.calendar_state.selected_date;
    runtime.execute(Command::LoadPreview(date), &mut model);
    assert!(model.preview.is_none());
    assert!(!model.show_error_popup);
    model.screen = Screen::Editor;
    runtime.execute(Command::LoadDiary(date), &mut model);
    assert_eq!(model.editor_state.get_content(), "");
    assert_eq!(model.editor_state.format, EntryFormat::Org);
    std::fs::create_dir(directory.path().join(format!("entries/{date}.org"))).unwrap();
    runtime.execute(Command::LoadDiary(date), &mut model);
    assert!(model.show_error_popup);
}
