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
        let mut model = Model::new(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(), Default::default());
        model.settings.language = diary_application::settings::Language::English;
        model.settings.keybindings = keybindings;
        let msg = app::handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), &model).unwrap();
        app::update(&mut model, msg);

        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| app::view(frame, &model)).unwrap();
        let buffer = terminal.backend().buffer();
        let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("2026-01"));
        assert!(text.contains("Diary: 2026-01-01"));
        assert!(text.contains("PgUp/Dn:Previous/Next month"));
        assert!(text.contains("C-PgUp/Dn:Previous/Next year"));
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
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    storage.save(date, "* Cached preview").unwrap();
    let mut model = Model::new(date, storage.scan_entries().unwrap());
    let mut runtime = Runtime::new(storage, None);
    runtime.execute(Command::LoadPreview(date), &mut model);
    std::fs::remove_file(directory.path().join(format!("entries/{date}.org"))).unwrap();

    let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
    terminal.draw(|frame| app::view(frame, &model)).unwrap();
    let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("◉ Cached preview"));
}

#[test]
fn terminal_adapter_preserves_modifiers_and_repeat_but_filters_release() {
    let modifiers = KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::SUPER;
    let key = input::translate(KeyEvent::new_with_kind(KeyCode::Char('한'), modifiers, KeyEventKind::Repeat)).unwrap();
    assert_eq!(key.code, LogicalCode::Char('한'));
    assert_eq!(
        key.modifiers,
        LogicalModifiers::CONTROL | LogicalModifiers::ALT | LogicalModifiers::SHIFT | LogicalModifiers::OTHER
    );
    assert!(input::translate(KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release)).is_none());
    assert_eq!(
        input::translate(KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE)).unwrap().code,
        LogicalCode::Unknown
    );
}

#[test]
fn long_diary_lines_do_not_overflow_and_the_cursor_remains_visible() {
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    let mut model = Model::new(date, Default::default());
    model.screen = diary_application::Screen::Editor;
    model.editor_state.load_content(&format!("{}END", "한".repeat(40_000)));
    model.editor_state.cursor_col = 40_003;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| app::view(frame, &model)).unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0 .. 40).map(|x| buffer[(x, 1)].symbol()).collect();
    assert!(text.contains("END"));
}

#[test]
fn many_diary_lines_do_not_wrap_vertical_cursor_coordinates() {
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    let mut model = Model::new(date, Default::default());
    model.screen = diary_application::Screen::Editor;
    model.editor_state.load_content(&format!("{}END", "\n".repeat(70_000)));
    model.editor_state.cursor_line = 70_000;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| app::view(frame, &model)).unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0 .. 40).map(|x| buffer[(x, 20)].symbol()).collect();
    assert!(text.contains("END"));
}

#[test]
fn terminal_control_characters_are_not_emitted_and_source_contents_are_preserved() {
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    let mut model = Model::new(date, Default::default());
    let content = "* diary\x1b]52;c;c2VjcmV0\x07\n\x1b[2Jbody\u{009b}31m";
    model.editor_state.load_content(content);
    model.error_message = Some(content.to_string());
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    for screen in [diary_application::Screen::Calendar, diary_application::Screen::Editor] {
        model.screen = screen;
        for popup in [false, true] {
            model.show_error_popup = popup;
            terminal.draw(|frame| app::view(frame, &model)).unwrap();
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
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| app::view(frame, &model)).unwrap();
    let buffer = terminal.backend().buffer();
    let palette = ratatui_diary::theme::palette(model.settings.theme);
    assert_eq!(buffer[(0, 1)].bg, palette.search);
    assert_eq!(buffer[(2, 1)].bg, palette.search);
    assert_eq!(buffer[(4, 1)].bg, palette.current_search);
}
