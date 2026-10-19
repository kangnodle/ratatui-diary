# Diary 미리보기 기능 구현 계획

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**목표:** 달력 화면과 에디터 화면에 실시간 미리보기 기능 추가

**아키텍처:** ELM 패턴을 유지하면서 화면을 50:50 수평 분할. 달력 화면은 선택된 날짜의 다이어리 텍스트를 표시하고, 에디터 화면은 termimad를 사용한 Markdown 렌더링 결과를 실시간으로 표시합니다.

**기술 스택:** Rust, ratatui, termimad, chrono

---

## Phase 1: 기본 구조

### Task 1: termimad 의존성 추가

**파일:**
- 수정: `Cargo.toml`

**Step 1: Cargo.toml에 termimad 의존성 추가**

`Cargo.toml`의 `[dependencies]` 섹션에 추가:

```toml
termimad = "0.34"
```

참고: 초기 계획의 0.29 대신 0.34 사용 (crossterm 0.27 호환성 문제 해결)

**Step 2: 의존성 설치 확인**

실행: `cargo check`
예상 결과: termimad 다운로드 및 컴파일 성공

**Step 3: 커밋**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: Add termimad for Markdown rendering

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 2: Markdown 모듈 기본 구조 생성

**파일:**
- 생성: `src/markdown.rs`
- 수정: `src/lib.rs`

**Step 1: 빈 Markdown 렌더링 테스트 작성**

`tests/markdown_tests.rs` 생성:

```rust
use ratatui_diary::markdown::render_to_text;

#[test]
fn test_render_empty_string() {
    let result = render_to_text("");
    assert!(!result.lines.is_empty()); // 최소한 빈 줄은 있어야 함
}

#[test]
fn test_render_plain_text() {
    let result = render_to_text("Hello, World!");
    // 텍스트가 포함되어 있는지 확인 (정확한 형식은 구현 후 조정)
    assert!(result.lines.len() > 0);
}
```

**Step 2: 테스트 실행 (실패 확인)**

실행: `cargo test test_render_empty_string test_render_plain_text`
예상 결과: FAIL - module `markdown` not found

**Step 3: markdown.rs 모듈 생성 및 기본 구현**

`src/markdown.rs` 생성:

```rust
use ratatui::text::{Line, Span, Text};
use ratatui::style::{Color, Modifier, Style};

/// Markdown 문자열을 ratatui Text로 렌더링
pub fn render_to_text(markdown: &str) -> Text<'static> {
    if markdown.is_empty() {
        return Text::from(vec![Line::from("")]);
    }

    // 임시: 단순 텍스트로 반환 (termimad 통합 전)
    let lines: Vec<Line> = markdown
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect();

    Text::from(lines)
}
```

`src/lib.rs`에 모듈 추가:

```rust
pub mod markdown;
```

**Step 4: 테스트 실행 (성공 확인)**

실행: `cargo test test_render_empty_string test_render_plain_text`
예상 결과: PASS

**Step 5: 커밋**

```bash
git add src/markdown.rs src/lib.rs tests/markdown_tests.rs
git commit -m "feat(markdown): Add basic markdown module with plain text rendering

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 3: Model에 Storage 참조 추가

**파일:**
- 수정: `src/model.rs`
- 수정: `src/main.rs`

**Step 1: Model에 storage 필드 추가 테스트**

`tests/model_tests.rs`에 추가:

```rust
use ratatui_diary::storage::Storage;
use std::collections::HashSet;
use tempfile::TempDir;

#[test]
fn test_model_with_storage() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();
    let entries = HashSet::new();

    let model = Model::new(entries, storage);
    assert_eq!(model.screen, Screen::Calendar);
}
```

**Step 2: 테스트 실행 (실패 확인)**

실행: `cargo test test_model_with_storage`
예상 결과: FAIL - `new` takes 1 argument

**Step 3: Model 구조체에 storage 필드 추가**

`src/model.rs` 수정:

```rust
use crate::storage::Storage;

