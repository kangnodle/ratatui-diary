//! Filesystem adapter for the domain's preferences format.
use diary_domain::settings::Settings;
use std::{fs,
          io,
          path::{Path,
                 PathBuf}};

pub fn path() -> io::Result<PathBuf> {
    dirs::config_dir()
        .map(|p| p.join("ratatui-diary/settings.conf"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Cannot find config directory"))
}

pub fn load(path: &Path) -> io::Result<Settings> {
    let source = match fs::read_to_string(path) {
        | Ok(source) => source,
        | Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
        | Err(error) => return Err(error),
    };
    Settings::parse(&source).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub fn save(settings: Settings, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, settings.serialize())
}
