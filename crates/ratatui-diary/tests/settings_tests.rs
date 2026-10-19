use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyModifiers};
use ratatui::{Terminal,
              backend::TestBackend,
              style::Color};
use ratatui_diary::{Model,
                    Msg,
                    app,
                    model::{EditorSubMode,
                            Screen},
                    settings::{Language,
                               Settings,
                               Theme},
                    storage::Storage};
use tempfile::TempDir;

fn model(dir: &TempDir) -> Model {
    let mut model = Model::new(Default::default(), Storage::with_dir(dir.path()).unwrap());
    model.settings_path = Some(dir.path().join("config/settings.conf"));
    model
}
fn press(model: &mut Model, code: KeyCode) {
    if let Some(msg) = app::handle_key(KeyEvent::new(code, KeyModifiers::NONE), model) {
        if let Some(cmd) = app::update(model, msg) {
            app::execute_command(cmd, model).unwrap();
        }
    }
}
fn rendered(model: &Model, width: u16, height: u16) -> (String, ratatui::buffer::Buffer) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| app::view(f, model)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut text = String::new();
    for y in 0 .. height {
        let mut x = 0;
        while x < width {
            let symbol = buffer[(x, y)].symbol();
            text.push_str(symbol);
            x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
        }
        text.push('\n');
    }
    (text, buffer)
}

