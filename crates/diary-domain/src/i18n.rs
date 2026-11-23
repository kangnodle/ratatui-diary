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
    OrgPreview,
    EmptyDiary,
    Diary,
    UnknownError,
    ErrorTitle,
    System,
    Dark,
    Light,
    Language,
    Theme,
    Keybindings,
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
                "C-b/f/p/n:이동 | Enter:편집\nM-p/n · PgUp/Dn:이전/다음 월\nM-[/] · C-PgUp/Dn:이전/다음 년\nC-x C-c:종료 | C-g:취소",
                "C-b/f/p/n:Move | Enter:Edit\nM-p/n · PgUp/Dn:Previous/Next month\nM-[/] · C-PgUp/Dn:Previous/Next year\nC-x C-c:Quit | C-g:Cancel",
            ),
            | Text::EditorKeys => (
                "C-x:명령 | C-g:취소 | C-s/r:검색 | C-/:실행취소 | M-d/BS:단어삭제",
                "C-x:Commands | C-g:Cancel | C-s/r:Search | C-/:Undo | M-d/BS:Kill word",
            ),
            | Text::CommandKeys => (
                "C-s/s:저장 | k:저장 후 달력 | C-c:저장 후 종료 | u:실행취소 | C-g:취소",
                "C-s/s:Save | k:Save & calendar | C-c:Save & quit | u:Undo | C-g:Cancel",
            ),
            | Text::SearchKeys => (
                "입력:즉시 검색 | C-s/r:다음/이전 | Enter:확정 | C-g:취소·위치복원",
                "Type:Search now | C-s/r:Next/Previous | Enter:Accept | C-g:Cancel & restore",
            ),

            | Text::Settings => ("설정", "Settings"),
            | Text::LanguageName => ("한국어", "English"),
            | Text::MarkdownPreview => ("Markdown 미리보기", "Markdown preview"),
            | Text::OrgPreview => ("Org 미리보기", "Org preview"),
            | Text::EmptyDiary => (
                "작성된 다이어리가 없습니다.\n\nEnter를 눌러 새로 작성하세요.",
                "No diary entry.\n\nPress Enter to start writing.",
            ),
            | Text::Diary => ("다이어리", "Diary"),
            | Text::UnknownError => ("알 수 없는 에러", "Unknown error"),
            | Text::ErrorTitle => ("오류 · C-g/Esc:닫기", "Error · C-g/Esc:Close"),
            | Text::System => ("시스템", "System"),
            | Text::Dark => ("다크", "Dark"),
            | Text::Light => ("라이트", "Light"),
            | Text::Language => ("언어", "Language"),
            | Text::Theme => ("테마", "Theme"),
            | Text::Keybindings => ("키바인딩", "Keybindings"),
            | Text::SettingsNavigation => ("C-n/p/↑/↓/Tab:항목  ←/→/Enter:변경", "C-n/p/↑/↓/Tab:Select  ←/→/Enter:Change"),
            | Text::SettingsHelp => ("즉시 적용·자동 저장 | C-g/Esc/F2:닫기", "Applied and saved automatically | C-g/Esc/F2:Close"),
            | Text::FailedToSaveSettings => ("설정 저장 실패", "Failed to save settings"),
            | Text::LoadFailed => ("로드 실패", "Load failed"),
            | Text::SaveFailed => ("저장 실패", "Save failed"),
        };
        self.text(ko, en)
    }
}
