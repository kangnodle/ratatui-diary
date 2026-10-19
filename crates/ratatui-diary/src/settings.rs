//! Persisted UI preferences. System colors inherit the terminal palette.
use std::{fs,
          io,
          path::{Path,
                 PathBuf}};

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
pub use crate::theme::{Palette,
                       Theme};
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
impl Settings {
    pub fn path() -> io::Result<PathBuf> {
        dirs::config_dir()
            .map(|p| p.join("ratatui-diary/settings.conf"))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Cannot find config directory"))
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let source = match fs::read_to_string(path) {
            | Ok(s) => s,
            | Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            | Err(e) => return Err(e),
        };
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
                | _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid settings entry")),
            }
        }
        Ok(settings)
    }

    pub fn save(self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let language = self.language.text("ko", "en");
        let theme = match self.theme {
            | Theme::System => "system",
            | Theme::Dark => "dark",
            | Theme::Light => "light",
        };
        let keybindings = self.keybindings.name().to_ascii_lowercase();
        fs::write(path, format!("language={language}\ntheme={theme}\nkeybindings={keybindings}\n"))
    }
}