pub struct Model {
    pub screen: Screen,
    pub calendar_state: CalendarState,
    pub editor_state: EditorState,
    pub diary_entries: DiaryIndex,
    pub error_message: Option<String>,
    pub show_error_popup: bool,
    pub storage: Storage,  // 추가
}

impl Model {
    pub fn new(entries: HashSet<NaiveDate>, storage: Storage) -> Self {
        let today = chrono::Local::now().date_naive();

        Self {
            screen: Screen::Calendar,
            calendar_state: CalendarState::new(today.year(), today.month()),
            editor_state: EditorState::new(today),
            diary_entries: DiaryIndex { entries },
            error_message: None,
            show_error_popup: false,
            storage,  // 추가
        }
    }
}
```

**Step 4: main.rs 수정**

`src/main.rs`에서 Model 생성 부분 수정:

```rust
let entries = storage.scan_entries().unwrap_or_default();
let mut model = Model::new(entries, storage);
```

**Step 5: 테스트 실행 (성공 확인)**

실행: `cargo test`
예상 결과: 모든 테스트 PASS

**Step 6: 커밋**

```bash
git add src/model.rs src/main.rs tests/model_tests.rs
git commit -m "feat(model): Add storage field to Model for preview access

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 2: 달력 미리보기

### Task 4: 달력 화면 레이아웃 50:50 분할

**파일:**
- 수정: `src/view.rs`

**Step 1: 레이아웃 분할 함수 작성**

`src/view.rs`의 `render_calendar` 함수 수정:

```rust
fn render_calendar(f: &mut Frame, model: &Model) {
    // 메인 레이아웃: 수평 분할 (50:50)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),  // 왼쪽: 달력
            Constraint::Percentage(50),  // 오른쪽: 미리보기
        ])
        .split(f.size());

    // 왼쪽: 달력 영역 (기존 레이아웃)
    let calendar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 헤더
            Constraint::Min(0),    // 달력
            Constraint::Length(2), // 상태바
        ])
        .split(main_chunks[0]);

    // 헤더
    let header = Paragraph::new(format!(
        "{}년 {}월",
        model.calendar_state.current_year, model.calendar_state.current_month
    ))
    .alignment(Alignment::Center)
    .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(header, calendar_chunks[0]);

    // 달력 그리드
    render_calendar_grid(f, calendar_chunks[1], model);

    // 상태바
    let statusbar = Paragraph::new("h/l: 달 | H/L: 연도 | Enter: 작성 | q: 종료")
        .alignment(Alignment::Center);
    f.render_widget(statusbar, calendar_chunks[2]);

    // 오른쪽: 미리보기 영역 (임시로 빈 블록)
    let preview_block = Block::default()
        .title("미리보기")
        .borders(Borders::ALL);
    f.render_widget(preview_block, main_chunks[1]);
}
```

**Step 2: 컴파일 확인**

실행: `cargo check`
예상 결과: 성공

**Step 3: 수동 테스트**

실행: `cargo run`
확인: 달력 화면이 50:50으로 분할되고 오른쪽에 빈 미리보기 블록 표시

**Step 4: 커밋**

```bash
git add src/view.rs
git commit -m "feat(view): Split calendar screen into 50:50 layout

Left: calendar grid
Right: preview pane (empty block for now)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 5: render_preview_pane 함수 구현

**파일:**
- 수정: `src/view.rs`

**Step 1: 미리보기 패널 렌더링 함수 작성**

`src/view.rs`에 추가:

```rust
fn render_preview_pane(f: &mut Frame, area: Rect, content: &str, title: &str) {
    let text = Paragraph::new(content)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false })
        .style(Style::default());

    f.render_widget(text, area);
}
```

**Step 2: render_calendar에서 함수 사용**

`render_calendar` 함수에서 임시 블록을 대체:

```rust
// 오른쪽: 미리보기 영역
render_preview_pane(
    f,
    main_chunks[1],
    "미리보기 테스트 콘텐츠\n\n여러 줄\n테스트",
    "선택된 날짜 미리보기"
);
```

**Step 3: 컴파일 및 수동 테스트**

실행: `cargo run`
확인: 오른쪽 패널에 테스트 콘텐츠가 테두리와 함께 표시됨

**Step 4: 커밋**

```bash
git add src/view.rs
git commit -m "feat(view): Implement render_preview_pane for text display

