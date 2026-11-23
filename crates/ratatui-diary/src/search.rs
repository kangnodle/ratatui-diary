use crate::Model;
use ratatui::{Frame,
              layout::{Constraint,
                       Layout},
              text::Line,
              widgets::{Block,
                        Borders,
                        List,
                        ListItem,
                        ListState,
                        Paragraph,
                        Wrap}};
use unicode_width::UnicodeWidthChar;

pub fn render(frame: &mut Frame, model: &Model) {
    let lang = model.settings.language;
    let palette = crate::theme::palette(model.settings.theme);
    let state = &model.search;
    let areas = Layout::vertical([Constraint::Length(3), Constraint::Min(0), Constraint::Length(5)]).split(frame.area());
    let title = lang.text("전체 일기 검색 · 로컬 의미 + 문자열", "All diaries · Local semantic + literal search");
    let width = usize::from(areas[0].width.saturating_sub(3));
    let mut visible = Vec::new();
    let mut used = 0;
    for ch in state.query.chars().rev() {
        let size = ch.width().unwrap_or(0);
        if used + size > width {
            break;
        }
        visible.push(ch);
        used += size;
    }
    let query: String = visible.into_iter().rev().collect();
    frame.render_widget(
        Paragraph::new(query).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(palette.base().fg(palette.accent)),
        ),
        areas[0],
    );
    if state.editing && areas[0].width > 2 && areas[0].height > 2 {
        frame.set_cursor_position((areas[0].x + 1 + used as u16, areas[0].y + 1));
    }
    let panes = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(areas[1]);
    let items: Vec<_> = state
        .results
        .iter()
        .map(|hit| {
            let kind = if hit.literal {
                lang.text("문자열", "Literal")
            } else {
                lang.text("의미 유사", "Related")
            };
            ListItem::new(vec![
                Line::from(format!("{} · {}:{} · {kind}", hit.date, hit.line + 1, hit.col + 1)),
                Line::from(
                    hit.excerpt
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .chars()
                        .take(100)
                        .collect::<String>(),
                ),
            ])
        })
        .collect();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{} ({})", lang.text("검색 결과", "Results"), state.results.len())),
        )
        .highlight_style(palette.base().fg(palette.selected_fg).bg(palette.selected_bg))
        .highlight_symbol("› ");
    let mut selection = ListState::default().with_selected((!state.results.is_empty()).then_some(state.selected));
    frame.render_stateful_widget(list, panes[0], &mut selection);
    let detail = state.results.get(state.selected).map(|hit| hit.excerpt.as_str()).unwrap_or_else(|| {
        if state.is_busy() {
            lang.text(
                "검색 중… 첫 검색은 모델 다운로드·색인으로 시간이 걸릴 수 있습니다.",
                "Searching… The first search may take a while to download the model and index diaries.",
            )
        } else if state.editing {
            lang.text("검색어를 입력하고 Enter를 누르세요.", "Type a query and press Enter.")
        } else {
            lang.text("검색 결과가 없습니다.", "No results.")
        }
    });
    frame.render_widget(
        Paragraph::new(detail)
            .wrap(Wrap {
                trim: false,
            })
            .block(Block::default().borders(Borders::ALL).title(lang.text("관련 구절 · 문맥", "Passage · Context"))),
        panes[1],
    );
    let action = if state.editing {
        lang.text("Enter:검색", "Enter:Search")
    } else {
        lang.text("Enter:해당 위치 열기", "Enter:Open passage")
    };
    let mut help = format!(
        "{action} | Tab:{} | ↑/↓ · C-p/n:{} | Esc · C-g:{}",
        lang.text("입력/결과", "Query/results"),
        lang.text("선택", "Select"),
        lang.text("달력", "Calendar")
    );
    if model.settings.keybindings == crate::settings::Keybindings::Helix {
        help.push_str(lang.text(" | j/k:결과 이동", " | j/k:Select result"));
    }
    if state.is_busy() {
        help.push_str(lang.text("\n검색 중…", "\nSearching…"));
    }
    if let Some(notice) = &state.notice {
        help.push('\n');
        help.push_str(notice);
    }
    frame.render_widget(
        Paragraph::new(help).wrap(Wrap {
            trim: false,
        }),
        areas[2],
    );
}
