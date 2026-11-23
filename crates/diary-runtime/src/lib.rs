//! Runs application commands and feeds observations back into the state
//! machine.
use diary_application::{Command,
                        Model,
                        Msg,
                        Transition,
                        initialize,
                        transition,
                        update};
use diary_search::Worker;
use diary_storage::{Storage,
                    settings};
use std::{io,
          path::PathBuf,
          sync::mpsc::TryRecvError};

mod effects;

pub struct Runtime {
    pub storage: Storage,
    pub settings_path: Option<PathBuf>,
    search_worker: Option<Worker>,
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
        let today = chrono::Local::now().date_naive();
        let path = settings::path();
        let loaded = match &path {
            | Ok(path) => settings::load(path).map_err(|error| error.to_string()),
            | Err(error) => Err(error.to_string()),
        };
        let mut runtime = Self::new(storage, path.ok());
        let model = runtime.complete(initialize(today, entries, loaded));
        Ok((runtime, model))
    }

    /// Handles an input and completes its synchronous file operations before
    /// the next input.
    pub fn advance(&mut self, model: Model, message: Msg) -> Model { self.complete(transition(model, message)) }

    fn complete(&mut self, mut step: Transition) -> Model {
        while let Some(message) = step.command.and_then(|command| self.perform(command)) {
            step = transition(step.model, message);
        }
        step.model
    }

    /// Executes a command and returns its immediate observation, if any.
    ///
    /// File observations are applied before accepting another input. Search is
    /// submitted to the worker and eventually produces an observation
    /// through next_message.
    pub fn perform(&mut self, command: Command) -> Option<Msg> { effects::perform(self, command) }

    pub fn execute(&mut self, command: Command, model: &mut Model) {
        let mut next = Some(command);
        while let Some(message) = next.and_then(|command| self.perform(command)) {
            next = update(model, message);
        }
    }

    pub fn preload_search_model(&mut self) { let _ = self.search_worker(Worker::preload); }

    fn search_worker(&mut self, start: impl FnOnce(Storage) -> io::Result<Worker>) -> io::Result<&Worker> {
        match &mut self.search_worker {
            | Some(worker) => Ok(worker),
            | slot @ None => Ok(slot.insert(start(self.storage.clone())?)),
        }
    }

    /// Returns the next observation from the search adapter without blocking.
    pub fn next_message(&mut self) -> Option<Msg> {
        match self.search_worker.as_ref()?.try_response() {
            | Ok(response) => Some(Msg::SearchFinished(response.revision, response.result)),
            | Err(TryRecvError::Empty) => None,
            | Err(TryRecvError::Disconnected) => {
                self.search_worker = None;
                Some(Msg::SearchWorkerStopped)
            },
        }
    }

    pub fn poll(&mut self, model: &mut Model) -> bool {
        let mut changed = false;
        while let Some(message) = self.next_message() {
            changed |= !matches!(message, Msg::SearchWorkerStopped) || model.search.is_busy();
            if let Some(command) = update(model, message) {
                self.execute(command, model);
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn reusing_the_search_worker_leaves_entry_operations_available() {
        let directory = tempfile::TempDir::new().expect("temporary diary directory");
        let storage = Storage::with_dir(directory.path()).expect("diary storage");
        let mut runtime = Runtime::new(storage, None);
        let starts = Cell::new(0);
        let start = |storage| {
            starts.set(starts.get() + 1);
            Worker::start(storage)
        };
        runtime.search_worker(start).expect("search worker");
        runtime.search_worker(start).expect("reused search worker");
        assert_eq!(starts.get(), 1);
        assert!(runtime.next_message().is_none());

        let today = chrono::NaiveDate::default();
        let model = Model::new(today, Default::default());
        let model = runtime.advance(model, Msg::CalendarSelectDate);
        let model = runtime.advance(model, Msg::EditorInsertChar('한'));
        let model = runtime.advance(model, Msg::Quit);
        assert!(model.should_quit);
        assert!(!model.editor_state.is_modified);
        assert_eq!(runtime.storage.load(today).expect("saved diary"), "한");
    }
}
