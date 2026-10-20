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
mod search;

pub struct Runtime {
    pub storage: Storage,
    pub settings_path: Option<PathBuf>,
    search_worker: Option<search::Worker>,
}

impl Runtime {
    pub fn new(storage: Storage, settings_path: Option<PathBuf>) -> Self {
        Self {
            storage,
            settings_path,
            search_worker: None,
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
                | Command::Search(id, query) => {
                    let result = (|| {
                        if self.search_worker.is_none() {
                            self.search_worker = Some(search::Worker::start(self.storage.clone()).map_err(|e| e.to_string())?);
                        }
                        self.search_worker.as_ref().unwrap().submit(id, query)
                    })();
                    match result {
                        | Ok(()) => return,
                        | Err(error) => Msg::SearchFinished(id, Err(error)),
                    }
                },
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

    /// Schedule once after the shell's first frame; model loading stays on the
    /// search worker.
    pub fn preload_search_model(&mut self) {
        if self.search_worker.is_none() {
            // Search retries thread creation and reports failure through its
            // normal result path.
            self.search_worker = search::Worker::preload(self.storage.clone()).ok();
        }
    }

    /// Called by the shell between terminal events; stale search replies are
    /// discarded by update.
    pub fn poll(&mut self, model: &mut Model) -> bool {
        let Some(worker) = &self.search_worker else {
            return false;
        };
        let mut changed = false;
        loop {
            match worker.responses.try_recv() {
                | Ok(message) => {
                    update(model, message);
                    changed = true;
                },
                | Err(std::sync::mpsc::TryRecvError::Empty) => break,
                | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if model.search.busy {
                        update(
                            model,
                            Msg::SearchFinished(model.search.revision, Err("검색 작업이 종료되었습니다 / Search worker stopped".into())),
                        );
                        changed = true;
                    }
                    self.search_worker = None;
                    break;
                },
            }
        }
        changed
    }
}

#[cfg(test)]
mod preload_tests {
    use super::*;
    use std::{sync::{Arc,
                     atomic::{AtomicUsize,
                              Ordering},
                     mpsc},
              time::Duration};
    use tempfile::TempDir;

    #[test]
    fn paused_preload_runs_once_off_thread_and_does_not_block_edit_save_or_quit() {
        let directory = TempDir::new().unwrap();
        let storage = Storage::with_dir(directory.path()).unwrap();
        let mut runtime = Runtime::new(storage.clone(), None);
        let mut model = Model::new(chrono::NaiveDate::default(), Default::default());
        let (entered, started) = mpsc::channel();
        let (release, paused) = mpsc::channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let loaded = calls.clone();
        runtime.search_worker = Some(
            search::Worker::spawn(
                storage,
                Some(move || {
                    loaded.fetch_add(1, Ordering::SeqCst);
                    entered.send(std::thread::current().id()).unwrap();
                    let _ = paused.recv();
                    Err("simulated preload failure".into())
                }),
            )
            .unwrap(),
        );
        assert_ne!(started.recv_timeout(Duration::from_secs(3)).unwrap(), std::thread::current().id());
        runtime.preload_search_model();
        runtime.preload_search_model();
        assert!(!runtime.poll(&mut model));
        let command = update(&mut model, Msg::CalendarSelectDate).unwrap();
        runtime.execute(command, &mut model);
        update(&mut model, Msg::EditorInsertChar('한'));
        runtime.execute(Command::SaveDiary(model.today, model.editor_state.get_content()), &mut model);
        assert_eq!(runtime.storage.load(model.today).unwrap(), "한");
        assert!(!model.editor_state.is_modified);
        assert!(update(&mut model, Msg::Quit).is_none());
        assert!(model.should_quit);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        release.send(()).unwrap();
        assert_eq!(
            runtime.search_worker.as_ref().unwrap().ready.recv_timeout(Duration::from_secs(3)).unwrap(),
            Err("simulated preload failure".into())
        );
        assert!(!model.show_error_popup);
    }

    #[test]
    fn queries_during_preload_share_the_worker_and_keep_only_the_latest_query() {
        let directory = TempDir::new().unwrap();
        let storage = Storage::with_dir(directory.path()).unwrap();
        let mut runtime = Runtime::new(storage.clone(), None);
        let mut model = Model::new(chrono::NaiveDate::default(), Default::default());
        let (entered, started) = mpsc::channel();
        let (release, paused) = mpsc::channel();
        runtime.search_worker = Some(
            search::Worker::spawn(
                storage,
                Some(move || {
                    entered.send(()).unwrap();
                    let _ = paused.recv();
                    Err("simulated preload failure".into())
                }),
            )
            .unwrap(),
        );
        started.recv_timeout(Duration::from_secs(3)).unwrap();
        for id in 1 ..= 3 {
            runtime.execute(Command::Search(id, format!("query {id}")), &mut model);
        }
        assert!(runtime.search_worker.as_ref().unwrap().responses.try_recv().is_err());
        release.send(()).unwrap();
        let worker = runtime.search_worker.as_ref().unwrap();
        worker.ready.recv_timeout(Duration::from_secs(3)).unwrap().unwrap_err();
        assert!(matches!(
            worker.responses.recv_timeout(Duration::from_secs(3)).unwrap(),
            Msg::SearchFinished(3, Ok(_))
        ));
        assert!(worker.responses.try_recv().is_err());
    }
}
