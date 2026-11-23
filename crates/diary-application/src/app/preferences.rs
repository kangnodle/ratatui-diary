//! Applies preferences and coordinates the editor's keybinding mode.
use super::{Command,
            Model};
use crate::{Msg,
            editor::helix,
            i18n::Text,
            settings::{Keybindings,
                       Language,
                       Settings,
                       Theme}};

pub(crate) fn initialize(model: &mut Model, result: Result<Settings, String>) {
    match result {
        | Ok(settings) => {
            model.settings = settings;
            if settings.keybindings == Keybindings::Helix {
                helix::reset(&mut model.editor_state, true);
            }
        },
        | Err(error) => {
            model.error_message = Some(format!("설정 읽기 실패 / Failed to load settings: {error}"));
            model.show_error_popup = true;
        },
    }
}

pub(super) fn interact(model: &mut Model, message: Msg) -> Option<Command> {
    match message {
        | Msg::ToggleSettings => model.show_settings = !model.show_settings,
        | Msg::SettingsMove => model.settings_row = (model.settings_row + 1) % 3,
        | Msg::SettingsMoveBack => model.settings_row = (model.settings_row + 2) % 3,
        | Msg::SettingsChange(forward) => {
            match model.settings_row {
                | 0 =>
                    model.settings.language = match model.settings.language {
                        | Language::Korean => Language::English,
                        | Language::English => Language::Korean,
                    },
                | 1 =>
                    model.settings.theme = match (model.settings.theme, forward) {
                        | (Theme::System, true) | (Theme::Light, false) => Theme::Dark,
                        | (Theme::Dark, true) | (Theme::System, false) => Theme::Light,
                        | _ => Theme::System,
                    },
                | _ => {
                    model.settings.keybindings = match model.settings.keybindings {
                        | Keybindings::Emacs => Keybindings::Helix,
                        | Keybindings::Helix => Keybindings::Emacs,
                    };
                    helix::reset(&mut model.editor_state, model.settings.keybindings == Keybindings::Helix);
                },
            }
            return Some(Command::SaveSettings(model.settings));
        },
        | Msg::SettingsSaveFailed(error) => model.report_error(Text::FailedToSaveSettings, error),
        | _ => {},
    }
    None
}
