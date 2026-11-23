pub mod calendar;
pub mod i18n;
pub mod settings;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EntryFormat {
    #[default]
    Org,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiaryEntry {
    pub content: String,
    pub format: EntryFormat,
}
