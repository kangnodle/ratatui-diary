//! Reproducible action timings; setup and teardown are outside each sample.
use chrono::NaiveDate;
use diary_application::{Model,
                        Msg,
                        Screen,
                        app,
                        input::{KeyCode,
                                KeyEvent,
                                KeyModifiers},
                        settings::Keybindings};
use ratatui::{Terminal,
              backend::TestBackend};
use std::{hint::black_box,
          io,
          time::Instant};

fn editor(content: &str) -> Model {
    let mut model = Model::new(NaiveDate::default(), Default::default());
    model.screen = Screen::Editor;
    model.editor_state.load_content(content);
    model
}

fn measure<T>(name: &str, mut setup: impl FnMut() -> io::Result<T>, mut action: impl FnMut(&mut T) -> io::Result<()>) -> io::Result<()> {
    let mut samples = Vec::new();
    for sample in 0 .. 8 {
        let mut state = setup()?;
        let start = Instant::now();
        action(&mut state)?;
        black_box(&state);
        let elapsed = start.elapsed().as_micros();
        if sample > 0 {
            samples.push(elapsed);
        }
    }
    samples.sort_unstable();
    println!("{name}: median {} us", samples[samples.len() / 2]);
    Ok(())
}

fn main() -> io::Result<()> {
    let dense = "한".repeat(30_000);
    measure(
        "unicode_search_30000_matches",
        || {
            let mut model = editor(&dense);
            model.editor_state.search_pattern = "한".into();
            Ok(model)
        },
        |model| {
            model.editor_state.execute_search();
            assert_eq!(model.editor_state.search_matches.len(), 30_000);
            Ok(())
        },
    )?;
    measure(
        "emacs_paste_30000_characters",
        || {
            let mut model = editor("prefix suffix");
            model.editor_state.cursor_col = 7;
            model.editor_state.clipboard = dense.clone();
            Ok(model)
        },
        |model| {
            black_box(app::update(model, Msg::EditorPaste));
            assert_eq!(model.editor_state.cursor_col, 30_007);
            Ok(())
        },
    )?;
    let document = format!("{}\n", "한글 hit ".repeat(30)).repeat(2_000);
    for search in [false, true] {
        measure(
            if search { "render_60000_search_matches" } else { "render_without_search" },
            || {
                let mut model = editor(&document);
                if search {
                    model.editor_state.search_pattern = "한글".into();
                    model.editor_state.execute_search();
                }
                model.editor_state.cursor_line = 1_999;
                model.editor_state.cursor_col = 0;
                let terminal = match Terminal::new(TestBackend::new(120, 32)) {
                    | Ok(terminal) => terminal,
                    | Err(never) => match never {},
                };
                Ok((model, terminal))
            },
            |(model, terminal)| {
                match terminal.draw(|frame| ratatui_diary::app::view(frame, model)) {
                    | Ok(_) => {},
                    | Err(never) => match never {},
                }
                Ok(())
            },
        )?;
    }
    measure(
        "helix_20_cursor_moves",
        || {
            let mut model = editor(&document);
            model.settings.keybindings = Keybindings::Helix;
            diary_application::editor::helix::reset(&mut model.editor_state, true);
            Ok(model)
        },
        |model| {
            for _ in 0 .. 20 {
                black_box(app::update(model, Msg::HelixKey(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE))));
            }
            Ok(())
        },
    )
}
