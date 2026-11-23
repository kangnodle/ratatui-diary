use chrono::NaiveDate;
use crossterm::event::{KeyCode,
                       KeyEvent,
                       KeyModifiers};
use diary_application::{Command,
                        Screen,
                        search::{SearchHit,
                                 SearchOutput}};
use ratatui::{Terminal,
              backend::TestBackend};
use ratatui_diary::{Model,
                    Msg,
                    Runtime,
                    app,
                    settings::{Keybindings,
                               Language,
                               Theme},
                    storage::Storage};
use tempfile::TempDir;

fn press(model: &mut Model, code: KeyCode, modifiers: KeyModifiers) -> Option<Command> {
    let message = app::handle_key(KeyEvent::new(code, modifiers), model)?;
    app::update(model, message)
}

fn hit(date: NaiveDate) -> SearchHit {
    SearchHit {
        date,
        line: 3,
        col: 3,
        excerpt: "앞뒤 문맥\n나는 미래가 걱정된다.".into(),
        literal: false,
        anchor: "미래가 걱정된다.".into(),
    }
}

#[test]
fn both_keymaps_open_results_at_the_korean_source_position_and_save_normally() {
    for keybindings in [Keybindings::Emacs, Keybindings::Helix] {
        let directory = match TempDir::new() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
        };
        let storage = match Storage::with_dir(directory.path()) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
        };
        let date = match NaiveDate::from_ymd_opt(2024, 2, 29) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        };
        let source = "새 줄\r\n제목\n\r\n나는 미래가 걱정된다.\r\n끝";
        match storage.save(date, source) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not save the diary: {error:?}"),
        };
        let mut runtime = Runtime::new(storage, None);
        let mut model = Model::new(NaiveDate::default(), Default::default());
        model.settings.keybindings = keybindings;
        press(&mut model, KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(model.screen, Screen::Search);
        for ch in "진로 불안".chars() {
            press(&mut model, KeyCode::Char(ch), KeyModifiers::NONE);
        }
        let command = match press(&mut model, KeyCode::Enter, KeyModifiers::NONE) {
            | Some(value) => value,
            | None => panic!("Expected the key press to produce a command"),
        };
        assert!(matches!(command, Command::Search(_, _)));
        assert!(model.search.is_busy());
        let mut result = hit(date);
        result.line = 2;
        let revision = model.search.pending_revision().expect("expected a pending search");
        app::update(
            &mut model,
            Msg::SearchFinished(
                revision,
                Ok(SearchOutput {
                    results: vec![result],
                    notice: None,
                }),
            ),
        );
        let command = match press(&mut model, KeyCode::Enter, KeyModifiers::NONE) {
            | Some(value) => value,
            | None => panic!("Expected the key press to produce a command"),
        };
        runtime.execute(command, &mut model);
        assert_eq!(model.screen, Screen::Editor);
        assert_eq!((model.editor_state.cursor_line, model.editor_state.cursor_col), (3, 3));
        assert_eq!(model.calendar_state.selected_date, date);
        assert_eq!((model.calendar_state.current_year, model.calendar_state.current_month), (2024, 2));
        assert!(!model.editor_state.is_modified);
        if keybindings == Keybindings::Helix {
            assert_eq!(
                (match model.editor_state.selection.as_ref() {
                    | Some(value) => value,
                    | None => panic!("Expected selection to be present"),
                })
                .anchor_line,
                3
            );
        }
        model.editor_state.insert_char('!');
        let command = match app::update(&mut model, Msg::EditorCtrlXBack) {
            | Some(value) => value,
            | None => panic!("Expected the application update to produce a command"),
        };
        runtime.execute(command, &mut model);
        assert_eq!(model.screen, Screen::Calendar);
        assert!(
            (match runtime.storage.load(date) {
                | Ok(value) => value,
                | Err(error) => panic!("Could not load the diary: {error:?}"),
            })
            .contains("나는 !미래")
        );
    }
}

