# Emacs 스타일 키바인딩 도움말

화면 하단에 현재 화면과 접두키·검색 상태에 맞는 도움말을 표시합니다.
`C-`는 Ctrl, `M-`는 Alt입니다. `C-x C-s`는 두 키 조합의 순차 입력입니다.

## 달력

- `C-b/f/p/n` 또는 방향키로 날짜 이동, `Enter`로 편집
- `M-n/p`로 월 이동, `M-]/[`로 연도 이동
- `C-x`를 누르면 종료 명령 도움말 표시, `C-x C-c`로 저장 후 종료
- `C-g` 또는 `Esc`로 접두키 취소

## 에디터

- 기본 상태: `C-x` 명령, `C-g` 취소, `C-s/r` 증분 검색, `C-/` 실행취소, `M-d/Backspace` 단어 잘라내기 안내
- `C-x` 접두키: `C-s` 또는 `s` 저장, `k` 저장 후 달력, `C-c` 저장 후 종료, `u` 실행취소
- 검색 상태: 문자 입력 즉시 검색, `C-s/r` 다음/이전, `Enter` 확정, `C-g` 취소와 커서 위치 복원
- 역방향 검색 중에는 상태바에 방향 표시
- 본문·미리보기와 키 도움말은 분리된 줄에 표시

일반 이동·영역 선택·잘라내기·붙여넣기와 나머지 키는 [README](../../README.md#에디터-화면-emacs-스타일)에 정리되어 있습니다.

## 설정과 오류

- `F2`로 설정 열기
- `C-n/p`, 방향키 또는 Tab으로 설정 항목 선택
- `C-g` 또는 `Esc`로 설정·오류 창 닫기
- 저장 실패 시 종료·달력 복귀를 중단하고 편집 내용 유지

## 구현 및 검증

- 키 입력: `crates/ratatui-diary/src/app/keymap.rs`, `editor/keymap.rs`, `calendar/keymap.rs`
- 번역: `crates/ratatui-diary/src/i18n.rs`의 한국어·영어 카탈로그
- 상태 표시: `editor/view.rs`, `calendar/view.rs`
- 실제 키 입력 순서·유니코드 검색·저장 실패 검증: `tests/emacs_keybindings.rs`
- 도움말 문자열 및 화면 검증: `tests/view_tests.rs`, `tests/settings_tests.rs`
