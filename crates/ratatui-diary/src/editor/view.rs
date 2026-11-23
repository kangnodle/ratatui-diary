use crate::{app::Model,
            editor::state::{EditorState,
                            EditorSubMode}};
use ratatui::{Frame,
              layout::{Constraint,
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

type SelectionRange = Option<((usize, usize), (usize, usize))>;

pub fn build_editor_keybindings(state: &EditorState) -> String {
    match &state.submode {
        | None => crate::settings::Language::Korean.tr(crate::i18n::Text::EditorKeys).to_string(),
        | Some(EditorSubMode::CtrlX) => crate::settings::Language::Korean.tr(crate::i18n::Text::CommandKeys).to_string(),
        | Some(EditorSubMode::Search) => crate::settings::Language::Korean.tr(crate::i18n::Text::SearchKeys).to_string(),
    }
}

pub fn render(f: &mut Frame, model: &Model) {
    let palette = crate::theme::palette(model.settings.theme);
    let editor_chunks = screen_chunks(model, f.area());

    let header = Paragraph::new(model.editor_state.date.to_string()).style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(header, editor_chunks[0]);

    let display_width: usize = if model.editor_state.cursor_line < model.editor_state.content.len() {
        model.editor_state.content[model.editor_state.cursor_line]
            .chars()
            .take(model.editor_state.cursor_col)
            .map(|c| match UnicodeWidthChar::width(c) {
                | Some(w) => w,
                | None => 0,
            })
            .sum()
    } else {
        0
    };
    let scroll_x = display_width.saturating_sub(usize::from(editor_chunks[1].width.saturating_sub(1)));
    let scroll_y = model
        .editor_state
        .cursor_line
        .saturating_sub(usize::from(editor_chunks[1].height.saturating_sub(1)));
    let styled_lines = render_editor_content(&model.editor_state, palette, editor_chunks[1], scroll_x, scroll_y);
    f.render_widget(Paragraph::new(styled_lines), editor_chunks[1]);
    let cursor_x = editor_chunks[1].x.saturating_add((display_width - scroll_x) as u16);
    let cursor_y = editor_chunks[1].y.saturating_add((model.editor_state.cursor_line - scroll_y) as u16);
    if !model.show_settings && !model.show_error_popup && editor_chunks[1].width > 0 && editor_chunks[1].height > 0 {
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let helix = model.settings.keybindings == crate::settings::Keybindings::Helix;
    f.render_widget(Paragraph::new(footer(model, editor_chunks[2].width)), editor_chunks[2]);
    if helix
        && matches!(
            model.editor_state.helix.prefix,
            Some(super::helix::Prefix::Command | super::helix::Prefix::Search)
        )
        && !model.show_settings
        && !model.show_error_popup
        && editor_chunks[2].width > 0
        && editor_chunks[2].height > 0
    {
        let prompt_width: usize = model
            .editor_state
            .helix
            .prompt
            .chars()
            .take(model.editor_state.helix.prompt_cursor)
            .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
            .sum();
        f.set_cursor_position((
            editor_chunks[2].x + (prompt_width + 1).min(usize::from(editor_chunks[2].width - 1)) as u16,
            editor_chunks[2].y,
        ));
    }

    let content = model.editor_state.get_content();
    render_diary_preview(f, editor_chunks[3], &content, model);
}

fn footer(model: &Model, width: u16) -> ratatui::text::Text<'static> {
    let lang = model.settings.language;
    let palette = crate::theme::palette(model.settings.theme);
    let helix = model.settings.keybindings == crate::settings::Keybindings::Helix;
    let mode_text = if helix {
        super::helix::status(&model.editor_state)
    } else {
        localize_status(build_status_text(&model.editor_state), lang)
    };
    let keybindings = if helix {
        super::helix::help(&model.editor_state, lang)
    } else {
        lang.tr(match model.editor_state.submode {
            | None => crate::i18n::Text::EditorKeys,
            | Some(EditorSubMode::CtrlX) => crate::i18n::Text::CommandKeys,
            | Some(EditorSubMode::Search) => crate::i18n::Text::SearchKeys,
        })
    };
    let help = format!("F2:{} | {}", lang.tr(crate::i18n::Text::Settings), keybindings);
    let mut text = crate::key_help::build(&help, width, palette);
    text.lines.insert(
        0,
        Line::from(format!(
            "{} · {} / {}",
            mode_text,
            lang.tr(crate::i18n::Text::LanguageName),
            model.settings.theme.label(lang)
        )),
    );
    text
}

fn screen_chunks(model: &Model, area: Rect) -> [Rect; 4] {
    let main = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length((footer(model, main[0].width).height() as u16).min(main[0].height.saturating_sub(2))),
    ])
    .split(main[0]);
    [chunks[0], chunks[1], chunks[2], main[1]]
}