#[test]
fn edits_and_closing_discard_stale_replies_and_search_does_not_intercept_editor_typing() {
    let mut model = Model::new(NaiveDate::default(), Default::default());
    press(&mut model, KeyCode::Char('/'), KeyModifiers::NONE);
    press(&mut model, KeyCode::Char('가'), KeyModifiers::NONE);
    press(&mut model, KeyCode::Enter, KeyModifiers::NONE);
    let revision = model.search.pending_revision().expect("expected a pending search");
    press(&mut model, KeyCode::Char('나'), KeyModifiers::NONE);
    app::update(&mut model, Msg::SearchFinished(revision, Err("stale".into())));
    assert!(model.search.notice.is_none());
    assert!(!model.search.is_busy());
    press(&mut model, KeyCode::Enter, KeyModifiers::NONE);
    let revision = model.search.pending_revision().expect("expected a pending search");
    press(&mut model, KeyCode::Esc, KeyModifiers::NONE);
    let date = model.today;
    app::update(
        &mut model,
        Msg::SearchFinished(
            revision,
            Ok(SearchOutput {
                results: vec![hit(date)],
                notice: None,
            }),
        ),
    );
    assert_eq!(model.screen, Screen::Calendar);
    assert!(model.search.results.is_empty());
    model.screen = Screen::Editor;
    press(&mut model, KeyCode::Char('/'), KeyModifiers::NONE);
    assert_eq!(model.editor_state.get_content(), "/");
}

#[test]
fn a_deleted_or_changed_passage_never_opens_an_empty_or_unrelated_diary() {
    for content in [None, Some("완전히 달라진 일기")] {
        let directory = match TempDir::new() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
        };
        let storage = match Storage::with_dir(directory.path()) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
        };
        let date = NaiveDate::default();
        if let Some(content) = content {
            match storage.save(date, content) {
                | Ok(_) => (),
                | Err(error) => panic!("Could not save the diary: {error:?}"),
            };
        }
        let mut runtime = Runtime::new(storage, None);
        let mut model = Model::new(date, Default::default());
        model.screen = Screen::Search;
        model.search.results = vec![hit(date)];
        let command = match app::update(&mut model, Msg::SearchSubmit) {
            | Some(value) => value,
            | None => panic!("Expected the application update to produce a command"),
        };
        runtime.execute(command, &mut model);
        assert!(model.show_error_popup);
        assert_eq!(model.screen, Screen::Calendar);
        assert!(!model.editor_state.is_modified);
    }
}

#[test]
fn search_renders_results_loading_errors_and_small_terminals_in_both_languages() {
    for language in [Language::Korean, Language::English] {
        for theme in [Theme::System, Theme::Dark, Theme::Light] {
            for (width, height) in [(100, 28), (40, 12), (1, 1)] {
                let mut model = Model::new(NaiveDate::default(), Default::default());
                model.settings.language = language;
                model.settings.theme = theme;
                model.screen = Screen::Search;
                model.search.query = "긴 한글 검색어 ".repeat(20);
                model.search.editing = true;
                let mut terminal = match Terminal::new(TestBackend::new(width, height)) {
                    | Ok(value) => value,
                    | Err(never) => match never {},
                };
                for phase in 0 .. 3 {
                    if phase == 0 {
                        app::update(&mut model, Msg::SearchSubmit);
                    }
                    if phase == 1 {
                        let revision = model.search.pending_revision().expect("expected a pending search");
                        app::update(&mut model, Msg::SearchFinished(revision, Ok(SearchOutput::default())));
                    }
                    if phase == 1 {
                        model.search.results = vec![hit(model.today)];
                    }
                    if phase == 2 {
                        model.search.notice = Some("Semantic search unavailable · literal results only".into());
                    }
                    match terminal.draw(|frame| app::view(frame, &model)) {
                        | Ok(_) => (),
                        | Err(never) => match never {},
                    };
                    if width == 100 && phase == 1 {
                        let text: String = terminal.backend().buffer().content.iter().map(|c| c.symbol()).collect();
                        assert!(text.chars().filter(|c| !c.is_whitespace()).collect::<String>().contains("미래가걱정된다"));
                        assert!(text.contains(&model.today.to_string()));
                    }
                }
            }
        }
    }
}
