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
            let mut model = Model::new(
                match NaiveDate::from_ymd_opt(2026, 10, 20) {
                    | Some(value) => value,
                    | None => panic!("Invalid date in test fixture"),
                },
                Default::default(),
            );
            model.screen = screen;
            model.settings.language = Language::English;
            model.settings.theme = Theme::Dark;
            let mut terminal = match Terminal::new(TestBackend::new(width, 30)) {
                | Ok(value) => value,
                | Err(never) => match never {},
            };
            match terminal.draw(|f| app::view(f, &model)) {
                | Ok(_) => (),
                | Err(never) => match never {},
            };
            let buffer = terminal.backend().buffer();
            let rows: Vec<String> = (0 .. 30).map(|y| (0 .. width / 2).map(|x| buffer[(x, y)].symbol()).collect()).collect();
            let settings_row = match rows.iter().position(|row| row.contains("Settings")) {
                | Some(value) => value,
                | None => panic!("Expected a matching item in the test output"),
            };
            let other = if screen == Screen::Calendar { "Move" } else { "Commands" };
            let other_row = match rows.iter().position(|row| row.contains(other)) {
                | Some(value) => value,
                | None => panic!("Expected a matching item in the test output"),
            };
            assert_eq!(other_row, settings_row + usize::from(width == 80));
            assert!(rows.iter().any(|row| row.contains("Cancel")));
            assert_eq!(buffer[(0, settings_row as u16)].fg, theme::palette(Theme::Dark).accent);
            let description_x = (match rows[settings_row].find("Settings") {
                | Some(value) => value,
                | None => panic!("Expected to find the text in the test output"),
            }) as u16;
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
    let mut model = Model::new(
        match NaiveDate::from_ymd_opt(2026, 10, 20) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        },
        Default::default(),
    );
    model.screen = Screen::Editor;
    model.settings.language = Language::English;
    model.settings.keybindings = ratatui_diary::settings::Keybindings::Helix;
    model.editor_state.helix.prefix = Some(editor::helix::Prefix::Command);
    model.editor_state.helix.prompt = "wq".to_owned();
    model.editor_state.helix.prompt_cursor = 2;
    let mut terminal = match Terminal::new(TestBackend::new(120, 30)) {
        | Ok(value) => value,
        | Err(never) => match never {},
    };
    match terminal.draw(|f| app::view(f, &model)) {
        | Ok(_) => (),
        | Err(never) => match never {},
    };
    let buffer = terminal.backend().buffer();
    let rows: Vec<String> = (0 .. 30).map(|y| (0 .. 60).map(|x| buffer[(x, y)].symbol()).collect()).collect();
    let prompt_row = match rows.iter().position(|row| row.starts_with(":wq")) {
        | Some(value) => value,
        | None => panic!("Expected a matching item in the test output"),
    };
    assert!(rows.iter().skip(prompt_row + 1).any(|row| row.contains(":w") && row.contains("Save")));
    assert_eq!(
        (match terminal.get_cursor_position() {
            | Ok(value) => value,
            | Err(never) => match never {},
        })
        .y,
        prompt_row as u16
    );
    assert_eq!(
        (match terminal.get_cursor_position() {
            | Ok(value) => value,
            | Err(never) => match never {},
        })
        .x,
        3
    );
}
