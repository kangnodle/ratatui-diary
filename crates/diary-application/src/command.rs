use chrono::NaiveDate;
use diary_domain::settings::Settings;

/// Effect requested by a transition. Each variant owns all its inputs.
pub enum Command {
    LoadPreview(NaiveDate),
    SaveSettings(Settings),
    LoadDiary(NaiveDate),
    SaveDiary(NaiveDate, String),
    DeleteDiary(NaiveDate),
    Search(u64, String),
}
