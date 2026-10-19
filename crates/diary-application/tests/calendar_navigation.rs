use chrono::{Datelike,
             NaiveDate};
use diary_application::{app::{Command,
                              Model,
                              handle_key,
                              update},
                        input::{KeyCode,
                                KeyEvent,
                                KeyModifiers},
                        settings::Keybindings};

fn press(model: &mut Model, code: KeyCode, modifiers: KeyModifiers) -> Option<Command> {
    handle_key(KeyEvent::new(code, modifiers), model).and_then(|msg| update(model, msg))
}

fn assert_date(model: &Model, expected: NaiveDate) {
    assert_eq!(model.calendar_state.selected_date, expected);
    assert_eq!(model.calendar_state.current_year, expected.year());
    assert_eq!(model.calendar_state.current_month, expected.month());
}

#[test]
fn cursor_navigation_follows_month_and_year_and_loads_the_selected_preview() {
    for keybindings in [Keybindings::Emacs, Keybindings::Helix] {
        for (start, code, end) in [
            ((2026, 1, 1), KeyCode::Left, (2025, 12, 31)),
            ((2025, 12, 31), KeyCode::Right, (2026, 1, 1)),
            ((2024, 3, 1), KeyCode::Up, (2024, 2, 23)),
            ((2024, 2, 28), KeyCode::Down, (2024, 3, 6)),
        ] {
            let mut model = Model::new(NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(), Default::default());
            model.settings.keybindings = keybindings;
            let expected = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();
            assert!(matches!(press(&mut model, code, KeyModifiers::NONE), Some(Command::LoadPreview(date)) if date == expected));
            assert_date(&model, expected);
        }
    }
}

#[test]
fn page_keys_change_month_and_year_in_both_keymaps_and_clamp_short_months() {
    for keybindings in [Keybindings::Emacs, Keybindings::Helix] {
        for (start, code, modifiers, end) in [
            ((2024, 1, 31), KeyCode::PageDown, KeyModifiers::NONE, (2024, 2, 29)),
            ((2025, 3, 31), KeyCode::PageUp, KeyModifiers::NONE, (2025, 2, 28)),
            ((2025, 12, 31), KeyCode::PageDown, KeyModifiers::NONE, (2026, 1, 31)),
            ((2026, 1, 31), KeyCode::PageUp, KeyModifiers::NONE, (2025, 12, 31)),
            ((2024, 2, 29), KeyCode::PageDown, KeyModifiers::CONTROL, (2025, 2, 28)),
            ((2024, 2, 29), KeyCode::PageUp, KeyModifiers::CONTROL, (2023, 2, 28)),
            ((2023, 10, 19), KeyCode::PageDown, KeyModifiers::CONTROL, (2024, 10, 19)),
        ] {
            let mut model = Model::new(NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(), Default::default());
            model.settings.keybindings = keybindings;
            let expected = NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap();
            assert!(matches!(press(&mut model, code, modifiers), Some(Command::LoadPreview(date)) if date == expected));
            assert_date(&model, expected);
        }
    }
}

#[test]
fn calendar_navigation_stays_in_sync_at_date_limits() {
    for (date, code, modifiers) in [
        (NaiveDate::MIN, KeyCode::Left, KeyModifiers::NONE),
        (NaiveDate::MIN, KeyCode::Up, KeyModifiers::NONE),
        (NaiveDate::MIN, KeyCode::PageUp, KeyModifiers::NONE),
        (NaiveDate::MIN, KeyCode::PageUp, KeyModifiers::CONTROL),
        (NaiveDate::MAX, KeyCode::Right, KeyModifiers::NONE),
        (NaiveDate::MAX, KeyCode::Down, KeyModifiers::NONE),
        (NaiveDate::MAX, KeyCode::PageDown, KeyModifiers::NONE),
        (NaiveDate::MAX, KeyCode::PageDown, KeyModifiers::CONTROL),
    ] {
        let mut model = Model::new(date, Default::default());
        press(&mut model, code, modifiers);
        assert_date(&model, date);
    }
}

#[test]
fn page_navigation_does_not_bypass_overlays_or_command_prefixes() {
    let date = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
    for keybindings in [Keybindings::Emacs, Keybindings::Helix] {
        let mut model = Model::new(date, Default::default());
        model.settings.keybindings = keybindings;
        model.show_settings = true;
        assert!(press(&mut model, KeyCode::PageDown, KeyModifiers::NONE).is_none());
        model.show_settings = false;
        model.show_error_popup = true;
        assert!(press(&mut model, KeyCode::PageDown, KeyModifiers::CONTROL).is_none());
        model.show_error_popup = false;
        match keybindings {
            | Keybindings::Emacs => {
                press(&mut model, KeyCode::Char('x'), KeyModifiers::CONTROL);
            },
            | Keybindings::Helix => {
                press(&mut model, KeyCode::Char(':'), KeyModifiers::NONE);
            },
        }
        assert!(press(&mut model, KeyCode::PageDown, KeyModifiers::NONE).is_none());
        assert_date(&model, date);
    }
}