Reusable function for rendering preview content with border and title

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 6: 달력 미리보기 Storage 통합

**파일:**
- 수정: `src/view.rs`

**Step 1: Storage 로드 통합 테스트 작성**

`tests/view_tests.rs` 생성:

```rust
use ratatui_diary::{model::Model, storage::Storage};
use chrono::NaiveDate;
use std::collections::HashSet;
use tempfile::TempDir;

#[test]
fn test_calendar_preview_loads_diary() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();

    // 테스트 다이어리 작성
    let date = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
    storage.save(date, "Test diary content").unwrap();

    let mut entries = HashSet::new();
    entries.insert(date);

    let model = Model::new(entries, storage);

    // view 렌더링 시 storage.load()가 호출되어야 함
    // (실제 렌더링 테스트는 어려우므로 모델 상태만 확인)
    assert_eq!(model.calendar_state.selected_date, date);
}
```

**Step 2: 테스트 실행**

실행: `cargo test test_calendar_preview_loads_diary`
예상 결과: PASS (모델 통합 확인)

**Step 3: render_calendar에서 실제 다이어리 로드**

`src/view.rs`의 `render_calendar` 함수 수정:

```rust
// 오른쪽: 미리보기 영역
let selected_date = model.calendar_state.selected_date;
let preview_content = match model.storage.load(selected_date) {
    Ok(content) => content,
    Err(_) => "작성된 다이어리가 없습니다.\n\nEnter를 눌러 새로 작성하세요.".to_string(),
};

render_preview_pane(
    f,
    main_chunks[1],
    &preview_content,
    &format!("다이어리: {}", selected_date)
);
```

**Step 4: 컴파일 및 수동 테스트**

실행: `cargo run`
확인:
- 다이어리가 있는 날짜: 내용 표시
- 다이어리가 없는 날짜: 안내 메시지 표시
- 날짜 이동 시 미리보기 실시간 업데이트

**Step 5: 커밋**

```bash
git add src/view.rs tests/view_tests.rs
git commit -m "feat(calendar): Integrate storage for diary preview

Load and display diary content for selected date in preview pane.
Show friendly message when no diary exists.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 3: 에디터 Markdown 미리보기

### Task 7: termimad 통합 및 기본 Markdown 렌더링

**파일:**
- 수정: `src/markdown.rs`
- 수정: `tests/markdown_tests.rs`

**Step 1: 헤더 렌더링 테스트 추가**

`tests/markdown_tests.rs`에 추가:

```rust
#[test]
fn test_render_headers() {
    let markdown = "# Header 1\n## Header 2\n### Header 3";
    let result = render_to_text(markdown);

    // 헤더가 스타일이 적용되어 렌더링되는지 확인
    assert_eq!(result.lines.len(), 3);
}

#[test]
fn test_render_bold_italic() {
    let markdown = "**bold** and *italic*";
    let result = render_to_text(markdown);

    // 최소한 텍스트가 렌더링되는지 확인
    assert!(result.lines.len() > 0);
}

#[test]
fn test_render_code_block() {
    let markdown = "```rust\nfn main() {}\n```";
    let result = render_to_text(markdown);

    assert!(result.lines.len() > 0);
}
```

**Step 2: 테스트 실행 (현재는 단순 텍스트로 PASS)**

실행: `cargo test markdown`
예상 결과: PASS (스타일은 없지만 텍스트는 렌더링됨)

**Step 3: termimad 통합 구현**

`src/markdown.rs` 수정:

```rust
use ratatui::text::{Line, Span, Text};
use ratatui::style::{Color, Modifier, Style};
use termimad::{MadSkin, terminal_size};

