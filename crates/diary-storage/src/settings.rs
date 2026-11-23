use crate::file_io;
use diary_domain::settings::Settings;
use std::{io,
          path::{Path,
                 PathBuf}};

pub const MAX_SETTINGS_BYTES: usize = 64 * 1024;

pub fn path() -> io::Result<PathBuf> {
    dirs::config_dir()
        .map(|p| p.join("ratatui-diary/settings.conf"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Cannot find config directory"))
}

pub fn load(path: &Path) -> io::Result<Settings> {
    let source = match file_io::read_text(path, MAX_SETTINGS_BYTES) {
        | Ok(source) => source,
        | Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
        | Err(error) => return Err(error),
    };
    Settings::parse(&source).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub fn save(settings: Settings, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        file_io::create_directory(parent)?;
    }
    file_io::atomic_write(path, &settings.serialize(), MAX_SETTINGS_BYTES)
}
