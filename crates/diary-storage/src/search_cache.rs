use crate::{Storage,
            file_io};
use chrono::NaiveDate;
use std::{collections::HashSet,
          fs,
          io};

const MAX_CACHE_BYTES: usize = 128 * 1024 * 1024;

impl Storage {
    pub fn load_search_cache(&self, date: NaiveDate) -> io::Result<Option<String>> {
        file_io::check_directory(&self.entries_dir)?;
        let directory = self.entries_dir.join(".search-cache");
        match file_io::check_directory(&directory) {
            | Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            | result => result?,
        }
        match file_io::read_text(&directory.join(format!("{date}.json")), MAX_CACHE_BYTES) {
            | Ok(source) => Ok(Some(source)),
            | Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            | Err(error) => Err(error),
        }
    }

    pub fn save_search_cache(&self, date: NaiveDate, source: &str) -> io::Result<()> {
        file_io::check_directory(&self.entries_dir)?;
        let directory = self.entries_dir.join(".search-cache");
        file_io::create_directory(&directory)?;
        file_io::atomic_write(&directory.join(format!("{date}.json")), source, MAX_CACHE_BYTES)
    }

    pub fn prune_search_cache(&self, dates: &HashSet<NaiveDate>) -> io::Result<()> {
        file_io::check_directory(&self.entries_dir)?;
        let directory = self.entries_dir.join(".search-cache");
        match file_io::check_directory(&directory) {
            | Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            | result => result?,
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let name = entry.file_name();
            let date = name
                .to_str()
                .and_then(|s| s.strip_suffix(".json"))
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
            if date.is_some_and(|date| !dates.contains(&date)) {
                file_io::regular_file(&entry.path())?;
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }
}