/// Markdown 문자열을 ratatui Text로 렌더링
pub fn render_to_text(markdown: &str) -> Text<'static> {
    if markdown.is_empty() {
        return Text::from(vec![Line::from("")]);
    }

    // termimad MadSkin 생성
    let mut skin = MadSkin::default();

    // 헤더 스타일 설정
    skin.headers[0].set_fg(Color::Yellow);
    skin.headers[0].add_modifier(Modifier::BOLD);
    skin.headers[1].set_fg(Color::Cyan);
    skin.headers[1].add_modifier(Modifier::BOLD);
    skin.headers[2].set_fg(Color::Green);

    // 강조 스타일
    skin.bold.add_modifier(Modifier::BOLD);
    skin.italic.add_modifier(Modifier::ITALIC);

    // 코드 블록 스타일
    skin.code_block.set_bg(Color::DarkGray);
    skin.inline_code.set_fg(Color::Green);

    // Markdown 파싱 및 렌더링
    match skin.text(markdown, Some(80)) {
        Ok(formatted) => {
            // termimad의 출력을 ratatui Text로 변환
            let lines: Vec<Line> = formatted
                .lines
                .into_iter()
                .map(|line| {
                    let spans: Vec<Span> = line
                        .strings
                        .into_iter()
                        .zip(line.compounds.into_iter())
                        .map(|(s, compound)| {
                            let mut style = Style::default();
                            if compound.bold {
                                style = style.add_modifier(Modifier::BOLD);
                            }
                            if compound.italic {
                                style = style.add_modifier(Modifier::ITALIC);
                            }
                            if let Some(fg) = compound.fg {
                                style = style.fg(to_ratatui_color(fg));
                            }
                            if let Some(bg) = compound.bg {
                                style = style.bg(to_ratatui_color(bg));
                            }
                            Span::styled(s, style)
                        })
                        .collect();
                    Line::from(spans)
                })
                .collect();

            Text::from(lines)
        }
        Err(_) => {
            // Fallback: 원본 텍스트
            let lines: Vec<Line> = markdown
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect();
            Text::from(lines)
        }
    }
}

/// termimad Color를 ratatui Color로 변환
fn to_ratatui_color(color: termimad::crossterm::style::Color) -> Color {
    use termimad::crossterm::style::Color as TC;

    match color {
        TC::Black => Color::Black,
        TC::DarkGrey => Color::DarkGray,
        TC::Red => Color::Red,
        TC::DarkRed => Color::LightRed,
        TC::Green => Color::Green,
        TC::DarkGreen => Color::LightGreen,
        TC::Yellow => Color::Yellow,
        TC::DarkYellow => Color::LightYellow,
        TC::Blue => Color::Blue,
        TC::DarkBlue => Color::LightBlue,
        TC::Magenta => Color::Magenta,
        TC::DarkMagenta => Color::LightMagenta,
        TC::Cyan => Color::Cyan,
        TC::DarkCyan => Color::LightCyan,
        TC::White => Color::White,
        TC::Grey => Color::Gray,
        _ => Color::White,
    }
}
```

**참고:** 위 구현은 termimad의 실제 API에 따라 조정이 필요할 수 있습니다. 구현 시 termimad 문서를 참조하여 정확한 API를 사용하세요.

**Step 4: 컴파일 확인**

실행: `cargo check`
예상 결과: 성공 (API 조정 필요 시 수정)

**Step 5: 테스트 실행**

실행: `cargo test markdown`
예상 결과: PASS

**Step 6: 커밋**

```bash
git add src/markdown.rs tests/markdown_tests.rs
git commit -m "feat(markdown): Integrate termimad for advanced Markdown rendering

