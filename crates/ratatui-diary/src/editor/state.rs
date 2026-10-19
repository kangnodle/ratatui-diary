use chrono::NaiveDate;
use std::ops::{Deref,
               DerefMut};
// 텍스트 에디터 엔진의 타입을 그대로 재노출 (기존 경로 호환)
pub use text_editor::Selection;
use text_editor::TextEditor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSubMode {
    CtrlX,
    Search,
}

/// 다이어리 화면의 에디터 상태.
///
/// 범용 텍스트 편집 기능은 [`TextEditor`] 엔진에 위임하고
/// (`Deref`/`DerefMut`로 투명하게 노출), 다이어리 고유의 상태인
/// 편집 대상 날짜와 키 서브모드만 직접 보유한다.
pub struct EditorState {
    pub editor: TextEditor,
    pub date: NaiveDate,
    pub submode: Option<EditorSubMode>,
    pub search_origin: Option<(usize, usize)>,
    pub search_backward: bool,
    pub last_search_pattern: String,
}

impl EditorState {
    pub fn new(date: NaiveDate) -> Self {
        Self {
            editor: TextEditor::new(),
            date,
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