#[test]
fn preferences_round_trip_and_validate() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nested/settings.conf");
    assert_eq!(Settings::load(&path).unwrap(), Settings::default());
    for language in [Language::Korean, Language::English] {
        for theme in [Theme::System, Theme::Dark, Theme::Light] {
            let settings = Settings {
                language,
                theme,
            };
            settings.save(&path).unwrap();
            assert_eq!(Settings::load(&path).unwrap(), settings);
        }
    }
    std::fs::write(&path, "theme=invalid").unwrap();
    assert_eq!(Settings::load(&path).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn popup_changes_save_immediately_and_preserve_editor_state() {
    let dir = TempDir::new().unwrap();
    let mut model = model(&dir);
    model.screen = Screen::Editor;
    model.editor_state.load_content("한글 diary");
    model.editor_state.submode = Some(EditorSubMode::Search);
    press(&mut model, KeyCode::F(2));
    press(&mut model, KeyCode::Char('x'));
    assert_eq!(model.editor_state.get_content(), "한글 diary");
    assert!(model.editor_state.search_pattern.is_empty());
    press(&mut model, KeyCode::Right);
    assert_eq!(model.settings.language, Language::English);
    press(&mut model, KeyCode::Tab);
    for theme in [Theme::Dark, Theme::Light, Theme::System] {
        press(&mut model, KeyCode::Right);
        assert_eq!(model.settings.theme, theme);
        assert_eq!(Settings::load(model.settings_path.as_deref().unwrap()).unwrap(), model.settings);
    }
    press(&mut model, KeyCode::Left);
    assert_eq!(model.settings.theme, Theme::Light);
    press(&mut model, KeyCode::Esc);
    assert!(!model.show_settings);
    assert_eq!(model.editor_state.submode, Some(EditorSubMode::Search));
}

#[test]
fn every_language_and_theme_renders_screens_and_overlays() {
    let dir = TempDir::new().unwrap();
    let mut model = model(&dir);
    model.editor_state.load_content("* 한글 diary\nhello");
    for language in [Language::Korean, Language::English] {
        for theme in [Theme::System, Theme::Dark, Theme::Light] {
            model.settings = Settings {
                language,
                theme,
            };
            for screen in [Screen::Calendar, Screen::Editor] {
                model.screen = screen;
                let (text, buffer) = rendered(&model, 160, 30);
                assert!(text.contains("F2:"));
                assert!(text.contains(language.text("설정", "Settings")));
                assert_eq!(buffer[(159, 29)].bg, theme.palette().bg);
                if screen == Screen::Calendar {
                    assert!(text.contains(language.text("작성된 다이어리가 없습니다", "No diary entry")));
                    assert!(text.contains(language.text("일", "Sun")));
                } else {
                    assert!(text.contains(language.text("Org 미리보기", "Org preview")));
                }
                model.show_settings = true;
                let (text, _) = rendered(&model, 160, 30);
                assert!(text.contains(language.text("언어:", "Language:")));
                assert!(text.contains(theme.label(language)));
                for (w, h) in [(1, 1), (20, 5), (80, 24)] {
                    rendered(&model, w, h);
                }
                model.show_settings = false;
            }
        }
    }
    assert_eq!(Theme::System.palette().bg, Color::Reset);
    assert_eq!(Theme::System.palette().fg, Color::Reset);
}

#[test]
fn org_and_legacy_markdown_keep_their_format_through_edit_save_and_preview() {
    for legacy in [false, true] {
        let dir = TempDir::new().unwrap();
        let mut model = model(&dir);
        let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        if legacy {
            std::fs::write(dir.path().join("entries/2026-10-19.md"), "# Existing diary").unwrap();
        }
        model.calendar_state.selected_date = date;
        let command = app::update(&mut model, Msg::CalendarSelectDate).unwrap();
        app::execute_command(command, &mut model).unwrap();
        assert_eq!(model.screen, Screen::Editor);
        assert_eq!(model.editor_state.get_content(), if legacy { "# Existing diary" } else { "" });

        let content = if legacy {
            "# Today\n## Lessons\nDiary body"
        } else {
            "* Today\n** Lessons\nDiary body"
        };
        model.editor_state.load_content(content);
        for language in [Language::Korean, Language::English] {
            model.settings.language = language;
            let (text, _) = rendered(&model, 160, 30);
            assert!(text.contains(if legacy {
                language.text("Markdown 미리보기", "Markdown preview")
            } else {
                language.text("Org 미리보기", "Org preview")
            }));
            assert_eq!(text.contains("◉ Today"), !legacy);
            assert_eq!(text.contains("○ Lessons"), !legacy);
            assert!(text.contains("Diary body"));
            assert_eq!(model.editor_state.get_content(), content);
        }

        app::update(&mut model, Msg::EditorEnterCtrlXMode);
        let command = app::update(&mut model, Msg::EditorCtrlXSave).unwrap();
        app::execute_command(command, &mut model).unwrap();
        assert!(!model.editor_state.is_modified);
        assert!(!model.show_error_popup);
        let extension = if legacy { "md" } else { "org" };
        assert_eq!(
            std::fs::read_to_string(dir.path().join(format!("entries/{date}.{extension}"))).unwrap(),
            content
        );
        assert!(model.diary_entries.entries.contains(&date));

        app::update(&mut model, Msg::EditorEnterCtrlXMode);
        app::update(&mut model, Msg::EditorCtrlXBack);
        assert_eq!(model.screen, Screen::Calendar);
        let (text, _) = rendered(&model, 160, 30);
        assert_eq!(text.contains("◉ Today"), !legacy);
        assert_eq!(text.contains("○ Lessons"), !legacy);
        assert!(text.contains("Diary body"));
        let command = app::update(&mut model, Msg::CalendarSelectDate).unwrap();
        app::execute_command(command, &mut model).unwrap();
        assert_eq!(model.editor_state.get_content(), content);
    }
}

#[test]
fn save_failure_is_visible_and_error_popup_blocks_input() {
    let dir = TempDir::new().unwrap();
    let mut model = model(&dir);
    model.settings.language = Language::English;
    model.settings_path = Some(dir.path().to_path_buf()); // Writing to a directory fails.
    model.screen = Screen::Editor;
    press(&mut model, KeyCode::F(2));
    press(&mut model, KeyCode::Tab);
    press(&mut model, KeyCode::Right);
    assert!(model.show_error_popup);
    assert!(model.error_message.as_ref().unwrap().contains("Failed to save settings"));
    press(&mut model, KeyCode::Char('x'));
    assert_eq!(model.editor_state.get_content(), "");
    assert!(rendered(&model, 160, 30).0.contains("Error"));
    press(&mut model, KeyCode::Esc);
    assert!(!model.show_error_popup);
    assert!(model.show_settings);
    app::update(&mut model, Msg::ToggleSettings);
    app::update(&mut model, Msg::LoadDiaryFailed("details".into()));
    assert_eq!(model.error_message.as_deref(), Some("Load failed: details"));
}
