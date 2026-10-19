use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyModifiers};
use ratatui::{Terminal,
              backend::TestBackend};
use ratatui_diary::{Model,
                    app,
                    model::{EditorSubMode,
                            Screen},
                    settings::Language,
                    storage::Storage};
use tempfile::TempDir;

fn model(content: &str) -> (TempDir, Model) {
    let dir = TempDir::new().unwrap();
    let mut model = Model::new(Default::default(), Storage::with_dir(dir.path()).unwrap());
    model.screen = Screen::Editor;
    model.editor_state.load_content(content);
    (dir, model)
}

fn key(model: &mut Model, code: KeyCode, modifiers: KeyModifiers) {
    if let Some(msg) = app::handle_key(KeyEvent::new(code, modifiers), model)
        && let Some(command) = app::update(model, msg)
    {
        app::execute_command(command, model).unwrap();
    }
}

fn ctrl(model: &mut Model, ch: char) { key(model, KeyCode::Char(ch), KeyModifiers::CONTROL); }
fn alt(model: &mut Model, ch: char) { key(model, KeyCode::Char(ch), KeyModifiers::ALT); }
fn type_text(model: &mut Model, text: &str) {
    for ch in text.chars() {
        key(model, KeyCode::Char(ch), KeyModifiers::NONE);
    }
}
fn cursor(model: &Model) -> (usize, usize) { (model.editor_state.cursor_line, model.editor_state.cursor_col) }

#[test]
fn shared_prefix_quits_from_calendar_and_cancels_without_inserting_keys() {
    let (_dir, mut model) = model("");
    model.screen = Screen::Calendar;
    ctrl(&mut model, 'x');
    assert_eq!(model.editor_state.submode, Some(EditorSubMode::CtrlX));
    ctrl(&mut model, 'g');
    assert_eq!(model.editor_state.submode, None);
    assert!(!model.should_quit);
    ctrl(&mut model, 'x');
    ctrl(&mut model, 'c');
    assert!(model.should_quit);
    assert_eq!(model.editor_state.get_content(), "");
}

#[test]
fn save_aliases_and_kill_buffer_save_to_org_and_return_to_calendar() {
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::NONE] {
        let (dir, mut model) = model("");
        type_text(&mut model, "* Diary");
        ctrl(&mut model, 'x');
        key(&mut model, KeyCode::Char('s'), modifiers);
        assert!(!model.editor_state.is_modified);
        assert_eq!(model.screen, Screen::Editor);
        let path = dir.path().join(format!("entries/{}.org", model.editor_state.date));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "* Diary");
        type_text(&mut model, " text");
        ctrl(&mut model, 'x');
        type_text(&mut model, "k");
        assert_eq!(model.screen, Screen::Calendar);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "* Diary text");
    }
}

#[test]
fn quit_and_calendar_return_keep_unsaved_text_when_saving_fails() {
    for quit in [false, true] {
        let (dir, mut model) = model("");
        let path = dir.path().join(format!("entries/{}.org", model.editor_state.date));
        std::fs::create_dir(&path).unwrap();
        type_text(&mut model, "한글 diary");
        ctrl(&mut model, 'x');
        key(
            &mut model,
            KeyCode::Char(if quit { 'c' } else { 'k' }),
            if quit { KeyModifiers::CONTROL } else { KeyModifiers::NONE },
        );
        assert!(!model.should_quit);
        assert_eq!(model.screen, Screen::Editor);
        assert!(model.show_error_popup);
        assert!(model.editor_state.is_modified);
        assert_eq!(model.editor_state.get_content(), "한글 diary");
        ctrl(&mut model, 'g');
        assert!(!model.show_error_popup);
        std::fs::remove_dir(&path).unwrap();
        ctrl(&mut model, 'x');
        key(
            &mut model,
            KeyCode::Char(if quit { 'c' } else { 'k' }),
            if quit { KeyModifiers::CONTROL } else { KeyModifiers::NONE },
        );
        assert_eq!(model.should_quit, quit);
        assert_eq!(model.screen, if quit { Screen::Editor } else { Screen::Calendar });
        assert_eq!(std::fs::read_to_string(path).unwrap(), "한글 diary");
    }
}

