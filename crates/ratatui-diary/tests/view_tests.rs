mod common;
use crate::common::TestApp;
use chrono::NaiveDate;
use ratatui_diary::{model::{EditorSubMode,
                            Screen,
                            Selection},
                    storage::Storage};
use std::collections::HashSet;
use tempfile::TempDir;

#[test]
fn test_calendar_preview_loads_from_storage() {
    let temp_dir = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp_dir.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let test_date = match NaiveDate::from_ymd_opt(2026, 2, 14) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };

    let _ = storage.save(test_date, "테스트 다이어리 내용");

    let entries = match storage.scan_entries() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not list the diary entries: {error:?}"),
    };
    let mut model = TestApp::new(entries, storage);

    model.calendar_state.selected_date = test_date;

    let content = model.runtime.storage.load(test_date);

    assert!(content.is_ok());
    assert_eq!(
        match content {
            | Ok(value) => value,
            | Err(error) => panic!("Could not read the diary content: {error:?}"),
        },
        "테스트 다이어리 내용"
    );
}

#[test]
fn test_calendar_preview_shows_empty_message() {
    let temp_dir = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp_dir.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let test_date = match NaiveDate::from_ymd_opt(2026, 1, 1) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };

    let entries = HashSet::new();
    let model = TestApp::new(entries, storage);

    let content = model.runtime.storage.load(test_date);

    assert!(content.is_err());
}

#[test]
fn test_editor_content_updates_preview() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = HashSet::new();

    let mut model = TestApp::new(entries, storage);

    model.screen = Screen::Editor;

    model.editor_state.insert_char('#');
    model.editor_state.insert_char(' ');
    model.editor_state.insert_char('H');

    let content = model.editor_state.get_content();
    assert_eq!(content, "# H");
}

#[test]
fn test_calendar_preview_empty_diary() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = HashSet::new();

    let model = TestApp::new(entries, storage);

    let date = model.calendar_state.selected_date;
    let result = model.runtime.storage.load(date);

    assert!(result.is_err());
}

#[test]
fn test_editor_selection_highlight_state() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = HashSet::new();

    let mut model = TestApp::new(entries, storage);
    model.screen = Screen::Editor;

    model.editor_state.content = vec!["Hello World".to_string(), "Rust is great".to_string()];

    model.editor_state.selection = Some(Selection {
        anchor_line: 0,
        anchor_col: 0,
        cursor_line: 0,
        cursor_col: 5,
    });

    let range = model.editor_state.get_selection_range();
    assert!(range.is_some());
    let ((start_line, start_col), (end_line, end_col)) = match range {
        | Some(value) => value,
        | None => panic!("Expected a valid selection range"),
    };
    assert_eq!(start_line, 0);
    assert_eq!(start_col, 0);
    assert_eq!(end_line, 0);
    assert_eq!(end_col, 5);

    let selected_text = model.editor_state.get_selected_text();
    assert_eq!(selected_text, Some("Hello".to_string()));
}

#[test]
fn test_editor_search_matches_state() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = HashSet::new();

    let mut model = TestApp::new(entries, storage);
    model.screen = Screen::Editor;

    model.editor_state.content = vec!["test text test".to_string(), "another test".to_string()];

    model.editor_state.search_pattern = "test".to_string();
    model.editor_state.execute_search();

    assert_eq!(model.editor_state.search_matches.len(), 3);
    assert_eq!(model.editor_state.search_matches[0], (0, 0));
    assert_eq!(model.editor_state.search_matches[1], (0, 10));
    assert_eq!(model.editor_state.search_matches[2], (1, 8));

    assert_eq!(model.editor_state.current_match_index, 0);

    let selection = match model.editor_state.selection.as_ref() {
        | Some(value) => value,
        | None => panic!("Expected selection to be present"),
    };
    assert_eq!(selection.cursor_col - selection.anchor_col, 4);
}

#[test]
fn test_editor_submode_display_state() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = HashSet::new();

    let mut model = TestApp::new(entries, storage);
    model.screen = Screen::Editor;

    model.editor_state.submode = Some(EditorSubMode::CtrlX);

    assert_eq!(model.editor_state.submode, Some(EditorSubMode::CtrlX));

    model.editor_state.submode = Some(EditorSubMode::Search);
    model.editor_state.search_pattern = "test".to_string();

    assert_eq!(model.editor_state.submode, Some(EditorSubMode::Search));
    assert_eq!(model.editor_state.search_pattern, "test");
}

