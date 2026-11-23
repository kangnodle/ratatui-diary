mod common;
use crate::common::TestApp;
use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyModifiers};
use ratatui::{Terminal,
              backend::TestBackend};
use ratatui_diary::{app,
                    editor::helix::{self,
                                    Mode},
                    model::Screen,
                    settings::{Keybindings,
                               Language,
                               Settings,
                               Theme},
                    storage::Storage};
use tempfile::TempDir;

fn fixture(content: &str) -> (TempDir, TestApp) {
    let dir = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let mut model = TestApp::new(
        Default::default(),
        match Storage::with_dir(dir.path()) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
        },
    );
    model.runtime.settings_path = Some(dir.path().join("settings.conf"));
    model.settings.keybindings = Keybindings::Helix;
    model.screen = Screen::Editor;
    model.editor_state.load_content(content);
    helix::reset(&mut model.editor_state, true);
    (dir, model)
}

fn key(model: &mut TestApp, code: KeyCode, modifiers: KeyModifiers) {
    if let Some(msg) = app::handle_key(KeyEvent::new(code, modifiers), model)
        && let Some(command) = app::update(model, msg)
    {
        crate::common::execute_command(command, model);
    }
}
fn keys(model: &mut TestApp, text: &str) {
    for ch in text.chars() {
        key(model, KeyCode::Char(ch), KeyModifiers::NONE);
    }
}
fn esc(model: &mut TestApp) { key(model, KeyCode::Esc, KeyModifiers::NONE); }
fn enter(model: &mut TestApp) { key(model, KeyCode::Enter, KeyModifiers::NONE); }
fn command(model: &mut TestApp, text: &str) {
    keys(model, &format!(":{text}"));
    enter(model);
}
fn selected(model: &TestApp) -> String { model.editor_state.get_selected_text().unwrap_or_default() }
fn point(model: &TestApp) -> (usize, usize) { (model.editor_state.cursor_line, model.editor_state.cursor_col) }

