use chrono::NaiveDate;
use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyEventKind,
                       KeyModifiers};
use diary_application::{Command,
                        Model,
                        input::{KeyCode as LogicalCode,
                                KeyModifiers as LogicalModifiers}};
use ratatui::{Terminal,
              backend::TestBackend};
use ratatui_diary::{Runtime,
                    app,
                    input,
                    storage::Storage};
use tempfile::TempDir;

#[test]
fn calendar_header_selection_and_navigation_help_follow_the_cursor() {
    for keybindings in [diary_application::settings::Keybindings::Emacs, diary_application::settings::Keybindings::Helix] {
        let mut model = Model::new(
            match NaiveDate::from_ymd_opt(2025, 12, 31) {
                | Some(value) => value,
                | None => panic!("Invalid date in test fixture"),
            },
            Default::default(),
        );
        model.settings.language = diary_application::settings::Language::English;
        model.settings.keybindings = keybindings;
        let msg = match app::handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), &model) {
            | Some(value) => value,
            | None => panic!("Expected a message for the test key"),
        };
        app::update(&mut model, msg);

        let mut terminal = match Terminal::new(TestBackend::new(100, 24)) {
            | Ok(value) => value,
            | Err(never) => match never {},
        };
        match terminal.draw(|frame| app::view(frame, &model)) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };
        let buffer = terminal.backend().buffer();
        let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("2026-01"));
        assert!(text.contains("Diary: 2026-01-01"));
        let rows: Vec<String> = (0 .. 24).map(|y| (0 .. 50).map(|x| buffer[(x, y)].symbol()).collect()).collect();
        assert!(rows.iter().any(|row| row.contains("PgUp/Dn") && row.contains("Previous/Next month")));
        assert!(rows.iter().any(|row| row.contains("C-PgUp/Dn") && row.contains("Previous/Next year")));
        let palette = ratatui_diary::theme::palette(model.settings.theme);
        assert!(
            buffer
                .content
                .iter()
                .any(|cell| cell.symbol() == "1" && cell.bg == palette.selected_bg && cell.fg == palette.selected_fg)
        );
    }
}

#[test]
fn rendering_uses_the_loaded_snapshot_even_after_the_file_is_removed() {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    match storage.save(date, "* Cached preview") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not save the diary: {error:?}"),
    };
    let mut model = Model::new(
        date,
        match storage.scan_entries() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not list the diary entries: {error:?}"),
        },
    );
    let mut runtime = Runtime::new(storage, None);
    runtime.execute(Command::LoadPreview(date), &mut model);
    match std::fs::remove_file(directory.path().join(format!("entries/{date}.org"))) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the file: {error:?}"),
    };

    let mut terminal = match Terminal::new(TestBackend::new(160, 30)) {
        | Ok(value) => value,
        | Err(never) => match never {},
    };
    match terminal.draw(|frame| app::view(frame, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };
    let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("◉ Cached preview"));
}

#[test]
fn terminal_adapter_preserves_modifiers_and_repeat_but_filters_release() {
    let modifiers = KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::SUPER;
    let key = match input::translate(KeyEvent::new_with_kind(KeyCode::Char('한'), modifiers, KeyEventKind::Repeat)) {
        | Some(value) => value,
        | None => panic!("Expected a logical key for the terminal event"),
    };
    assert_eq!(key.code, LogicalCode::Char('한'));
    assert_eq!(
        key.modifiers,
        LogicalModifiers::CONTROL | LogicalModifiers::ALT | LogicalModifiers::SHIFT | LogicalModifiers::OTHER
    );
    assert!(input::translate(KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release)).is_none());
    assert_eq!(
        (match input::translate(KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE)) {
            | Some(value) => value,
            | None => panic!("Expected a logical key for the terminal event"),
        })
        .code,
        LogicalCode::Unknown
    );
}

#[test]
fn long_diary_lines_do_not_overflow_and_the_cursor_remains_visible() {
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let mut model = Model::new(date, Default::default());
    model.screen = diary_application::Screen::Editor;
    model.editor_state.load_content(&format!("{}END", "한".repeat(40_000)));
    model.editor_state.cursor_col = 40_003;
    let mut terminal = match Terminal::new(TestBackend::new(80, 24)) {
        | Ok(value) => value,
        | Err(never) => match never {},
    };
    match terminal.draw(|frame| app::view(frame, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };
    let buffer = terminal.backend().buffer();
    let text: String = (0 .. 40).map(|x| buffer[(x, 1)].symbol()).collect();
    assert!(text.contains("END"));
}

#[test]
fn many_diary_lines_do_not_wrap_vertical_cursor_coordinates() {
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let mut model = Model::new(date, Default::default());
    model.screen = diary_application::Screen::Editor;
    model.editor_state.load_content(&format!("{}END", "\n".repeat(70_000)));
    model.editor_state.cursor_line = 70_000;
    let mut terminal = match Terminal::new(TestBackend::new(80, 24)) {
        | Ok(value) => value,
        | Err(never) => match never {},
    };
    match terminal.draw(|frame| app::view(frame, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };
    let buffer = terminal.backend().buffer();
    let last_row = ratatui_diary::editor::view::viewport_height(&model, buffer.area) as u16;
    let text: String = (0 .. 40).map(|x| buffer[(x, last_row)].symbol()).collect();
    assert!(text.contains("END"));
}

#[test]
fn terminal_control_characters_are_not_emitted_and_source_contents_are_preserved() {
    let date = match NaiveDate::from_ymd_opt(2026, 10, 19) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let mut model = Model::new(date, Default::default());
    let content = "* diary\x1b]52;c;c2VjcmV0\x07\n\x1b[2Jbody\u{009b}31m";
    model.editor_state.load_content(content);
    model.error_message = Some(content.to_string());
    let mut terminal = match Terminal::new(TestBackend::new(100, 30)) {
        | Ok(value) => value,
        | Err(never) => match never {},
    };
    for screen in [diary_application::Screen::Calendar, diary_application::Screen::Editor] {
        model.screen = screen;
        for popup in [false, true] {
            model.show_error_popup = popup;
            match terminal.draw(|frame| app::view(frame, &model)) {
                | Ok(_) => (),
                | Err(never) => match never {},
            };
            assert!(terminal.backend().buffer().content.iter().all(|cell| !cell.symbol().contains(char::is_control)));
        }
    }
    assert_eq!(model.editor_state.get_content(), content);
}

#[test]
fn overlapping_unicode_highlights_keep_the_first_match_and_current_match_colors() {
    let mut model = Model::new(NaiveDate::default(), Default::default());
    model.screen = diary_application::Screen::Editor;
    model.editor_state.load_content("한한한");
    model.editor_state.search_pattern = "한한".into();
    model.editor_state.execute_search();
    model.editor_state.current_match_index = 1;
    model.editor_state.selection = None;
    let mut terminal = match Terminal::new(TestBackend::new(80, 24)) {
        | Ok(value) => value,
        | Err(never) => match never {},
    };
    match terminal.draw(|frame| app::view(frame, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };
    let buffer = terminal.backend().buffer();
    let palette = ratatui_diary::theme::palette(model.settings.theme);
    assert_eq!(buffer[(0, 1)].bg, palette.search);
    assert_eq!(buffer[(2, 1)].bg, palette.search);
    assert_eq!(buffer[(4, 1)].bg, palette.current_search);
}