pub fn viewport_height(model: &Model, area: Rect) -> usize { usize::from(screen_chunks(model, area)[1].height).max(1) }

fn render_diary_preview(f: &mut Frame, area: Rect, content: &str, model: &Model) {
    let markdown = model.editor_state.format == diary_domain::EntryFormat::Markdown;
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
                .border_style(Style::default().fg(crate::theme::palette(model.settings.theme).accent)),
        )
        .style(crate::theme::palette(model.settings.theme).base())
        .wrap(Wrap {
            trim: false,
        });

    f.render_widget(preview, area);
}

fn render_editor_content(editor_state: &EditorState, palette: crate::settings::Palette, area: Rect, scroll_x: usize, scroll_y: usize) -> Vec<Line<'static>> {
    let selection_range = editor_state.get_selection_range();
    let pattern_len = editor_state.search_pattern.chars().count();

    editor_state
        .content
        .iter()
        .enumerate()
        .skip(scroll_y)
        .take(usize::from(area.height))
        .map(|(line_idx, line_text)| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut display_col = 0;
            let right = scroll_x + usize::from(area.width);

            for (col_idx, ch) in line_text.chars().enumerate() {
                let width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if width == 0 {
                    if display_col > scroll_x
                        && display_col <= right
                        && !ch.is_control()
                        && let Some(span) = spans.last_mut()
                    {
                        span.content.to_mut().push(ch);
                    }
                    continue;
                }
                if display_col >= right {
                    break;
                }
                let next_col = display_col + width;
                if next_col <= scroll_x {
                    display_col = next_col;
                    continue;
                }

                let style = get_char_style(
                    line_idx,
                    col_idx,
                    &selection_range,
                    &editor_state.search_matches,
                    editor_state.current_match_index,
                    pattern_len,
                    palette,
                );

                let visible = if display_col < scroll_x || next_col > right {
                    " ".repeat(next_col.min(right) - display_col.max(scroll_x))
                } else {
                    ch.to_string()
                };
                spans.push(Span::styled(visible, style));
                display_col = next_col;
            }

            if spans.is_empty() {
                spans.push(Span::raw(" "));
            }

            Line::from(spans)
        })
        .collect()
}

fn get_char_style(
    line: usize,
    col: usize,
    selection_range: &SelectionRange,
    search_matches: &[(usize, usize)],
    current_match_index: usize,
    pattern_len: usize,
    palette: crate::settings::Palette,
) -> Style {
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

fn is_search_match(line: usize, col: usize, matches: &[(usize, usize)], current_match_index: usize, pattern_len: usize) -> (bool, bool) {
    if pattern_len == 0 {
        return (false, false);
    }
    let earliest = (line, col.saturating_sub(pattern_len - 1));
    let index = matches.partition_point(|position| *position < earliest);
    if let Some(&(match_line, match_col)) = matches.get(index)
        && match_line == line
        && match_col <= col
    {
        return (true, index == current_match_index);
    }
    (false, false)
}

fn build_status_text(editor_state: &EditorState) -> String {
    let submode_text = match &editor_state.submode {
        | Some(EditorSubMode::CtrlX) => "-- C-x --",
        | Some(EditorSubMode::Search) => {
            let label = if editor_state.search_backward { "역방향 검색:" } else { "검색:" };
            return format!("{} {}", label, editor_state.search_pattern);
        },
        | None => "-- EMACS --",
    };

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
