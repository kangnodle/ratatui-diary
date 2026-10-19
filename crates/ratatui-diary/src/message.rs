//! 앱 메시지와 Emacs/Helix 편집 입력.

use chrono::NaiveDate;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Quit,
    ToggleSettings,
    SettingsMove,
    SettingsMoveBack,
    SettingsChange(bool),
    SettingsSaveFailed(String),
    DismissError,
    HelixKey(crossterm::event::KeyEvent),

    // 에디터 - 네비게이션
    EditorMoveLeft,
    EditorMoveRight,
    EditorMoveUp,
    EditorMoveDown,
    EditorWordNext,
    EditorWordPrev,

    // 에디터 - 점프
    EditorGotoDocStart,
    EditorGotoDocEnd,
    EditorGotoLineStart,
    EditorGotoLineEnd,
    EditorExitSubMode,
    EditorKeyboardQuit,

    // 에디터 - 문자 입력 (항상 활성)
    EditorInsertChar(char),
    EditorBackspace,
    EditorNewLine,
    EditorOpenLine,

    // 에디터 - Selection
    EditorToggleSelection,
    EditorSelectLine,

    // 에디터 - 편집
    EditorDelete,
    EditorDeleteForward,
    EditorKillLine,
    EditorKillWord,
    EditorBackwardKillWord,
    EditorYank,
    EditorPaste,

    // 에디터 - Undo/Redo
    EditorUndo,
    EditorRedo,

    // 에디터 - 검색
    EditorEnterSearchMode,
    EditorEnterReverseSearchMode,
    EditorSearchChar(char),
    EditorSearchBackspace,
    EditorSearchNext,
    EditorSearchPrev,
    EditorExecuteSearch,

    // 에디터 - Ctrl+X 프리픽스
    EditorEnterCtrlXMode,
    EditorCtrlXSave,
    EditorCtrlXBack,

    // 달력
    CalendarMoveLeft,
    CalendarMoveRight,
    CalendarMoveUp,
    CalendarMoveDown,
    CalendarSelectDate,
    CalendarNextMonth,
    CalendarPrevMonth,
    CalendarNextYear,
    CalendarPrevYear,

    // 파일 I/O 결과
    LoadDiarySuccess(NaiveDate, String),
    LoadDiaryFailed(String),
    SaveDiarySuccess,
    SaveDiaryFailed(String),
    DeleteDiarySuccess(NaiveDate),
    RefreshIndex(HashSet<NaiveDate>),
}