#[test]
fn test_editor_multi_line_selection_highlight() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = HashSet::new();

    let mut model = TestApp::new(entries, storage);
    model.screen = Screen::Editor;

    model.editor_state.content = vec!["First line".to_string(), "Second line".to_string(), "Third line".to_string()];

    model.editor_state.selection = Some(Selection {
        anchor_line: 0,
        anchor_col: 6,
        cursor_line: 2,
        cursor_col: 5,
    });

    let range = model.editor_state.get_selection_range();
    assert!(range.is_some());

    let selected_text = model.editor_state.get_selected_text();
    assert!(selected_text.is_some());
    let text = match selected_text {
        | Some(value) => value,
        | None => panic!("Expected nonempty editor selection"),
    };

    assert!(text.starts_with("line"));
    assert!(text.contains("Second line"));
    assert!(text.ends_with("Third"));
}

#[test]
fn test_editor_search_navigation_updates_selection() {
    let temp = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(temp.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let entries = HashSet::new();

    let mut model = TestApp::new(entries, storage);
    model.screen = Screen::Editor;

    model.editor_state.content = vec!["test abc test".to_string(), "test def".to_string()];

    model.editor_state.search_pattern = "test".to_string();
    model.editor_state.execute_search();

    let first_selection = model.editor_state.selection.clone();
    model.editor_state.search_next();

    let second_selection = model.editor_state.selection.clone();
    assert!(first_selection != second_selection);

    assert_eq!(model.editor_state.current_match_index, 1);

    model.editor_state.search_prev();

    assert_eq!(model.editor_state.current_match_index, 0);
}

#[cfg(test)]
mod view_rendering_complete {
    use crate::common::TestApp;
    use chrono::NaiveDate;
    use ratatui::{Terminal,
                  backend::TestBackend};
    use ratatui_diary::{model::{EditorSubMode,
                                Screen,
                                Selection},
                        storage::Storage,
                        view};
    use std::collections::HashSet;
    use tempfile::TempDir;

    fn create_test_model() -> (TempDir, TestApp) {
        let temp = match TempDir::new() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
        };
        let storage = match Storage::with_dir(temp.path()) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
        };
        (temp, TestApp::new(HashSet::new(), storage))
    }

    fn setup_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        match Terminal::new(backend) {
            | Ok(value) => value,
            | Err(never) => match never {},
        }
    }

    #[test]
    fn test_render_calendar_view() {
        let (_temp, model) = create_test_model();
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_view() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "에디터 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_with_error_popup() {
        let (_temp, mut model) = create_test_model();
        model.show_error_popup = true;
        model.error_message = Some("테스트 에러".to_string());
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "에러 팝업 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_small_terminal() {
        let backend = TestBackend::new(10, 5);
        let mut terminal = match Terminal::new(backend) {
            | Ok(value) => value,
            | Err(never) => match never {},
        };
        let (_temp, model) = create_test_model();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "작은 터미널 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_large_terminal() {
        let backend = TestBackend::new(200, 50);
        let mut terminal = match Terminal::new(backend) {
            | Ok(value) => value,
            | Err(never) => match never {},
        };
        let (_temp, model) = create_test_model();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "큰 터미널 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_with_selection() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec!["Hello World".to_string()];
        model.editor_state.selection = Some(Selection {
            anchor_line: 0,
            anchor_col: 0,
            cursor_line: 0,
            cursor_col: 5,
        });
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "선택 영역 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_with_search_matches() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec!["test test test".to_string()];
        model.editor_state.search_pattern = "test".to_string();
        model.editor_state.execute_search();
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "검색 매치 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_calendar_with_entries() {
        let (_temp, mut model) = create_test_model();
        let date = chrono::Local::now().date_naive();
        model.diary_entries.entries.insert(date);
        model.screen = Screen::Calendar;
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "일기 엔트리 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_emacs_mode() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "Emacs 모드 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_ctrlx_submode() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.submode = Some(EditorSubMode::CtrlX);
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "CtrlX 서브모드 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_search_submode() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.submode = Some(EditorSubMode::Search);
        model.editor_state.search_pattern = "test".to_string();
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "Search 서브모드 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_with_markdown_content() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec!["# 헤더 1".to_string(), "**굵은 텍스트**".to_string(), "- 리스트 항목".to_string()];
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "Markdown 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_with_long_lines() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec!["a".repeat(200)];
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "긴 라인 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_with_multiple_lines() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = (0 .. 50).map(|i| format!("라인 {}", i)).collect();
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "여러 라인 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_multiline_selection() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec!["첫 번째 라인".to_string(), "두 번째 라인".to_string(), "세 번째 라인".to_string()];
        model.editor_state.selection = Some(Selection {
            anchor_line: 0,
            anchor_col: 2,
            cursor_line: 2,
            cursor_col: 4,
        });
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "여러 줄 선택 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_calendar_different_months() {
        let (_temp, mut model) = create_test_model();
        model.calendar_state.current_month = 12;
        model.calendar_state.selected_date = match NaiveDate::from_ymd_opt(2026, 12, 25) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        };
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "다른 달 달력 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_empty_content() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec![];
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "빈 에디터 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_default_mode() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec!["테스트 콘텐츠".to_string()];
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "기본 모드 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_with_mixed_content() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec!["# Header".to_string(), "search test".to_string(), "another test".to_string()];
        model.editor_state.search_pattern = "test".to_string();
        model.editor_state.execute_search();
        model.editor_state.selection = Some(Selection {
            anchor_line: 1,
            anchor_col: 0,
            cursor_line: 1,
            cursor_col: 6,
        });
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "혼합 콘텐츠 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_editor_with_unicode_content() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec!["한글 테스트".to_string(), "日本語テスト".to_string(), "Unicode: café 𠮷".to_string()];
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "유니코드 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_error_popup_text() {
        let (_temp, mut model) = create_test_model();
        model.show_error_popup = true;
        model.error_message = Some("이것은 매우 긴 에러 메시지입니다. 여러 줄로 나뉠 수 있으며 매우 상세한 정보를 포함합니다.".to_string());
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "에러 팝업 텍스트 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_with_cursor_positioning() {
        let (_temp, mut model) = create_test_model();
        model.screen = Screen::Editor;
        model.editor_state.content = vec!["라인 1".to_string(), "라인 2".to_string()];
        model.editor_state.cursor_line = 1;
        model.editor_state.cursor_col = 3;
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "커서 위치 렌더링 버퍼가 비어있음");
    }

    #[test]
    fn test_render_calendar_complete_last_week() {
        let (_temp, mut model) = create_test_model();
        model.calendar_state.current_year = 2026;
        model.calendar_state.current_month = 1;
        model.calendar_state.selected_date = match NaiveDate::from_ymd_opt(2026, 1, 1) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        };
        model.screen = Screen::Calendar;
        let mut terminal = setup_terminal();

        match terminal.draw(|f| {
            view::view(f, &model);
        }) {
            | Ok(_) => (),
            | Err(never) => match never {},
        };

        let buffer = terminal.backend().buffer();
        assert!(!buffer.content.is_empty(), "완전한 마지막 주 달력 렌더링 버퍼가 비어있음");
    }
}

