//! Effect interpreter and startup composition. Filesystem and clock access stay
//! here.
use diary_application::{Command,
                        EntryFormat,
                        Model,
                        Msg,
                        update};
use diary_storage::{Storage,
                    settings};
use std::{io,
          path::PathBuf};

pub struct Runtime {
    pub storage: Storage,
    pub settings_path: Option<PathBuf>,
}

impl Runtime {
    pub fn new(storage: Storage, settings_path: Option<PathBuf>) -> Self {
        Self {
            storage,
            settings_path,
        }
    }

    pub fn start() -> io::Result<(Self, Model)> {
        let storage = Storage::new()?;
        let entries = storage.scan_entries()?;
        let mut model = Model::new(chrono::Local::now().date_naive(), entries);
        let mut runtime = Self::new(storage, None);
        let loaded = settings::path().and_then(|path| {
            runtime.settings_path = Some(path.clone());
            settings::load(&path)
        });
        match loaded {
            | Ok(settings) => model.settings = settings,
            | Err(error) => {
                model.error_message = Some(format!("설정 읽기 실패 / Failed to load settings: {error}"));
                model.show_error_popup = true;
            },
        }
        if model.settings.keybindings == diary_application::settings::Keybindings::Helix {
            diary_application::editor::helix::reset(&mut model.editor_state, true);
        }
        runtime.execute(Command::LoadPreview(model.calendar_state.selected_date), &mut model);
        Ok((runtime, model))
    }

    pub fn execute(&mut self, command: Command, model: &mut Model) {
        let mut next = Some(command);
        while let Some(command) = next {
            let message = match command {
                | Command::SaveSettings => {
                    let result = self
                        .settings_path
                        .as_deref()
                        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No settings path"))
                        .and_then(|path| settings::save(model.settings, path));
                    match result {
                        | Ok(()) => return,
                        | Err(error) => Msg::SettingsSaveFailed(error.to_string()),
                    }
                },
                | Command::LoadPreview(date) => match self.storage.load_entry(date) {
                    | Ok(entry) => Msg::PreviewLoaded(date, Some(entry)),
                    | Err(error) if error.kind() == io::ErrorKind::NotFound => Msg::PreviewLoaded(date, None),
                    | Err(error) => Msg::LoadDiaryFailed(error.to_string()),
                },
                | Command::LoadDiary(date) => match self.storage.load_entry(date) {
                    | Ok(entry) => Msg::LoadDiarySuccess(date, entry.content, entry.format),
                    | Err(error) if error.kind() == io::ErrorKind::NotFound => Msg::LoadDiarySuccess(date, String::new(), EntryFormat::Org),
                    | Err(error) => Msg::LoadDiaryFailed(error.to_string()),
                },
                | Command::SaveDiary(date, content) => match self.storage.save(date, &content) {
                    | Ok(()) => Msg::SaveDiarySuccess(date),
                    | Err(error) => Msg::SaveDiaryFailed(error.to_string()),
                },
                | Command::DeleteDiary(date) => match self.storage.delete(date) {
                    | Ok(()) => Msg::DeleteDiarySuccess(date),
                    | Err(error) => Msg::SaveDiaryFailed(error.to_string()),
                },
            };
            next = update(model, message);
        }
    }
}