#[test]
fn undo_keys_undo_typing_and_redo_restores_it() {
    for (undo, modifiers) in [
        ('/', KeyModifiers::CONTROL),
        ('_', KeyModifiers::CONTROL),
        ('z', KeyModifiers::CONTROL),
        ('_', KeyModifiers::CONTROL | KeyModifiers::SHIFT),
    ] {
        let (_dir, mut model) = model("");
        type_text(&mut model, "ab");
        key(&mut model, KeyCode::Char(undo), modifiers);
        assert_eq!(model.editor_state.get_content(), "a");
        key(&mut model, KeyCode::Char('z'), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        assert_eq!(model.editor_state.get_content(), "ab");
        key(&mut model, KeyCode::Char('_'), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        ctrl(&mut model, '?');
        assert_eq!(model.editor_state.get_content(), "ab");
        ctrl(&mut model, 'x');
        type_text(&mut model, "u");
        assert_eq!(model.editor_state.get_content(), "a");
        assert_eq!(model.editor_state.submode, None);
    }
}

#[test]
fn unsupported_modified_keys_never_become_text_or_search_input() {
    let (_dir, mut model) = model("body");
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::CONTROL | KeyModifiers::ALT] {
        key(&mut model, KeyCode::Char('t'), modifiers);
    }
    assert_eq!(model.editor_state.get_content(), "body");
    ctrl(&mut model, 'x');
    type_text(&mut model, "j");
    assert_eq!(model.editor_state.get_content(), "body");
    assert_eq!(model.editor_state.submode, None);
    ctrl(&mut model, 's');
    ctrl(&mut model, 'x');
    assert!(model.editor_state.search_pattern.is_empty());
}

#[test]
fn point_and_mark_cross_line_boundaries_and_keyboard_quit_deactivates_mark() {
    let (_dir, mut model) = model("한글\nnext");
    ctrl(&mut model, 'e');
    ctrl(&mut model, ' ');
    ctrl(&mut model, 'f');
    assert_eq!(cursor(&model), (1, 0));
    assert_eq!(model.editor_state.get_selected_text().as_deref(), Some("\n"));
    ctrl(&mut model, 'b');
    assert_eq!(cursor(&model), (0, 2));
    ctrl(&mut model, 'g');
    assert!(model.editor_state.selection.is_none());
    assert_eq!(model.editor_state.get_content(), "한글\nnext");
}

#[test]
fn word_motion_skips_punctuation_and_blank_lines_using_character_positions() {
    let (_dir, mut model) = model("한글, world!\n\n다음_단어 end");
    for expected in [(0, 2), (0, 9), (2, 5)] {
        alt(&mut model, 'f');
        assert_eq!(cursor(&model), expected);
    }
    for expected in [(2, 0), (0, 4), (0, 0)] {
        alt(&mut model, 'b');
        assert_eq!(cursor(&model), expected);
    }
}

#[test]
fn word_kills_yank_and_undo_keep_unicode_and_newlines() {
    let (_dir, mut model) = model("한글, next\n다음 단어");
    alt(&mut model, 'd');
    assert_eq!(model.editor_state.clipboard, "한글");
    assert_eq!(model.editor_state.get_content(), ", next\n다음 단어");
    ctrl(&mut model, 'y');
    assert_eq!(model.editor_state.get_content(), "한글, next\n다음 단어");
    alt(&mut model, '>');
    key(&mut model, KeyCode::Backspace, KeyModifiers::ALT);
    assert_eq!(model.editor_state.clipboard, "단어");
    assert_eq!(model.editor_state.get_content(), "한글, next\n다음 ");
    ctrl(&mut model, '/');
    assert_eq!(model.editor_state.get_content(), "한글, next\n다음 단어");
    alt(&mut model, '>');
    ctrl(&mut model, 'a');
    key(&mut model, KeyCode::Backspace, KeyModifiers::ALT);
    assert_eq!(model.editor_state.clipboard, "next\n");
    assert_eq!(model.editor_state.get_content(), "한글, 다음 단어");
    ctrl(&mut model, 'y');
    assert_eq!(model.editor_state.get_content(), "한글, next\n다음 단어");
}

#[test]
fn yanking_a_killed_newline_inserts_it_at_point() {
    let (_dir, mut model) = model("aa\nbb");
    ctrl(&mut model, 'e');
    ctrl(&mut model, 'k');
    assert_eq!(model.editor_state.get_content(), "aabb");
    assert_eq!(model.editor_state.clipboard, "\n");
    ctrl(&mut model, 'y');
    assert_eq!(model.editor_state.get_content(), "aa\nbb");
    assert_eq!(cursor(&model), (1, 0));
}

#[test]
fn incremental_unicode_search_moves_immediately_and_cancel_restores_origin() {
    let (_dir, mut model) = model("aa 한글 xx 한글 zz\n끝 한글");
    ctrl(&mut model, 's');
    type_text(&mut model, "한");
    assert_eq!(cursor(&model), (0, 3));
    assert_eq!(model.editor_state.search_matches.len(), 3);
    type_text(&mut model, "글");
    ctrl(&mut model, 's');
    assert_eq!(cursor(&model), (0, 9));
    ctrl(&mut model, 'r');
    assert_eq!(cursor(&model), (0, 3));
    key(&mut model, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(model.editor_state.search_pattern, "한");
    ctrl(&mut model, 'g');
    assert_eq!(cursor(&model), (0, 0));
    assert!(model.editor_state.search_matches.is_empty());
    assert_eq!(model.editor_state.submode, None);
    assert!(!model.editor_state.is_modified);
}

#[test]
fn reverse_search_wraps_and_reuses_the_last_query() {
    let (_dir, mut model) = model("한글 xx 한글");
    alt(&mut model, '>');
    ctrl(&mut model, 'r');
    type_text(&mut model, "한글");
    assert_eq!(cursor(&model), (0, 6));
    ctrl(&mut model, 'r');
    assert_eq!(cursor(&model), (0, 0));
    ctrl(&mut model, 'r');
    assert_eq!(cursor(&model), (0, 6));
    key(&mut model, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(model.editor_state.submode, None);
    assert_eq!(cursor(&model), (0, 6));
    ctrl(&mut model, 's');
    ctrl(&mut model, 's');
    assert_eq!(model.editor_state.search_pattern, "한글");
    key(&mut model, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(cursor(&model), (0, 8));
}

#[test]
fn empty_and_failing_searches_clear_stale_matches_without_modifying_text() {
    let (_dir, mut model) = model("한글 한글");
    ctrl(&mut model, 's');
    type_text(&mut model, "한");
    key(&mut model, KeyCode::Backspace, KeyModifiers::NONE);
    assert!(model.editor_state.search_matches.is_empty());
    assert_eq!(cursor(&model), (0, 0));
    type_text(&mut model, "없는검색어");
    assert!(model.editor_state.search_matches.is_empty());
    assert!(model.editor_state.selection.is_none());
    key(&mut model, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(cursor(&model), (0, 0));
    assert_eq!(model.editor_state.get_content(), "한글 한글");
}

#[test]
fn search_highlights_only_matching_unicode_characters() {
    let (_dir, mut model) = model("한글X");
    ctrl(&mut model, 's');
    type_text(&mut model, "한글");
    let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
    terminal.draw(|f| app::view(f, &model)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 1)].bg, model.settings.theme.palette().current_search);
    assert_eq!(buffer[(4, 1)].symbol(), "X");
    assert_eq!(buffer[(4, 1)].bg, model.settings.theme.palette().bg);
    model.settings.language = Language::English;
    ctrl(&mut model, 'r');
    terminal.draw(|f| app::view(f, &model)).unwrap();
    let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("I-search backward:"));
}

#[test]
fn emacs_cancel_and_navigation_work_in_settings() {
    let (_dir, mut model) = model("diary");
    key(&mut model, KeyCode::F(2), KeyModifiers::NONE);
    let row = model.settings_row;
    ctrl(&mut model, 'n');
    assert_ne!(model.settings_row, row);
    ctrl(&mut model, 'p');
    assert_eq!(model.settings_row, row);
    ctrl(&mut model, 'g');
    assert!(!model.show_settings);
    assert_eq!(model.editor_state.get_content(), "diary");
}
