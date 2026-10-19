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
    let result = run_app(&mut terminal, &mut model, &mut runtime);

    // Terminal 복원
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ratatui 0.30부터 Backend는 연관 타입 `Error`를 가진다. `?`로 io::Result에
// 합치기 위해 io::Error를 쓰는 백엔드로 한정한다 (CrosstermBackend가 해당).
fn run_app<B: Backend<Error = io::Error>>(terminal: &mut Terminal<B>, model: &mut Model, runtime: &mut Runtime) -> std::io::Result<()> {
    loop {
        model.today = chrono::Local::now().date_naive();
        model.editor_state.viewport_height = usize::from(terminal.size()?.height.saturating_sub(4).max(1));
        // 렌더링
        terminal.draw(|f| app::view(f, model))?;

        // 이벤트 처리
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && let Some(msg) = app::handle_key(key, model)
        {
            // Update 호출
            if let Some(cmd) = app::update(model, msg) {
                runtime.execute(cmd, model);
            }
            if model.should_quit {
                break;
            }
        }
    }

    Ok(())
}
