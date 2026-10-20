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
            model.editor_state.viewport_height =
                ratatui_diary::editor::view::viewport_height(model, ratatui::layout::Rect::from((ratatui::layout::Position::ORIGIN, terminal.size()?)));
            terminal.draw(|frame| app::view(frame, model))?;
            Ok(())
        },
        |timeout| {
            if event::poll(timeout)? { event::read().map(Some) } else { Ok(None) }
        },
        || chrono::Local::now().date_naive(),
        Runtime::preload_search_model,
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
    preload: impl FnOnce(&mut Runtime),
) -> io::Result<()> {
    let mut redraw = true;
    let mut preload = Some(preload);
    loop {
        redraw |= runtime.poll(model);
        let date = today();
        if model.today != date {
            model.today = date;
            redraw = true;
        }
        if redraw {
            draw(model)?;
            redraw = false;
        }
        if let Some(preload) = preload.take() {
            preload(runtime);
        }
        match next_event(if model.search.busy {
            Duration::from_millis(50)
        } else {
            Duration::from_secs(1)
        })? {
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
    fn preload_is_scheduled_once_after_first_frame_and_does_not_add_redraws() {
        use std::cell::Cell;
        let directory = TempDir::new().unwrap();
        let mut runtime = Runtime::new(Storage::with_dir(directory.path()).unwrap(), None);
        let date = NaiveDate::default();
        let mut model = Model::new(date, Default::default());
        let frames = Cell::new(0);
        let preloads = Cell::new(0);
        let settings = Some(Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE)));
        let mut events = [None, settings.clone(), Some(Event::Resize(80, 24)), settings, quit()].into_iter();
        run_app(
            &mut model,
            &mut runtime,
            |_| {
                frames.set(frames.get() + 1);
                Ok(())
            },
            |_| {
                assert_eq!(preloads.get(), 1);
                Ok(events.next().unwrap())
            },
            || date,
            |_| {
                assert_eq!(frames.get(), 1);
                preloads.set(preloads.get() + 1);
            },
        )
        .unwrap();
        assert_eq!(preloads.get(), 1);
        assert_eq!(frames.get(), 4);
    }

    #[test]
    #[ignore = "release TUI responsiveness benchmark; requires cached real model"]
    fn preload_tui_benchmark() {
        use ratatui::backend::TestBackend;
        use std::{cell::{Cell,
                         RefCell},
                  time::Instant};
        if cfg!(debug_assertions) {
            panic!("run with --release");
        }
        let mut report = String::from("preload,first_frame_ms,schedule_ms,input_median_ms,input_p95_ms,input_max_ms,draw_max_ms,quit_ms,frames\n");
        for _ in 0 .. 3 {
            for enabled in [false, true] {
                let directory = TempDir::new().unwrap();
                let storage = Storage::with_dir(directory.path()).unwrap();
                let start = Instant::now();
                let mut model = Model::new(NaiveDate::default(), storage.scan_entries().unwrap());
                let mut runtime = Runtime::new(storage, None);
                runtime.execute(diary_application::Command::LoadPreview(model.today), &mut model);
                let mut terminal = Terminal::new(TestBackend::new(100, 28)).unwrap();
                let first = Cell::new(None);
                let scheduled = Cell::new(false);
                let schedule_ms = Cell::new(0.0);
                let pending_input = Cell::new(None::<Instant>);
                let latencies = RefCell::new(Vec::<f64>::new());
                let draws = RefCell::new(Vec::<f64>::new());
                let key = |code, modifiers| Some(Event::Key(KeyEvent::new(code, modifiers)));
                let mut events = vec![key(KeyCode::F(2), KeyModifiers::NONE); 40];
                events.extend([
                    Some(Event::Resize(100, 28)),
                    key(KeyCode::Enter, KeyModifiers::NONE),
                    key(KeyCode::Char('한'), KeyModifiers::NONE),
                    key(KeyCode::Char('x'), KeyModifiers::CONTROL),
                    key(KeyCode::Char('s'), KeyModifiers::CONTROL),
                    key(KeyCode::Char('x'), KeyModifiers::CONTROL),
                    key(KeyCode::Char('k'), KeyModifiers::NONE),
                    quit(),
                ]);
                let mut events = events.into_iter();
                run_app(
                    &mut model,
                    &mut runtime,
                    |model| {
                        let draw = Instant::now();
                        terminal.draw(|frame| app::view(frame, model)).unwrap();
                        draws.borrow_mut().push(draw.elapsed().as_secs_f64() * 1000.0);
                        if first.get().is_none() {
                            assert!(!scheduled.get());
                            first.set(Some(start.elapsed().as_secs_f64() * 1000.0));
                        }
                        if let Some(input) = pending_input.take() {
                            latencies.borrow_mut().push(input.elapsed().as_secs_f64() * 1000.0);
                        }
                        Ok(())
                    },
                    |_| {
                        std::thread::sleep(Duration::from_millis(5));
                        pending_input.set(Some(Instant::now()));
                        Ok(events.next().expect("unexpected poll"))
                    },
                    NaiveDate::default,
                    |runtime| {
                        assert!(first.get().is_some());
                        let schedule = Instant::now();
                        if enabled {
                            runtime.preload_search_model();
                        }
                        schedule_ms.set(schedule.elapsed().as_secs_f64() * 1000.0);
                        scheduled.set(true);
                    },
                )
                .unwrap();
                let quit_ms = pending_input.take().unwrap().elapsed().as_secs_f64() * 1000.0;
                assert!(model.should_quit);
                assert_eq!(runtime.storage.load(model.today).unwrap(), "한");
                let mut latencies = latencies.into_inner();
                latencies.sort_by(f64::total_cmp);
                let median = latencies[latencies.len() / 2];
                let p95 = latencies[latencies.len() * 95 / 100];
                let maximum = *latencies.last().unwrap();
                assert!(maximum < 100.0, "input delayed {maximum} ms");
                let draws = draws.into_inner();
                let draw_max = draws.iter().copied().fold(0.0, f64::max);
                report.push_str(&format!(
                    "{enabled},{},{},{median},{p95},{maximum},{draw_max},{quit_ms},{}\n",
                    first.get().unwrap(),
                    schedule_ms.get(),
                    draws.len()
                ));
                // Drain through search after measuring quit, so native loading
                // from one trial cannot overlap the next.
                app::update(&mut model, ratatui_diary::Msg::OpenSearch);
                app::update(&mut model, ratatui_diary::Msg::SearchChar('한'));
                let command = app::update(&mut model, ratatui_diary::Msg::SearchSubmit).unwrap();
                runtime.execute(command, &mut model);
                let deadline = Instant::now() + Duration::from_secs(30);
                while model.search.busy && Instant::now() < deadline {
                    runtime.poll(&mut model);
                    std::thread::sleep(Duration::from_millis(1));
                }
                assert!(!model.search.busy);
                assert!(model.search.notice.is_none(), "{:?}", model.search.notice);
                assert!(model.search.results[0].literal);
            }
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/preload-tui.csv");
        std::fs::write(path, &report).unwrap();
        println!("{report}");
    }

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
            |_| {},
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
            |_| {},
        )
        .unwrap();
        assert_eq!(frames, [date, tomorrow]);
        assert_eq!(model.calendar_state.selected_date, date);
    }
}
