use chrono::NaiveDate;
use diary_domain::{DiaryEntry,
                   EntryFormat};
pub mod settings;
use std::{collections::HashSet,
          ffi::OsString,
          fs,
          io,
          path::{Path,
                 PathBuf}};

pub struct Storage {
    entries_dir: PathBuf,
}

impl Storage {
    pub fn with_dir(base_dir: &Path) -> io::Result<Self> {
        let entries_dir = base_dir.join("entries");
        fs::create_dir_all(&entries_dir)?;
        Ok(Self {
            entries_dir,
        })
    }

    pub fn new() -> io::Result<Self> { Self::new_impl(dirs::data_local_dir()) }

    fn new_impl(data_dir: Option<PathBuf>) -> io::Result<Self> {
        let base_dir = data_dir
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Cannot find local data directory"))?
            .join("ratatui-diary");
        Self::with_dir(&base_dir)
    }

    #[doc(hidden)]
    pub fn new_with_none_dir() -> io::Result<Self> { Self::new_impl(None) }

    pub fn save(&self, date: NaiveDate, content: &str) -> io::Result<()> {
        let path = self.get_path(date)?;
        fs::write(path, content)
    }

    pub fn load(&self, date: NaiveDate) -> io::Result<String> { self.load_entry(date).map(|entry| entry.content) }

    pub fn load_entry(&self, date: NaiveDate) -> io::Result<DiaryEntry> {
        let path = self.get_path(date)?;
        let format = if path.extension().is_some_and(|ext| ext == "md") {
            EntryFormat::Markdown
        } else {
            EntryFormat::Org
        };
        Ok(DiaryEntry {
            content: fs::read_to_string(path)?,
            format,
        })
    }

    pub fn delete(&self, date: NaiveDate) -> io::Result<()> {
        let path = self.get_path(date)?;
        fs::remove_file(&path)?;
        // Remove a shadowed legacy entry too, so a deleted date cannot
        // reappear.
        if path.extension().is_some_and(|ext| ext == "org") {
            match fs::remove_file(path.with_extension("md")) {
                | Err(e) if e.kind() == io::ErrorKind::NotFound => {},
                | result => result?,
            }
        }
        Ok(())
    }

    pub fn scan_entries(&self) -> io::Result<HashSet<NaiveDate>> {
        let mut entries = HashSet::new();

        for entry in fs::read_dir(&self.entries_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            if let Some(date) = parse_filename(entry.file_name()) {
                entries.insert(date);
            }
        }

        Ok(entries)
    }

    /// Existing Markdown entries keep their format; new dates default to Org.
    pub fn is_markdown(&self, date: NaiveDate) -> io::Result<bool> {
        Ok(!self.entries_dir.join(format!("{date}.org")).try_exists()? && self.entries_dir.join(format!("{date}.md")).try_exists()?)
    }

    fn get_path(&self, date: NaiveDate) -> io::Result<PathBuf> {
        let extension = if self.is_markdown(date)? { "md" } else { "org" };
        Ok(self.entries_dir.join(format!("{date}.{extension}")))
    }
}

fn parse_filename(filename: OsString) -> Option<NaiveDate> {
    let filename = filename.to_str()?;
    let name = filename.strip_suffix(".org").or_else(|| filename.strip_suffix(".md"))?;
    NaiveDate::parse_from_str(name, "%Y-%m-%d").ok()
}