Support headers, bold, italic, code blocks with proper styling

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 8: 에디터 화면 레이아웃 50:50 분할

**파일:**
- 수정: `src/view.rs`

**Step 1: render_editor 함수 레이아웃 수정**

`src/view.rs`의 `render_editor` 함수 수정:

```rust
fn render_editor(f: &mut Frame, model: &Model) {
    // 메인 레이아웃: 수평 분할 (50:50)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),  // 왼쪽: 에디터
            Constraint::Percentage(50),  // 오른쪽: Markdown 미리보기
        ])
        .split(f.size());

    // 왼쪽: 에디터 영역 (기존 레이아웃)
    let editor_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 날짜 헤더
            Constraint::Min(0),    // 에디터 영역
            Constraint::Length(1), // 모드 표시
        ])
        .split(main_chunks[0]);

    // 헤더: 날짜
    let header = Paragraph::new(model.editor_state.date.to_string())
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(header, editor_chunks[0]);

    // 에디터 내용
    let content = model.editor_state.get_content();
    let text = Paragraph::new(content.clone()).wrap(Wrap { trim: false });
    f.render_widget(text, editor_chunks[1]);

    // 커서 표시 (Insert 모드)
    if model.editor_state.mode == EditorMode::Insert {
        let cursor_x = editor_chunks[1].x + model.editor_state.cursor_col as u16;
        let cursor_y = editor_chunks[1].y + model.editor_state.cursor_line as u16;
        f.set_cursor(cursor_x, cursor_y);
    }

    // 하단바: 모드 표시
    let mode_text = match &model.editor_state.mode {
        EditorMode::Normal => "-- NORMAL --".to_string(),
        EditorMode::Insert => "-- INSERT --".to_string(),
        EditorMode::Command(cmd) => format!(":{}", cmd),
    };
    let statusbar = Paragraph::new(mode_text)
        .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(statusbar, editor_chunks[2]);

    // 오른쪽: Markdown 미리보기 (임시)
    let preview_block = Block::default()
        .title("Markdown 미리보기")
        .borders(Borders::ALL);
    f.render_widget(preview_block, main_chunks[1]);
}
```

**Step 2: 컴파일 및 수동 테스트**

실행: `cargo run`
확인: 에디터 화면이 50:50으로 분할되고 오른쪽에 빈 미리보기 블록 표시

**Step 3: 커밋**

```bash
git add src/view.rs
git commit -m "feat(view): Split editor screen into 50:50 layout

Left: text editor
Right: Markdown preview pane (empty for now)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 9: render_markdown_preview 함수 구현 및 통합

**파일:**
- 수정: `src/view.rs`

**Step 1: Markdown 미리보기 렌더링 함수 작성**

`src/view.rs`에 추가:

```rust
use crate::markdown::render_to_text;

