use chrono::NaiveDate;
use std::ops::{Deref,
               DerefMut};
pub use text_editor::Selection;
use text_editor::TextEditor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSubMode {
    CtrlX,
    Search,
}

pub struct EditorState {
    pub helix: super::helix::HelixState,
    pub viewport_height: usize,
    pub editor: TextEditor,
    pub date: NaiveDate,
    pub format: diary_domain::EntryFormat,
    pub submode: Option<EditorSubMode>,
    pub search_origin: Option<(usize, usize)>,
    pub search_backward: bool,
    pub last_search_pattern: String,
}

impl EditorState {
    pub fn new(date: NaiveDate) -> Self {
        Self {
            helix: Default::default(),
            viewport_height: 20,
            editor: TextEditor::new(),
            date,
            format: Default::default(),
            submode: None,
            search_origin: None,
            search_backward: false,
            last_search_pattern: String::new(),
        }
    }

    pub(crate) fn start_search(&mut self, backward: bool) {
        if !self.search_pattern.is_empty() {
            self.last_search_pattern = self.search_pattern.clone();
        }
        self.search_origin = Some((self.cursor_line, self.cursor_col));
        self.search_backward = backward;
        self.submode = Some(EditorSubMode::Search);
        self.search_pattern.clear();
        self.refresh_search();
    }

    pub(crate) fn refresh_search(&mut self) {
        self.selection = None;
        if self.search_pattern.is_empty() {
            self.search_matches.clear();
            self.current_match_index = 0;
            if let Some((line, col)) = self.search_origin {
                self.cursor_line = line;
                self.cursor_col = col;
            }
        } else if self.search_backward {
            self.execute_search_backward();
        } else {
            self.execute_search();
        }
    }
}

impl Deref for EditorState {
    type Target = TextEditor;

    fn deref(&self) -> &Self::Target { &self.editor }
}

impl DerefMut for EditorState {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.editor }
}
