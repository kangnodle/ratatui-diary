//! 에디터 화면 렌더링.

use crate::{app::Model,
            editor::state::{EditorState,
                            EditorSubMode}};
use ratatui::{Frame,
              layout::{Constraint,
                       Direction,
                       Layout,
                       Rect},
              style::{Color,
                      Modifier,
                      Style},
              text::{Line,
                     Span},
              widgets::{Block,
                        Borders,
                        Paragraph,
                        Wrap}};
use unicode_width::UnicodeWidthChar;

/// 선택 영역 타입: ((시작 라인, 시작 컬럼), (끝 라인, 끝 컬럼))
type SelectionRange = Option<((usize, usize), (usize, usize))>;

/// 에디터 화면의 키바인딩 도움말 텍스트 생성
pub fn build_editor_keybindings(state: &EditorState) -> String {
    match &state.submode {
        | None => crate::settings::Language::Korean.tr(crate::i18n::Text::EditorKeys).to_string(),
        | Some(EditorSubMode::CtrlX) => crate::settings::Language::Korean.tr(crate::i18n::Text::CommandKeys).to_string(),
        | Some(EditorSubMode::Search) => crate::settings::Language::Korean.tr(crate::i18n::Text::SearchKeys).to_string(),
    }
}

pub fn render(f: &mut Frame, model: &Model) {
    let lang = model.settings.language;
    let palette = model.settings.theme.palette();
    // 메인 레이아웃: 수평 분할 (50:50)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // 왼쪽: 에디터
            Constraint::Percentage(50), // 오른쪽: 일기 미리보기
        ])
        .split(f.area());

    // 왼쪽: 에디터 영역 (기존 레이아웃)
    let editor_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 날짜 헤더
            Constraint::Min(0),    // 에디터 영역
            Constraint::Length(3), // 상태바 + 키바인딩 도움말
        ])
        .split(main_chunks[0]);

    // 헤더: 날짜
    let header = Paragraph::new(model.editor_state.date.to_string()).style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(header, editor_chunks[0]);

    // 에디터 내용 - 스타일이 적용된 라인들로 렌더링
    let styled_lines = render_editor_content(&model.editor_state, palette);
    let text = Paragraph::new(styled_lines).wrap(Wrap {
        trim: false,
    });
    f.render_widget(text, editor_chunks[1]);

    // 커서 표시
    let display_width: u16 = if model.editor_state.cursor_line < model.editor_state.content.len() {
        model.editor_state.content[model.editor_state.cursor_line]
            .chars()
            .take(model.editor_state.cursor_col)
            .map(|c| match UnicodeWidthChar::width(c) {
                | Some(w) => w as u16,
                | None => 0,
            })
            .sum()
    } else {
        0
    };
    let cursor_x = editor_chunks[1].x + display_width;
    let cursor_y = editor_chunks[1].y + model.editor_state.cursor_line as u16;
    if !model.show_settings && !model.show_error_popup {
        f.set_cursor_position((cursor_x, cursor_y));
    }

    // 하단바: 상태 정보와 키바인딩 표시
    let mode_text = localize_status(build_status_text(&model.editor_state), lang);
    let keybindings = lang.tr(match model.editor_state.submode {
        | None => crate::i18n::Text::EditorKeys,
        | Some(EditorSubMode::CtrlX) => crate::i18n::Text::CommandKeys,
        | Some(EditorSubMode::Search) => crate::i18n::Text::SearchKeys,
    });
    let status_text = format!(
        "{} | F2:{} · {} / {}\n{}",
        mode_text,
        lang.tr(crate::i18n::Text::Settings),
        lang.tr(crate::i18n::Text::LanguageName),
        model.settings.theme.label(lang),
        keybindings
    );
    let statusbar = Paragraph::new(status_text).style(Style::default().add_modifier(Modifier::BOLD)).wrap(Wrap {
        trim: false,
    });
    f.render_widget(statusbar, editor_chunks[2]);

    // 오른쪽: 일기 미리보기
    let content = model.editor_state.get_content();
    render_diary_preview(f, main_chunks[1], &content, model);
}