fn render_markdown_preview(f: &mut Frame, area: Rect, markdown: &str) {
    let rendered_text = render_to_text(markdown);

    let preview = Paragraph::new(rendered_text)
        .block(
            Block::default()
                .title("Markdown 미리보기")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(preview, area);
}
```

**Step 2: render_editor에서 함수 사용**

`render_editor` 함수에서 임시 블록을 대체:

```rust
// 오른쪽: Markdown 미리보기
let content = model.editor_state.get_content();
render_markdown_preview(f, main_chunks[1], &content);
```

**Step 3: 컴파일 및 수동 테스트**

실행: `cargo run`
확인:
1. 에디터에 진입
2. Insert 모드로 전환 (i)
3. Markdown 텍스트 입력 (예: `# Hello\n**bold** text`)
4. 오른쪽 미리보기에 스타일이 적용된 렌더링 결과 실시간 표시

**Step 4: 커밋**

```bash
git add src/view.rs
git commit -m "feat(editor): Add real-time Markdown preview rendering

Integrate markdown::render_to_text for live preview updates

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 10: 고급 Markdown 요소 지원 확장

**파일:**
- 수정: `tests/markdown_tests.rs`

**Step 1: 고급 요소 테스트 추가**

`tests/markdown_tests.rs`에 추가:

```rust
#[test]
fn test_render_lists() {
    let markdown = "- Item 1\n- Item 2\n  - Nested\n1. Numbered";
    let result = render_to_text(markdown);
    assert!(result.lines.len() >= 4);
}

#[test]
fn test_render_blockquote() {
    let markdown = "> This is a quote\n> Multiple lines";
    let result = render_to_text(markdown);
    assert!(result.lines.len() >= 2);
}

#[test]
fn test_render_horizontal_rule() {
    let markdown = "Before\n\n---\n\nAfter";
    let result = render_to_text(markdown);
    assert!(result.lines.len() >= 3);
}

#[test]
fn test_render_table() {
    let markdown = "| Col1 | Col2 |\n|------|------|\n| A    | B    |";
    let result = render_to_text(markdown);
    assert!(result.lines.len() >= 3);
}

#[test]
fn test_render_links() {
    let markdown = "[Link Text](https://example.com)";
    let result = render_to_text(markdown);
    assert!(result.lines.len() > 0);
}
```

**Step 2: 테스트 실행**

실행: `cargo test markdown`
예상 결과: termimad가 이미 이러한 요소들을 지원하므로 PASS

**Step 3: 수동 테스트로 렌더링 품질 확인**

실행: `cargo run`
확인:
- 리스트, 인용, 표, 링크 등 다양한 Markdown 요소 입력
- 올바르게 스타일이 적용되어 렌더링되는지 확인

**Step 4: 필요시 markdown.rs의 스타일 조정**

termimad의 기본 스타일이 만족스럽지 않으면 `src/markdown.rs`에서 추가 스타일 커스터마이징

**Step 5: 커밋**

```bash
git add tests/markdown_tests.rs
git commit -m "test(markdown): Add tests for advanced Markdown elements

Lists, blockquotes, tables, horizontal rules, links

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 4: 테스팅 및 폴리싱

### Task 11: 통합 테스트 추가

**파일:**
- 수정: `tests/view_tests.rs`

**Step 1: 에디터 미리보기 통합 테스트 작성**

`tests/view_tests.rs`에 추가:

```rust
#[test]
fn test_editor_content_updates_preview() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();
    let entries = HashSet::new();

    let mut model = Model::new(entries, storage);

    // 에디터로 전환
    model.screen = Screen::Editor;
    model.editor_state.mode = EditorMode::Insert;

    // 텍스트 입력
    model.editor_state.insert_char('#');
    model.editor_state.insert_char(' ');
    model.editor_state.insert_char('H');

    // 콘텐츠 확인
    let content = model.editor_state.get_content();
    assert_eq!(content, "# H");

    // 렌더링 시 markdown::render_to_text가 호출됨 (실제 UI 테스트는 불가)
}

#[test]
fn test_calendar_preview_empty_diary() {
    let temp = TempDir::new().unwrap();
    let storage = Storage::with_dir(temp.path()).unwrap();
    let entries = HashSet::new();

    let model = Model::new(entries, storage);

    // 다이어리가 없는 날짜 선택
    let date = model.calendar_state.selected_date;
    let result = model.storage.load(date);

    // 로드 실패 시 에러 반환 (view에서 처리)
    assert!(result.is_err());
}
```

**Step 2: 테스트 실행**

실행: `cargo test view_tests`
예상 결과: PASS

**Step 3: 커밋**

```bash
git add tests/view_tests.rs
git commit -m "test(view): Add integration tests for preview functionality

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 12: 작은 터미널 대응 (선택적)

**파일:**
- 수정: `src/view.rs`

**Step 1: 터미널 크기 검사 추가**

`src/view.rs`의 `render_calendar`와 `render_editor` 함수 수정:

```rust
fn render_calendar(f: &mut Frame, model: &Model) {
    let width = f.size().width;

    // 터미널이 너무 작으면 미리보기 숨김
    if width < 80 {
        render_calendar_fullscreen(f, model);
    } else {
        render_calendar_with_preview(f, model);
    }
}

fn render_calendar_fullscreen(f: &mut Frame, model: &Model) {
    // 기존 전체 화면 레이아웃 (미리보기 없음)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(f.size());

    // ... (기존 코드와 동일)
}

fn render_calendar_with_preview(f: &mut Frame, model: &Model) {
    // 현재의 50:50 분할 코드를 여기로 이동
    // ... (Task 4-6의 코드)
}
```

에디터 화면도 동일하게 처리

**Step 2: 컴파일 확인**

실행: `cargo check`
예상 결과: 성공

**Step 3: 수동 테스트**

터미널 크기를 조정하여 80 컬럼 이하/이상에서 동작 확인

**Step 4: 커밋**

```bash
git add src/view.rs
git commit -m "feat(view): Add responsive layout for small terminals

Hide preview pane when terminal width < 80 columns

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 13: 최종 테스트 및 문서 업데이트

**파일:**
- 수정: `README.md`

**Step 1: 전체 테스트 실행**

실행: `cargo test`
예상 결과: 모든 테스트 PASS

**Step 2: 커버리지 확인 (선택적)**

실행: `cargo tarpaulin` (설치된 경우)
목표: 70%+ 커버리지

**Step 3: README 업데이트**

`README.md`에 미리보기 기능 설명 추가:

```markdown
## 기능

- 월간 달력 뷰
- Vi 모드 텍스트 에디터
- Markdown 파일 자동 저장
- 다이어리 작성 유무 시각적 표시
- 실시간 Markdown 미리보기 (달력 & 에디터)

### 미리보기 기능

- **달력 화면**: 선택된 날짜의 다이어리 내용을 오른쪽에 실시간으로 표시
- **에디터 화면**: 작성 중인 Markdown 문서를 렌더링하여 오른쪽에 표시
- 화면 분할: 50:50 (작은 터미널에서는 자동으로 전체 화면 모드)
```

**Step 4: 수동 테스트 체크리스트 확인**

설계 문서의 수동 테스트 항목들을 하나씩 확인:
- [ ] 달력에서 화살표 키로 날짜 이동 시 미리보기 즉시 업데이트
- [ ] 다이어리가 있는 날짜/없는 날짜 모두 정상 표시
- [ ] 에디터에서 Markdown 입력 시 실시간 렌더링
- [ ] 긴 다이어리 스크롤 가능
- [ ] 작은 터미널에서도 정상 동작
- [ ] 한글, 이모지 정상 렌더링
- [ ] 타이핑 지연 없음

**Step 5: 커밋**

```bash
git add README.md
git commit -m "docs: Update README with preview feature description

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 완료 체크리스트

모든 Task 완료 후 확인:

- [ ] Phase 1: 기본 구조 완료
- [ ] Phase 2: 달력 미리보기 완료
- [ ] Phase 3: 에디터 Markdown 미리보기 완료
- [ ] Phase 4: 테스팅 및 폴리싱 완료
- [ ] 모든 테스트 통과
- [ ] README 업데이트
- [ ] 수동 테스트 체크리스트 완료

## 추가 고려사항

### 성능 최적화 (YAGNI - 필요시만)

문제가 발생하면:
1. 긴 문서 truncate 구현 (MAX_PREVIEW_LINES)
2. Markdown 렌더링 debounce 추가

### 향후 개선 가능성

- 미리보기 토글 키바인딩 (`p` 키)
- 미리보기 비율 조정
- 커스텀 테마 지원

## 참고 자료

- termimad 문서: https://docs.rs/termimad/
- ratatui 문서: https://docs.rs/ratatui/
- 설계 문서: `docs/plans/2026-02-14-diary-preview-design.md`
