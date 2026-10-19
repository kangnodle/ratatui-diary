use crossterm::{event::{self,
                        Event},
                execute,
                terminal::{EnterAlternateScreen,
                           LeaveAlternateScreen,
                           disable_raw_mode,
                           enable_raw_mode}};
use ratatui::{Terminal,
              backend::CrosstermBackend,
              prelude::*};
use ratatui_diary::{Model,
                    app,
                    storage::Storage};
use std::{io,
          time::Duration};

fn main() -> std::io::Result<()> {
    // Storage 초기화
    let storage = Storage::new()?;
    let entries = storage.scan_entries()?;

    // Model 초기화
    let mut model = Model::new(entries, storage);

    match ratatui_diary::settings::Settings::path() {
        | Ok(path) => {
            match ratatui_diary::settings::Settings::load(&path) {
                | Ok(settings) => model.settings = settings,
                | Err(e) => {
                    model.error_message = Some(format!("설정 읽기 실패 / Failed to load settings: {e}"));
                    model.show_error_popup = true;
                },
            }
            model.settings_path = Some(path);
        },
        | Err(e) => {
            model.error_message = Some(e.to_string());
            model.show_error_popup = true;
        },
    }

    // Terminal 초기화
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // 이벤트 루프
    let result = run_app(&mut terminal, &mut model);

    // Terminal 복원
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ratatui 0.30부터 Backend는 연관 타입 `Error`를 가진다. `?`로 io::Result에
// 합치기 위해 io::Error를 쓰는 백엔드로 한정한다 (CrosstermBackend가 해당).
fn run_app<B: Backend<Error = io::Error>>(terminal: &mut Terminal<B>, model: &mut Model) -> std::io::Result<()> {
    loop {
        // 렌더링
        terminal.draw(|f| app::view(f, model))?;

        // 이벤트 처리
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && let Some(msg) = app::handle_key(key, model)
        {
            // Update 호출
            if let Some(cmd) = app::update(model, msg) {
                app::execute_command(cmd, model)?;
            }
            if model.should_quit {
                break;
            }
        }
    }

    Ok(())
}