#[cfg(test)]
mod keybinding_tests {
    use chrono::NaiveDate;
    use ratatui_diary::{model::{CalendarState,
                                EditorState,
                                EditorSubMode},
                        view::{build_calendar_keybindings,
                               build_editor_keybindings}};

    #[test]
    fn test_build_calendar_keybindings() {
        let state = CalendarState::new(2026, 2);
        let result = build_calendar_keybindings(&state);
        assert_eq!(
            result,
            "C-b/f/p/n:이동 | Enter:편집\nM-p/n · PgUp/Dn:이전/다음 월\nM-[/] · C-PgUp/Dn:이전/다음 년\nC-x C-c:종료 | C-g:취소"
        );
    }

    #[test]
    fn test_build_editor_keybindings_default() {
        let date = match NaiveDate::from_ymd_opt(2026, 2, 15) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        };
        let state = EditorState::new(date);
        let result = build_editor_keybindings(&state);
        assert_eq!(result, "C-x:명령 | C-g:취소 | C-s/r:검색 | C-/:실행취소 | M-d/BS:단어삭제");
    }

    #[test]
    fn test_build_editor_keybindings_ctrlx() {
        let date = match NaiveDate::from_ymd_opt(2026, 2, 15) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        };
        let mut state = EditorState::new(date);
        state.submode = Some(EditorSubMode::CtrlX);
        let result = build_editor_keybindings(&state);
        assert_eq!(result, "C-s/s:저장 | k:저장 후 달력 | C-c:저장 후 종료 | u:실행취소 | C-g:취소");
    }

    #[test]
    fn test_build_editor_keybindings_search() {
        let date = match NaiveDate::from_ymd_opt(2026, 2, 15) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        };
        let mut state = EditorState::new(date);
        state.submode = Some(EditorSubMode::Search);
        let result = build_editor_keybindings(&state);
        assert_eq!(result, "입력:즉시 검색 | C-s/r:다음/이전 | Enter:확정 | C-g:취소·위치복원");
    }
}
