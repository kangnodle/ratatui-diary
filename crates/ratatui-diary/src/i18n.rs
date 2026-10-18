//! UI translation catalog. Diary contents are never translated.
use crate::settings::Language;
#[derive(Clone, Copy)]
pub enum Text {
    CalendarKeys,
    EditorKeys,
    CommandKeys,
    SearchKeys,

    Settings,
    LanguageName,
    MarkdownPreview,
    EmptyDiary,
    Diary,
    UnknownError,
    ErrorTitle,
    System,
    Dark,
    Light,
    Language,
    Theme,
    SettingsNavigation,
    SettingsHelp,
    FailedToSaveSettings,
    LoadFailed,
    SaveFailed,
}
impl Language {
    pub fn tr(self, text: Text) -> &'static str {
        let (ko, en) = match text {
            | Text::CalendarKeys => (
                "C-b/f/p/n:이동 | Enter:편집 | M-n/p:월 | M-]/[:년 | C-q:종료",
                "C-b/f/p/n:Move | Enter:Edit | M-n/p:Month | M-]/[:Year | C-q:Quit",
            ),
            | Text::EditorKeys => (
                "C-f/b/n/p:이동 | C-a/e:줄시작/끝 | M-f/b:단어 | M-</>:문서 | C-h/d:삭제 | C-k:줄삭제 | C-o:줄열기 | C-SPC:마크 | C-w:잘라내기 | M-w:복사 | C-y:붙여넣기 | C-z:실행취소 | C-s:검색 | C-x:명령 | C-q:종료",
                "C-f/b/n/p:Move | C-a/e:Line start/end | M-f/b:Word | M-</>:Document | C-h/d:Delete | C-k:Kill line | C-o:Open line | C-SPC:Mark | C-w:Cut | M-w:Copy | C-y:Paste | C-z:Undo | C-s:Search | C-x:Command | C-q:Quit",
            ),
            | Text::CommandKeys => ("C-s:저장 | C-c:뒤로 | Esc:취소", "C-s:Save | C-c:Back | Esc:Cancel"),
            | Text::SearchKeys => (
                "입력:검색어 | Enter:실행 | C-s/r:다음/이전 | Esc:취소",
                "Type:Query | Enter:Search | C-s/r:Next/Previous | Esc:Cancel",
            ),

            | Text::Settings => ("설정", "Settings"),
            | Text::LanguageName => ("한국어", "English"),
            | Text::MarkdownPreview => ("Markdown 미리보기", "Markdown preview"),
            | Text::EmptyDiary => (
                "📝 작성된 다이어리가 없습니다.\n\nEnter를 눌러 새로 작성하세요.",
                "📝 No diary entry.\n\nPress Enter to start writing.",
            ),
            | Text::Diary => ("다이어리", "Diary"),
            | Text::UnknownError => ("알 수 없는 에러", "Unknown error"),
            | Text::ErrorTitle => ("오류 · Esc:닫기", "Error · Esc:Close"),
            | Text::System => ("시스템", "System"),
            | Text::Dark => ("다크", "Dark"),
            | Text::Light => ("라이트", "Light"),
            | Text::Language => ("언어", "Language"),
            | Text::Theme => ("테마", "Theme"),
            | Text::SettingsNavigation => ("↑/↓/Tab:항목  ←/→/Enter:변경", "↑/↓/Tab:Select  ←/→/Enter:Change"),
            | Text::SettingsHelp => ("즉시 적용·자동 저장 | Esc/F2:닫기", "Applied and saved automatically | Esc/F2:Close"),
            | Text::FailedToSaveSettings => ("설정 저장 실패", "Failed to save settings"),
            | Text::LoadFailed => ("로드 실패", "Load failed"),
            | Text::SaveFailed => ("저장 실패", "Save failed"),
        };
        self.text(ko, en)
    }
}
