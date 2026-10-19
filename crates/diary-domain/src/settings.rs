//! Preferences and their deterministic text representation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    #[default]
    Korean,
    English,
}
impl Language {
    pub fn text<'a>(self, ko: &'a str, en: &'a str) -> &'a str {
        match self {
            | Self::Korean => ko,
            | Self::English => en,
        }
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}
impl Theme {
    pub fn label(self, language: Language) -> &'static str {
        use crate::i18n::Text;
        language.tr(match self {
            | Self::System => Text::System,
            | Self::Dark => Text::Dark,
            | Self::Light => Text::Light,
        })
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Keybindings {
    #[default]
    Emacs,
    Helix,
}
impl Keybindings {
    pub fn name(self) -> &'static str {
        match self {
            | Self::Emacs => "Emacs",
            | Self::Helix => "Helix",
        }
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub language: Language,
    pub theme: Theme,
    pub keybindings: Keybindings,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSettings;
impl std::fmt::Display for InvalidSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("Invalid settings entry") }
}
impl std::error::Error for InvalidSettings {}
impl Settings {
    pub fn parse(source: &str) -> Result<Self, InvalidSettings> {
        let mut settings = Self::default();
        for line in source.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                | Some(("language", "ko")) => settings.language = Language::Korean,
                | Some(("language", "en")) => settings.language = Language::English,
                | Some(("theme", "system")) => settings.theme = Theme::System,
                | Some(("theme", "dark")) => settings.theme = Theme::Dark,
                | Some(("theme", "light")) => settings.theme = Theme::Light,
                | Some(("keybindings", "emacs")) => settings.keybindings = Keybindings::Emacs,
                | Some(("keybindings", "helix")) => settings.keybindings = Keybindings::Helix,
                | _ => return Err(InvalidSettings),
            }
        }
        Ok(settings)
    }

    pub fn serialize(self) -> String {
        let language = self.language.text("ko", "en");
        let theme = match self.theme {
            | Theme::System => "system",
            | Theme::Dark => "dark",
            | Theme::Light => "light",
        };
        let keybindings = self.keybindings.name().to_ascii_lowercase();
        format!("language={language}\ntheme={theme}\nkeybindings={keybindings}\n")
    }
}