#[test]
fn preferences_are_backward_compatible_and_persist_both_keymaps() {
    let dir = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let path = dir.path().join("settings.conf");
    match std::fs::write(&path, "language=en\ntheme=dark\n") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    assert_eq!(
        (match diary_storage::settings::load(&path) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load settings: {error:?}"),
        })
        .keybindings,
        Keybindings::Emacs
    );
    for mode in [Keybindings::Emacs, Keybindings::Helix] {
        let settings = Settings {
            keybindings: mode,
            ..match diary_storage::settings::load(&path) {
                | Ok(value) => value,
                | Err(error) => panic!("Could not load settings: {error:?}"),
            }
        };
        match diary_storage::settings::save(settings, &path) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not save settings: {error:?}"),
        };
        assert_eq!(
            match diary_storage::settings::load(&path) {
                | Ok(value) => value,
                | Err(error) => panic!("Could not load settings: {error:?}"),
            },
            settings
        );
    }
    match std::fs::write(&path, "keybindings=vim\n") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    assert_eq!(diary_storage::settings::load(&path).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn switching_from_settings_preserves_inserted_text_and_undo_history() {
    let (_dir, mut model) = fixture("原文");
    keys(&mut model, "i한글");
    assert_eq!(model.editor_state.helix.mode, Mode::Insert);
    key(&mut model, KeyCode::F(2), KeyModifiers::NONE);
    key(&mut model, KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(model.settings_row, 2);
    key(&mut model, KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(model.settings.keybindings, Keybindings::Emacs);
    assert_eq!(model.editor_state.get_content(), "한글原文");
    assert_eq!(point(&model), (0, 2));
    assert!(model.editor_state.is_modified);
    assert_eq!(
        (match diary_storage::settings::load(match model.runtime.settings_path.as_deref() {
            | Some(value) => value,
            | None => panic!("Expected settings path to be present"),
        }) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load settings: {error:?}"),
        })
        .keybindings,
        Keybindings::Emacs
    );
    esc(&mut model);
    key(&mut model, KeyCode::Char('/'), KeyModifiers::CONTROL);
    assert_eq!(model.editor_state.get_content(), "原文");
    key(&mut model, KeyCode::F(2), KeyModifiers::NONE);
    key(&mut model, KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(model.settings.keybindings, Keybindings::Helix);
    esc(&mut model);
    assert_eq!(model.editor_state.helix.mode, Mode::Normal);
    assert_eq!(model.editor_state.get_content(), "原文");
}

#[test]
fn normal_does_not_insert_and_insert_session_is_one_undo_step() {
    let (_dir, mut model) = fixture("abc");
    keys(&mut model, "qz");
    assert_eq!(model.editor_state.get_content(), "abc");
    assert!(!model.should_quit);
    keys(&mut model, "ihello");
    esc(&mut model);
    assert_eq!(model.editor_state.get_content(), "helloabc");
    assert_eq!(selected(&model), "o");
    keys(&mut model, "u");
    assert_eq!(model.editor_state.get_content(), "abc");
    keys(&mut model, "U");
    assert_eq!(model.editor_state.get_content(), "helloabc");
}

#[test]
fn normal_motion_replaces_selection_and_select_mode_extends_in_both_directions() {
    let (_dir, mut model) = fixture("한글abc");
    keys(&mut model, "l");
    assert_eq!(selected(&model), "글");
    keys(&mut model, "vll");
    assert_eq!(selected(&model), "글ab");
    keys(&mut model, "hhhl");
    assert_eq!(selected(&model), "글");
    keys(&mut model, "h");
    assert_eq!(selected(&model), "한글");
    esc(&mut model);
    assert_eq!(model.editor_state.helix.mode, Mode::Normal);
    keys(&mut model, "l");
    assert_eq!(selected(&model), "글");
    keys(&mut model, "vll;");
    assert_eq!(selected(&model), "b");
}

#[test]
fn word_motions_select_before_delete_and_change_instead_of_vim_operator_order() {
    let (_dir, mut model) = fixture("one two three");
    keys(&mut model, "w");
    assert_eq!(selected(&model), "one ");
    keys(&mut model, "d");
    assert_eq!(model.editor_state.get_content(), "two three");
    assert_eq!(model.editor_state.clipboard, "one ");
    keys(&mut model, "wcNEW");
    esc(&mut model);
    assert_eq!(model.editor_state.get_content(), "NEWthree");
    keys(&mut model, "u");
    assert_eq!(model.editor_state.get_content(), "two three");
    keys(&mut model, "gg2w");
    assert_eq!(selected(&model), "three");
    keys(&mut model, "b");
    assert_eq!(selected(&model), "three");
}

#[test]
fn word_and_word_end_distinguish_punctuation_and_long_words() {
    let (_dir, mut model) = fixture("abc.def  다음_단어");
    keys(&mut model, "w");
    assert_eq!(selected(&model), "abc");
    keys(&mut model, "w");
    assert_eq!(selected(&model), ".");
    keys(&mut model, "ggW");
    assert_eq!(selected(&model), "abc.def  ");
    keys(&mut model, "ggE");
    assert_eq!(selected(&model), "abc.def");
    keys(&mut model, "gge");
    assert_eq!(selected(&model), "abc");
}

#[test]
fn counts_goto_and_find_have_helix_meanings() {
    let (_dir, mut model) = fixture("abcdef\n12345\nxyz");
    keys(&mut model, "3l");
    assert_eq!(point(&model), (0, 3));
    keys(&mut model, "2gg");
    assert_eq!(point(&model), (1, 0));
    keys(&mut model, "ge");
    assert_eq!(point(&model), (2, 0));
    keys(&mut model, "1G");
    assert_eq!(point(&model), (0, 0));
    keys(&mut model, "vf5");
    assert_eq!(selected(&model), "abcdef\n12345");
    keys(&mut model, ";");
    assert_eq!(selected(&model), "5");
    assert_eq!(point(&model), (1, 4));
    keys(&mut model, "Ta");
    assert_eq!(point(&model), (0, 1));
    assert_eq!(selected(&model), "bcdef\n12345");
}

#[test]
fn whole_line_selection_extends_and_yanks_paste_as_lines() {
    let (_dir, mut model) = fixture("one\ntwo\nthree");
    keys(&mut model, "x");
    assert_eq!(selected(&model), "one\n");
    keys(&mut model, "x");
    assert_eq!(selected(&model), "one\ntwo\n");
    keys(&mut model, "ggxyjp");
    assert_eq!(model.editor_state.get_content(), "one\ntwo\none\nthree");
    keys(&mut model, "u");
    assert_eq!(model.editor_state.get_content(), "one\ntwo\nthree");
    keys(&mut model, "geP");
    assert_eq!(model.editor_state.get_content(), "one\ntwo\none\nthree");
}

#[test]
fn replace_is_characterwise_and_paste_before_after_preserves_unicode() {
    let (_dir, mut model) = fixture("한글abc");
    keys(&mut model, "vllr界");
    assert_eq!(model.editor_state.get_content(), "界界界bc");
    keys(&mut model, "uyglp");
    assert_eq!(model.editor_state.get_content(), "한글abc한글a");
    keys(&mut model, "ggP");
    assert_eq!(model.editor_state.get_content(), "한글a한글abc한글a");
}

#[test]
fn insert_positions_and_open_lines_are_not_normal_text_commands() {
    for (binding, expected) in [("i", "Xabc"), ("a", "aXbc"), ("I", "Xabc"), ("A", "abcX"), ("o", "abc\nX"), ("O", "X\nabc")] {
        let (_dir, mut model) = fixture("abc");
        keys(&mut model, binding);
        keys(&mut model, "X");
        esc(&mut model);
        assert_eq!(model.editor_state.get_content(), expected, "{binding}");
        keys(&mut model, "u");
        assert_eq!(model.editor_state.get_content(), "abc", "{binding}");
    }
}

#[test]
fn insert_controls_do_not_run_emacs_save_search_or_quit_commands() {
    let (_dir, mut model) = fixture("abc");
    keys(&mut model, "A hello");
    key(&mut model, KeyCode::Char('w'), KeyModifiers::CONTROL);
    assert_eq!(model.editor_state.get_content(), "abc ");
    key(&mut model, KeyCode::Char('q'), KeyModifiers::CONTROL);
    key(&mut model, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert!(!model.should_quit);
    key(&mut model, KeyCode::Char('s'), KeyModifiers::CONTROL);
    keys(&mut model, "world");
    esc(&mut model);
    keys(&mut model, "u");
    assert_eq!(model.editor_state.get_content(), "abc ");
}

#[test]
fn regex_search_selects_complete_unicode_matches_and_repeats_by_direction() {
    let (_dir, mut model) = fixture("a 한글123 b 한글45 c");
    keys(&mut model, "/한글[0-9]+");
    enter(&mut model);
    assert_eq!(selected(&model), "한글123");
    keys(&mut model, "n");
    assert_eq!(selected(&model), "한글45");
    keys(&mut model, "N");
    assert_eq!(selected(&model), "한글123");
    keys(&mut model, "gl?한글[0-9]+");
    enter(&mut model);
    assert_eq!(selected(&model), "한글45");
    keys(&mut model, "n");
    assert_eq!(selected(&model), "한글123");
    keys(&mut model, "N");
    assert_eq!(selected(&model), "한글45");
}

#[test]
fn search_escape_restores_selection_and_invalid_regex_does_not_change_content() {
    let (_dir, mut model) = fixture("abc def abc");
    keys(&mut model, "vl");
    let original = selected(&model);
    let origin = point(&model);
    keys(&mut model, "/def");
    assert_eq!(selected(&model), "def");
    esc(&mut model);
    assert_eq!(selected(&model), original);
    assert_eq!(point(&model), origin);
    keys(&mut model, "/[");
    enter(&mut model);
    assert!(model.show_error_popup);
    assert_eq!(model.editor_state.get_content(), "abc def abc");
    esc(&mut model);
    esc(&mut model);
    assert!(!model.show_error_popup);
    assert_eq!(point(&model), origin);
}

#[test]
fn helix_quit_refuses_dirty_text_and_write_quit_only_exits_after_success() {
    let (dir, mut model) = fixture("");
    keys(&mut model, "i* Org diary");
    esc(&mut model);
    command(&mut model, "q");
    assert!(!model.should_quit);
    assert!(model.show_error_popup);
    esc(&mut model);
    let path = dir.path().join(format!("entries/{}.org", model.editor_state.date));
    match std::fs::create_dir(&path) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the directory: {error:?}"),
    };
    command(&mut model, "wq");
    assert!(!model.should_quit);
    assert!(model.editor_state.is_modified);
    esc(&mut model);
    match std::fs::remove_dir(&path) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the directory: {error:?}"),
    };
    command(&mut model, "wq");
    assert!(model.should_quit);
    assert_eq!(
        match std::fs::read_to_string(path) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "* Org diary"
    );
}

#[test]
fn explicit_force_quit_does_not_write_or_lose_data_implicitly() {
    let (dir, mut model) = fixture("original");
    keys(&mut model, "ichange");
    esc(&mut model);
    command(&mut model, "q!");
    assert!(model.should_quit);
    assert!(model.editor_state.is_modified);
    assert!(!dir.path().join(format!("entries/{}.org", model.editor_state.date)).exists());
}

#[test]
fn space_w_does_not_save_and_space_f_uses_the_diary_picker() {
    let (dir, mut model) = fixture("");
    keys(&mut model, "i* Diary");
    esc(&mut model);
    keys(&mut model, " w");
    assert!(model.editor_state.is_modified);
    assert!(!dir.path().join(format!("entries/{}.org", model.editor_state.date)).exists());
    keys(&mut model, " f");
    assert_eq!(model.screen, Screen::Calendar);
    assert!(!model.editor_state.is_modified);
    keys(&mut model, "hljk");
    enter(&mut model);
    assert_eq!(model.screen, Screen::Editor);
    assert_eq!(model.editor_state.helix.mode, Mode::Normal);
    assert_eq!(model.editor_state.get_content(), "* Diary");
}

#[test]
fn page_motion_uses_viewport_height_and_keeps_the_cursor_visible() {
    let text = (0 .. 50).map(|i| format!("line{i}")).collect::<Vec<_>>().join("\n");
    let (_dir, mut model) = fixture(&text);
    model.editor_state.viewport_height = 8;
    key(&mut model, KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert_eq!(point(&model), (8, 0));
    key(&mut model, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert_eq!(point(&model), (4, 0));
    keys(&mut model, "ge");
    let mut terminal = match Terminal::new(TestBackend::new(80, 12)) {
        | Ok(value) => value,
        | Err(never) => match never {},
    };
    match terminal.draw(|f| app::view(f, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };
    let cursor = match terminal.get_cursor_position() {
        | Ok(value) => value,
        | Err(never) => match never {},
    };
    assert!(cursor.x < 40 && cursor.y < 9);
    let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("line49"));
}

#[test]
fn helix_modes_and_both_languages_render_safely_in_small_terminals() {
    let (_dir, mut model) = fixture("한글\norg");
    for language in [Language::Korean, Language::English] {
        model.settings.language = language;
        for mode in [Mode::Normal, Mode::Insert, Mode::Select] {
            model.editor_state.helix.mode = mode;
            for (w, h) in [(1, 1), (20, 5), (80, 24), (160, 30)] {
                let mut terminal = match Terminal::new(TestBackend::new(w, h)) {
                    | Ok(value) => value,
                    | Err(never) => match never {},
                };
                match terminal.draw(|f| app::view(f, &model)) {
                    | Ok(_) => (),
                    | Err(never) => match never {},
                };
            }
        }
    }
}

#[test]
fn search_cancel_preserves_the_previous_accepted_query() {
    let (_dir, mut model) = fixture("abc def abc");
    keys(&mut model, "/abc");
    enter(&mut model);
    keys(&mut model, "/def");
    assert_eq!(selected(&model), "def");
    esc(&mut model);
    keys(&mut model, "n");
    assert_eq!(selected(&model), "abc");
    assert_eq!(point(&model), (0, 10));
}

#[test]
fn line_paste_at_eof_and_save_reload_preserve_the_final_newline() {
    let (dir, mut model) = fixture("one\ntwo");
    keys(&mut model, "xygep");
    assert_eq!(model.editor_state.get_content(), "one\ntwo\none\n");
    command(&mut model, "w");
    let date = model.editor_state.date;
    let path = dir.path().join(format!("entries/{date}.org"));
    assert_eq!(
        match std::fs::read_to_string(path) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "one\ntwo\none\n"
    );
    command(&mut model, "bc");
    enter(&mut model);
    assert_eq!(model.editor_state.get_content(), "one\ntwo\none\n");
}

#[test]
fn opening_and_splitting_lines_keeps_the_existing_indentation() {
    for binding in ["o", "O"] {
        let (_dir, mut model) = fixture("  abc");
        keys(&mut model, binding);
        assert_eq!(model.editor_state.content[model.editor_state.cursor_line], "  ");
        assert_eq!(model.editor_state.cursor_col, 2);
        esc(&mut model);
        keys(&mut model, "u");
        assert_eq!(model.editor_state.get_content(), "  abc");
    }
    let (_dir, mut model) = fixture("  abc");
    keys(&mut model, "A");
    enter(&mut model);
    keys(&mut model, "next");
    esc(&mut model);
    assert_eq!(model.editor_state.get_content(), "  abc\n  next");
}

#[test]
fn writing_legacy_markdown_from_helix_preserves_its_format() {
    let (dir, mut model) = fixture("");
    let path = dir.path().join(format!("entries/{}.md", model.editor_state.date));
    match std::fs::write(&path, "# Existing diary\n") {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    crate::common::execute_command(app::Command::LoadDiary(model.editor_state.date), &mut model);
    keys(&mut model, "A updated");
    esc(&mut model);
    command(&mut model, "w");
    assert_eq!(
        match std::fs::read_to_string(&path) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "# Existing diary updated\n"
    );
    assert!(!path.with_extension("org").exists());
}

#[test]
fn unmodified_write_quit_still_writes_and_observes_save_failures() {
    let (dir, mut model) = fixture("unchanged");
    let path = dir.path().join(format!("entries/{}.org", model.editor_state.date));
    match std::fs::create_dir(&path) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not create the directory: {error:?}"),
    };
    command(&mut model, "wq");
    assert!(!model.should_quit);
    assert!(model.show_error_popup);
    esc(&mut model);
    match std::fs::remove_dir(&path) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not remove the directory: {error:?}"),
    };
    command(&mut model, "wq");
    assert!(model.should_quit);
    assert_eq!(
        match std::fs::read_to_string(path) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the text file: {error:?}"),
        },
        "unchanged"
    );
}

#[test]
fn settings_show_the_keymap_in_every_language_theme_and_screen() {
    let (_dir, mut model) = fixture("diary");
    for language in [Language::Korean, Language::English] {
        for theme in [Theme::System, Theme::Dark, Theme::Light] {
            for screen in [Screen::Calendar, Screen::Editor] {
                model.settings.language = language;
                model.settings.theme = theme;
                model.screen = screen;
                model.show_settings = true;
                let mut terminal = match Terminal::new(TestBackend::new(160, 30)) {
                    | Ok(value) => value,
                    | Err(never) => match never {},
                };
                match terminal.draw(|f| app::view(f, &model)) {
                    | Ok(_) => (),
                    | Err(never) => match never {},
                };
                let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
                assert!(text.contains("Helix"));
                assert!(text.replace(' ', "").contains(language.text("키바인딩", "Keybindings")));
            }
        }
    }
}

#[test]
fn search_uses_helix_smart_case_by_default() {
    let (_dir, mut model) = fixture("ABC abc Abc");
    keys(&mut model, "/abc");
    enter(&mut model);
    assert_eq!(selected(&model), "ABC");
    keys(&mut model, "n");
    assert_eq!(selected(&model), "abc");
    keys(&mut model, "/Abc");
    enter(&mut model);
    assert_eq!(selected(&model), "Abc");
    keys(&mut model, "n");
    assert_eq!(selected(&model), "Abc");
}

#[test]
fn select_search_repeat_does_not_fake_multiple_selections_as_one_large_region() {
    let (_dir, mut model) = fixture("one two one");
    keys(&mut model, "/one");
    enter(&mut model);
    let selection = selected(&model);
    let cursor = point(&model);
    keys(&mut model, "vnN");
    assert_eq!(selected(&model), selection);
    assert_eq!(point(&model), cursor);
}
