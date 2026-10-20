use chrono::NaiveDate;
use ratatui::{Terminal,
              backend::TestBackend,
              layout::Rect};
use ratatui_diary::{Model,
                    app::{self,
                          Screen},
                    editor,
                    settings::{Language,
                               Theme},
                    theme};

#[test]
fn calendar_and_editor_help_align_and_switch_columns() {
    for screen in [Screen::Calendar, Screen::Editor] {
        for width in [80, 200] {
            let mut model = Model::new(NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(), Default::default());
            model.screen = screen;
            model.settings.language = Language::English;
            model.settings.theme = Theme::Dark;
            let mut terminal = Terminal::new(TestBackend::new(width, 30)).unwrap();
            terminal.draw(|f| app::view(f, &model)).unwrap();
            let buffer = terminal.backend().buffer();
            let rows: Vec<String> = (0 .. 30).map(|y| (0 .. width / 2).map(|x| buffer[(x, y)].symbol()).collect()).collect();
            let settings_row = rows.iter().position(|row| row.contains("Settings")).unwrap();
            let other = if screen == Screen::Calendar { "Move" } else { "Commands" };
            let other_row = rows.iter().position(|row| row.contains(other)).unwrap();
            assert_eq!(other_row, settings_row + usize::from(width == 80));
            assert!(rows.iter().any(|row| row.contains("Cancel")));
            assert_eq!(buffer[(0, settings_row as u16)].fg, theme::palette(Theme::Dark).accent);
            let description_x = rows[settings_row].find("Settings").unwrap() as u16;
            assert_eq!(buffer[(description_x, settings_row as u16)].fg, theme::palette(Theme::Dark).fg);
            if screen == Screen::Editor {
                let visible_rows = editor::view::viewport_height(&model, Rect::new(0, 0, width, 30));
                assert_eq!(visible_rows, settings_row - 2);
            }
        }
    }
}

#[test]
fn helix_command_prompt_and_colon_key_remain_separate() {
    let mut model = Model::new(NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(), Default::default());
    model.screen = Screen::Editor;
    model.settings.language = Language::English;
    model.settings.keybindings = ratatui_diary::settings::Keybindings::Helix;
    model.editor_state.helix.prefix = Some(editor::helix::Prefix::Command);
    model.editor_state.helix.prompt = "wq".to_owned();
    model.editor_state.helix.prompt_cursor = 2;
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|f| app::view(f, &model)).unwrap();
    let buffer = terminal.backend().buffer();
    let rows: Vec<String> = (0 .. 30).map(|y| (0 .. 60).map(|x| buffer[(x, y)].symbol()).collect()).collect();
    let prompt_row = rows.iter().position(|row| row.starts_with(":wq")).unwrap();
    assert!(rows.iter().skip(prompt_row + 1).any(|row| row.contains(":w") && row.contains("Save")));
    assert_eq!(terminal.get_cursor_position().unwrap().y, prompt_row as u16);
    assert_eq!(terminal.get_cursor_position().unwrap().x, 3);
}
