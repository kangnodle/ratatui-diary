use diary_domain::{calendar::CalendarState,
                   settings::{Keybindings,
                              Language,
                              Settings,
                              Theme}};

#[test]
fn preferences_round_trip_without_a_filesystem() {
    for language in [Language::Korean, Language::English] {
        for theme in [Theme::System, Theme::Dark, Theme::Light] {
            for keybindings in [Keybindings::Emacs, Keybindings::Helix] {
                let settings = Settings {
                    language,
                    theme,
                    keybindings,
                };
                assert_eq!(Settings::parse(&settings.serialize()).unwrap(), settings);
            }
        }
    }
    assert_eq!(
        Settings::parse("# old config\nlanguage=en\ntheme=dark\n").unwrap().keybindings,
        Keybindings::Emacs
    );
    for source in ["theme=invalid", "language=kr", "keybindings=vim", "missing-equals", "unknown=value"] {
        assert!(Settings::parse(source).is_err());
    }
}

#[test]
fn invalid_calendar_input_has_a_deterministic_valid_fallback() {
    let calendar = CalendarState::new(2026, 13);
    assert_eq!(calendar.selected_date, Default::default());
    assert_eq!(calendar.current_year, 1970);
    assert_eq!(calendar.current_month, 1);
}
