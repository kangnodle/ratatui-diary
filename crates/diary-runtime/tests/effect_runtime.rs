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

#[test]
fn background_search_completes_through_poll_without_blocking_execute() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let mut runtime = Runtime::new(storage, None);
    let mut model = Model::new(chrono::NaiveDate::default(), Default::default());
    update(&mut model, Msg::OpenSearch);
    update(&mut model, Msg::SearchChar('가'));
    let command = match update(&mut model, Msg::SearchSubmit) {
        | Some(value) => value,
        | None => panic!("Expected the application update to produce a command"),
    };
    runtime.execute(command, &mut model);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while model.search.is_busy() && std::time::Instant::now() < deadline {
        runtime.poll(&mut model);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!model.search.is_busy());
    assert!(model.search.notice.is_none());
    assert!(model.search.results.is_empty());
}

fn fixture() -> (TempDir, Runtime, Model) {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let model = Model::new(
        match NaiveDate::from_ymd_opt(2026, 10, 19) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        },
        Default::default(),
    );
    (directory, Runtime::new(storage, None), model)
}

#[test]
fn preview_and_editor_receive_the_same_legacy_format() {
    let (directory, mut runtime, mut model) = fixture();
    let date = model.calendar_state.selected_date;
    match std::fs::write(directory.path().join(format!("entries/{date}.md")), "# legacy") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    runtime.execute(Command::LoadPreview(date), &mut model);
    assert_eq!(
        (match model.preview.as_ref() {
            | Some(value) => value,
            | None => panic!("Expected preview to be present"),
        })
        .1
        .format,
        EntryFormat::Markdown
    );
    let command = match update(&mut model, Msg::CalendarSelectDate) {
        | Some(value) => value,
        | None => panic!("Expected the application update to produce a command"),
    };
    runtime.execute(command, &mut model);
    assert_eq!(model.editor_state.format, EntryFormat::Markdown);
    assert_eq!(model.editor_state.get_content(), "# legacy");
    model.editor_state.insert_char('a');
    let command = match update(&mut model, Msg::EditorCtrlXBack) {
        | Some(value) => value,
        | None => panic!("Expected the application update to produce a command"),
    };
    runtime.execute(command, &mut model);
    assert_eq!(model.screen, Screen::Calendar);
    assert_eq!(
        (match model.preview.as_ref() {
            | Some(value) => value,
            | None => panic!("Expected preview to be present"),
        })
        .1
        .content,
        "a# legacy"
    );
    assert!(!directory.path().join(format!("entries/{date}.org")).exists());
}

#[test]
fn failed_save_keeps_the_editor_open_and_retry_can_exit() {
    let (directory, mut runtime, mut model) = fixture();
    model.screen = Screen::Editor;
    model.editor_state.insert_char('한');
    let path = directory.path().join(format!("entries/{}.org", model.editor_state.date));
    match std::fs::create_dir(&path) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the directory: {error:?}"),
    };
    model = runtime.advance(model, Msg::Quit);
    assert!(!model.should_quit);
    assert!(model.show_error_popup);
    assert!(model.editor_state.is_modified);
    assert_eq!(model.editor_state.get_content(), "한");
    match std::fs::remove_dir(&path) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the directory: {error:?}"),
    };
    let model = runtime.advance(model, Msg::DismissError);
    let model = runtime.advance(model, Msg::Quit);
    assert!(model.should_quit);
    assert_eq!(
        match std::fs::read_to_string(path) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "한"
    );
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
    match std::fs::create_dir(directory.path().join(format!("entries/{date}.org"))) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the directory: {error:?}"),
    };
    runtime.execute(Command::LoadDiary(date), &mut model);
    assert!(model.show_error_popup);
}

#[test]
fn failed_open_cannot_relabel_the_previous_buffer_and_overwrite_another_diary() {
    let (directory, mut runtime, mut model) = fixture();
    let previous = model.editor_state.date;
    model.editor_state.load_content("* Previous diary");
    let selected = match previous.succ_opt() {
        | Some(value) => value,
        | None => panic!("Expected a next day for the fixture date"),
    };
    let path = directory.path().join(format!("entries/{selected}.org"));
    match std::fs::write(&path, [0xff]) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    model.calendar_state.selected_date = selected;
    let command = match update(&mut model, Msg::CalendarSelectDate) {
        | Some(value) => value,
        | None => panic!("Expected the application update to produce a command"),
    };
    runtime.execute(command, &mut model);
    update(&mut model, Msg::DismissError);
    assert_eq!(model.screen, Screen::Calendar);
    assert_eq!(model.editor_state.date, previous);
    assert_eq!(model.editor_state.get_content(), "* Previous diary");
    update(&mut model, Msg::EditorEnterCtrlXMode);
    assert!(update(&mut model, Msg::EditorCtrlXSave).is_none());
    assert_eq!(
        match std::fs::read(path) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the file: {error:?}"),
        },
        [0xff]
    );
}
