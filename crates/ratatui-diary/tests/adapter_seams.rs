use chrono::NaiveDate;
use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyEventKind,
                       KeyModifiers};
use diary_application::{Command,
                        Model,
                        input::{KeyCode as LogicalCode,
                                KeyModifiers as LogicalModifiers}};
use ratatui::{Terminal,
              backend::TestBackend};
use ratatui_diary::{Runtime,
                    app,
                    input,
                    storage::Storage};
use tempfile::TempDir;

#[test]
fn rendering_uses_the_loaded_snapshot_even_after_the_file_is_removed() {
    let directory = TempDir::new().unwrap();
    let storage = Storage::with_dir(directory.path()).unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    storage.save(date, "* Cached preview").unwrap();
    let mut model = Model::new(date, storage.scan_entries().unwrap());
    let mut runtime = Runtime::new(storage, None);
    runtime.execute(Command::LoadPreview(date), &mut model);
    std::fs::remove_file(directory.path().join(format!("entries/{date}.org"))).unwrap();

    let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
    terminal.draw(|frame| app::view(frame, &model)).unwrap();
    let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("◉ Cached preview"));
}

#[test]
fn terminal_adapter_preserves_modifiers_and_repeat_but_filters_release() {
    let modifiers = KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::SUPER;
    let key = input::translate(KeyEvent::new_with_kind(KeyCode::Char('한'), modifiers, KeyEventKind::Repeat)).unwrap();
    assert_eq!(key.code, LogicalCode::Char('한'));
    assert_eq!(
        key.modifiers,
        LogicalModifiers::CONTROL | LogicalModifiers::ALT | LogicalModifiers::SHIFT | LogicalModifiers::OTHER
    );
    assert!(input::translate(KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release)).is_none());
    assert_eq!(
        input::translate(KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE)).unwrap().code,
        LogicalCode::Unknown
    );
}
