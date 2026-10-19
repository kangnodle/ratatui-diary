use chrono::NaiveDate;
use crossterm::{event::{self,
                        Event},
                execute,
                terminal::{EnterAlternateScreen,
                           LeaveAlternateScreen,
                           disable_raw_mode,
                           enable_raw_mode}};
use ratatui::{Terminal,
              backend::CrosstermBackend};
use ratatui_diary::{Model,
                    Runtime,
                    app};
use std::{io,
          time::Duration};

fn main() -> std::io::Result<()> {
    let (mut runtime, mut model) = ratatui_diary::Runtime::start()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // 이벤트 루프
    let result = run_app(
        &mut model,
        &mut runtime,
        |model| {
            model.editor_state.viewport_height = usize::from(terminal.size()?.height.saturating_sub(4).max(1));
            terminal.draw(|frame| app::view(frame, model))?;
            Ok(())
        },
        |timeout| {
            if event::poll(timeout)? { event::read().map(Some) } else { Ok(None) }
        },
        || chrono::Local::now().date_naive(),
    );

    // Terminal 복원
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// Rendering and terminal input stay at the shell; tests supply in-memory
// drivers.
fn run_app(
    model: &mut Model,
    runtime: &mut Runtime,
    mut draw: impl FnMut(&mut Model) -> io::Result<()>,
    mut next_event: impl FnMut(Duration) -> io::Result<Option<Event>>,
    mut today: impl FnMut() -> NaiveDate,
) -> io::Result<()> {
    let mut redraw = true;
    loop {
        let date = today();
        if model.today != date {
            model.today = date;
            redraw = true;
        }
        if redraw {
            draw(model)?;
            redraw = false;
        }
        match next_event(Duration::from_secs(1))? {
            | Some(Event::Key(key)) => {
                if let Some(message) = app::handle_key(key, model) {
                    if let Some(command) = app::update(model, message) {
                        runtime.execute(command, model);
                    }
                    redraw = true;
                }
                if model.should_quit {
                    return Ok(());
                }
            },
            | Some(Event::Resize(_, _)) => redraw = true,
            | _ => {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode,
                           KeyEvent,
                           KeyEventKind,
                           KeyModifiers};
    use ratatui_diary::storage::Storage;
    use tempfile::TempDir;

    fn quit() -> Option<Event> { Some(Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL))) }

    #[test]
    fn idle_polls_and_released_keys_do_not_redraw_but_settings_and_resize_do() {
        let directory = TempDir::new().unwrap();
        let mut runtime = Runtime::new(Storage::with_dir(directory.path()).unwrap(), None);
        let date = NaiveDate::default();
        let mut model = Model::new(date, Default::default());
        let settings = Some(Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE)));
        let release = Some(Event::Key(KeyEvent::new_with_kind(KeyCode::F(2), KeyModifiers::NONE, KeyEventKind::Release)));
        let mut events = [None, None, settings.clone(), release, None, settings, Some(Event::Resize(100, 30)), quit()].into_iter();
        let mut frames = Vec::new();
        run_app(
            &mut model,
            &mut runtime,
            |model| {
                frames.push(model.show_settings);
                Ok(())
            },
            |_| Ok(events.next().expect("unexpected poll")),
            || date,
        )
        .unwrap();
        assert_eq!(frames, [false, true, false, false]);
        assert!(model.should_quit);
    }

    #[test]
    fn an_idle_app_still_updates_the_date_when_midnight_passes() {
        let directory = TempDir::new().unwrap();
        let mut runtime = Runtime::new(Storage::with_dir(directory.path()).unwrap(), None);
        let date = NaiveDate::default();
        let tomorrow = date.succ_opt().unwrap();
        let mut model = Model::new(date, Default::default());
        let mut dates = [date, tomorrow, tomorrow].into_iter();
        let mut events = [None, None, quit()].into_iter();
        let mut frames = Vec::new();
        run_app(
            &mut model,
            &mut runtime,
            |model| {
                frames.push(model.today);
                Ok(())
            },
            |_| Ok(events.next().expect("unexpected poll")),
            || dates.next().unwrap(),
        )
        .unwrap();
        assert_eq!(frames, [date, tomorrow]);
        assert_eq!(model.calendar_state.selected_date, date);
    }
}