fn render_diary_preview(f: &mut Frame, area: Rect, content: &str, model: &Model) {
    let markdown = model.storage.is_markdown(model.editor_state.date).unwrap_or(false);
    let (rendered_text, title) = if markdown {
        (crate::markdown::render_to_text(content), crate::i18n::Text::MarkdownPreview)
    } else {
        (crate::org::render_to_text(content), crate::i18n::Text::OrgPreview)
    };

    let preview = Paragraph::new(rendered_text)
        .block(
            Block::default()
                .title(model.settings.language.tr(title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(model.settings.theme.palette().accent)),
        )
        .style(model.settings.theme.palette().base())
        .wrap(Wrap {
            trim: false,
        });

    f.render_widget(preview, area);
}

/// 에디터 내용을 스타일이 적용된 Line 벡터로 변환
fn render_editor_content(editor_state: &EditorState, palette: crate::settings::Palette) -> Vec<Line<'static>> {
    let selection_range = editor_state.get_selection_range();

    editor_state
        .content
        .iter()
        .enumerate()
        .map(|(line_idx, line_text)| {
            let mut spans = Vec::new();
            let chars: Vec<char> = line_text.chars().collect();

            let mut col_idx = 0;
            while col_idx < chars.len() {
                let ch = chars[col_idx];

                let style = get_char_style(
                    line_idx,
                    col_idx,
                    &selection_range,
                    &editor_state.search_matches,
                    editor_state.current_match_index,
                    &editor_state.search_pattern,
                    palette,
                );

                spans.push(Span::styled(ch.to_string(), style));
                col_idx += 1;
            }

            if spans.is_empty() {
                spans.push(Span::raw(" "));
            }

            Line::from(spans)
        })
        .collect()
}

/// 문자의 스타일 결정 (선택 영역, 검색 매치 등)
fn get_char_style(
    line: usize,
    col: usize,
    selection_range: &SelectionRange,
    search_matches: &[(usize, usize)],
    current_match_index: usize,
    search_pattern: &str,
    palette: crate::settings::Palette,
) -> Style {
    let pattern_len = search_pattern.chars().count();

    let (is_match, is_current) = is_search_match(line, col, search_matches, current_match_index, pattern_len);

    let in_selection = is_in_selection(line, col, selection_range);

    if is_current {
        Style::default().bg(palette.current_search).fg(Color::Black).add_modifier(Modifier::BOLD)
    } else if in_selection {
        Style::default().bg(palette.selection).fg(palette.fg)
    } else if is_match {
        Style::default().bg(palette.search).fg(Color::Black)
    } else {
        palette.base()
    }
}

/// 특정 위치가 선택 영역 내에 있는지 확인
fn is_in_selection(line: usize, col: usize, selection_range: &SelectionRange) -> bool {
    if let Some(((start_line, start_col), (end_line, end_col))) = selection_range {
        if line < *start_line || line > *end_line {
            return false;
        }

        if *start_line == *end_line {
            col >= *start_col && col < *end_col
        } else if line == *start_line {
            col >= *start_col
        } else if line == *end_line {
            col < *end_col
        } else {
            true
        }
    } else {
        false
    }
}

/// 특정 위치가 검색 매치인지 확인
fn is_search_match(line: usize, col: usize, matches: &[(usize, usize)], current_match_index: usize, pattern_len: usize) -> (bool, bool) {
    for (idx, (match_line, match_col)) in matches.iter().enumerate() {
        if *match_line == line && col >= *match_col && col < match_col + pattern_len {
            let is_current = idx == current_match_index;
            return (true, is_current);
        }
    }
    (false, false)
}

/// 상태바 텍스트 생성
fn build_status_text(editor_state: &EditorState) -> String {
    // Submode 표시
    let submode_text = match &editor_state.submode {
        | Some(EditorSubMode::CtrlX) => "-- C-x --",
        | Some(EditorSubMode::Search) => {
            let label = if editor_state.search_backward { "역방향 검색:" } else { "검색:" };
            return format!("{} {}", label, editor_state.search_pattern);
        },
        | None => "-- EMACS --",
    };

    // 검색 매치 정보 표시
    let search_info = if !editor_state.search_matches.is_empty() {
        format!(" | 검색: {}/{} 매치", editor_state.current_match_index + 1, editor_state.search_matches.len())
    } else {
        String::new()
    };

    format!("{}{}", submode_text, search_info)
}

fn localize_status(text: String, lang: crate::settings::Language) -> String {
    if lang == crate::settings::Language::English {
        text.replace("역방향 검색:", "I-search backward:")
            .replace("검색:", "Search:")
            .replace("매치", "matches")
    } else {
        text
    }
}
