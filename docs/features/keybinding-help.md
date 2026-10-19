# Emacs / Helix 키바인딩 도움말

화면 하단에 현재 화면과 접두키·검색 상태에 맞는 도움말을 표시합니다.
`C-`는 Ctrl, `M-`는 Alt입니다. `C-x C-s`는 두 키 조합의 순차 입력입니다.
`F2` 설정의 키바인딩 항목에서 Emacs와 Helix를 전환합니다. 선택은 저장되어 재시작 후에도 유지됩니다.

## Emacs 달력

- `C-b/f/p/n` 또는 방향키로 날짜 이동, `Enter`로 편집
- `M-n/p`로 월 이동, `M-]/[`로 연도 이동
- `C-x`를 누르면 종료 명령 도움말 표시, `C-x C-c`로 저장 후 종료
- `C-g` 또는 `Esc`로 접두키 취소

## Emacs 에디터

- 기본 상태: `C-x` 명령, `C-g` 취소, `C-s/r` 증분 검색, `C-/` 실행취소, `M-d/Backspace` 단어 잘라내기 안내
- `C-x` 접두키: `C-s` 또는 `s` 저장, `k` 저장 후 달력, `C-c` 저장 후 종료, `u` 실행취소
- 검색 상태: 문자 입력 즉시 검색, `C-s/r` 다음/이전, `Enter` 확정, `C-g` 취소와 커서 위치 복원
- 역방향 검색 중에는 상태바에 방향 표시
- 본문·미리보기와 키 도움말은 분리된 줄에 표시

일반 이동·영역 선택·잘라내기·붙여넣기와 나머지 키는 [README](../../README.md#에디터-화면-emacs-스타일)에 정리되어 있습니다.

## Helix

- Normal / Insert / Select와 숫자 반복·접두키 상태를 표시
- Normal·Select: `h/j/k/l`, `w/b/e`, `v`, `i/a`, `d/c/y/p`, `/`, `:` 안내
- Insert: `Esc` Normal 복귀, `C-s` 실행취소 지점 안내
- Goto: `g/e/h/l/s`의 문서·줄 이동 안내
- 검색: 정규식 입력, Enter 확정, Esc 원래 선택 복원
- Command: `:w`, `:q`, `:wq`, `:bc` 안내. `:q`와 `:bc`는 미저장 변경이 있으면 중단
- Space: `f`로 날짜 기반 일기 선택(달력). `Space w`에 저장을 배정하지 않음
- 달력: `h/j/k/l` 이동, `H/L` 월 이동, `{/}` 연도 이동, `Enter` 편집, `:q` 종료

[지원 키와 제한](../../README.md#에디터-화면-helix)은 README에 명시합니다. 단일 선택 편집을 지원하며 다중 커서·LSP·분할 창은 제공하지 않습니다.

## 설정과 오류

- `F2`로 설정 열기
- `C-n/p`, 방향키 또는 Tab/Shift+Tab으로 설정 항목 선택. Helix에서는 `j/k`도 지원
- `C-g` 또는 `Esc`로 설정·오류 창 닫기
- 저장 실패 시 종료·달력 복귀를 중단하고 편집 내용 유지

## 구현 및 검증

- 키 입력: `crates/ratatui-diary/src/app/keymap.rs`, `editor/keymap.rs`, `calendar/keymap.rs`
- 번역: `crates/ratatui-diary/src/i18n.rs`의 한국어·영어 카탈로그
- 상태 표시: `editor/view.rs`, `calendar/view.rs`
- 실제 키 입력 순서·유니코드 검색·저장 실패 검증: `tests/emacs_keybindings.rs`
- Helix 모드·선택·접두키·정규식·전환·설정 호환 검증: `tests/helix_keybindings.rs`
- 도움말 문자열 및 화면 검증: `tests/view_tests.rs`, `tests/settings_tests.rs`
