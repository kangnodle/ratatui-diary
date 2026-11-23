use super::Runtime;
use diary_application::{Command,
                        EntryFormat,
                        Msg};
use diary_search::Worker;
use diary_storage::settings;
use std::io;

pub(super) fn perform(runtime: &mut Runtime, command: Command) -> Option<Msg> {
    let storage = &runtime.storage;
    Some(match command {
        | Command::Search(id, query) => {
            let worker = match runtime.search_worker(Worker::start) {
                | Ok(worker) => worker,
                | Err(error) => return Some(Msg::SearchFinished(id, Err(error.to_string()))),
            };
            return worker.submit(id, query).err().map(|error| Msg::SearchFinished(id, Err(error)));
        },
        | Command::SaveSettings(value) => {
            let result = runtime
                .settings_path
                .as_deref()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No settings path"))
                .and_then(|path| settings::save(value, path));
            return result.err().map(|error| Msg::SettingsSaveFailed(error.to_string()));
        },
        | Command::LoadPreview(date) => match storage.load_entry(date) {
            | Ok(entry) => Msg::PreviewLoaded(date, Some(entry)),
            | Err(error) if error.kind() == io::ErrorKind::NotFound => Msg::PreviewLoaded(date, None),
            | Err(error) => Msg::LoadDiaryFailed(error.to_string()),
        },
        | Command::LoadDiary(date) => match storage.load_entry(date) {
            | Ok(entry) => Msg::LoadDiarySuccess(date, entry.content, entry.format),
            | Err(error) if error.kind() == io::ErrorKind::NotFound => Msg::LoadDiarySuccess(date, String::new(), EntryFormat::Org),
            | Err(error) => Msg::LoadDiaryFailed(error.to_string()),
        },
        | Command::SaveDiary(date, content) => match storage.save(date, &content) {
            | Ok(()) => Msg::SaveDiarySuccess(date),
            | Err(error) => Msg::SaveDiaryFailed(error.to_string()),
        },
        | Command::DeleteDiary(date) => match storage.delete(date) {
            | Ok(()) => Msg::DeleteDiarySuccess(date),
            | Err(error) => Msg::SaveDiaryFailed(error.to_string()),
        },
    })
}
