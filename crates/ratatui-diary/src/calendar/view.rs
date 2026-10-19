//! 달력 화면 렌더링.

use crate::{app::Model,
            calendar::state::CalendarState};
use ratatui::{Frame,
              layout::{Alignment,
                       Constraint,
                       Direction,
                       Layout,
                       Rect},
              style::{Modifier,
                      Style},
              text::{Line,
                     Span},
              widgets::{Block,
                        Borders,
                        Paragraph,
                        Wrap}};

/// 달력 화면의 키바인딩 도움말 텍스트 생성
pub fn build_calendar_keybindings(_state: &CalendarState) -> String { crate::settings::Language::Korean.tr(crate::i18n::Text::CalendarKeys).to_string() }

pub fn render(f: &mut Frame, model: &Model) {
    let lang = model.settings.language;
    let palette = crate::theme::palette(model.settings.theme);
    // 메인 레이아웃: 수평 분할 (50:50)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // 왼쪽: 달력
            Constraint::Percentage(50), // 오른쪽: 미리보기
        ])
        .split(f.area());

    // 왼쪽: 달력 영역 (기존 레이아웃)
    let calendar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 헤더
            Constraint::Min(0),    // 달력
            Constraint::Length(5), // 상태바
        ])
        .split(main_chunks[0]);

    // 헤더
    let header = Paragraph::new(match lang {
        | crate::settings::Language::Korean => format!("{}년 {}월", model.calendar_state.current_year, model.calendar_state.current_month),
        | crate::settings::Language::English => format!("{}-{:02}", model.calendar_state.current_year, model.calendar_state.current_month),
    })
    .alignment(Alignment::Center)
    .style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(header, calendar_chunks[0]);

    // 달력 그리드
    render_calendar_grid(f, calendar_chunks[1], model);

    // 상태바 - 동적 키바인딩
    let keybindings = if model.settings.keybindings == crate::settings::Keybindings::Helix {
        if model.editor_state.helix.prefix == Some(crate::editor::helix::Prefix::Command) {
            format!(
                "{}\n{}",
                crate::editor::helix::status(&model.editor_state),
                crate::editor::helix::help(&model.editor_state, lang)
            )
        } else {
            lang.text(
                "h/j/k/l:이동 | Enter:편집\nH/L · PgUp/Dn:이전/다음 월\n{/} · C-PgUp/Dn:이전/다음 년\n:q:종료 | F2:설정",
                "h/j/k/l:Move | Enter:Edit\nH/L · PgUp/Dn:Previous/Next month\n{/} · C-PgUp/Dn:Previous/Next year\n:q:Quit | F2:Settings",
            )
            .to_string()
        }
    } else if model.editor_state.submode == Some(crate::editor::state::EditorSubMode::CtrlX) {
        lang.text("C-x C-c:저장 후 종료 | C-g:취소", "C-x C-c:Save & quit | C-g:Cancel").to_string()
    } else {
        format!(
            "F2:{} · {} / {}\n{}",
            lang.tr(crate::i18n::Text::Settings),
            lang.tr(crate::i18n::Text::LanguageName),
            model.settings.theme.label(lang),
            lang.tr(crate::i18n::Text::CalendarKeys)
        )
    };
    let statusbar = Paragraph::new(keybindings).alignment(Alignment::Center);
    f.render_widget(statusbar, calendar_chunks[2]);

    // 오른쪽: 미리보기 영역
    let selected_date = model.calendar_state.selected_date;
    let entry = model.preview.as_ref().filter(|(date, _)| *date == selected_date).map(|(_, entry)| entry);
    let preview_content = entry.map_or_else(|| lang.tr(crate::i18n::Text::EmptyDiary), |entry| entry.content.as_str());
    let preview_text = if entry.is_some_and(|entry| entry.format == diary_domain::EntryFormat::Markdown) {
        crate::markdown::render_to_text(preview_content)
    } else {
        crate::org::render_to_text(preview_content)
    };
    render_preview_pane(
        f,
        main_chunks[1],
        preview_text,
        &format!("{}: {}", lang.tr(crate::i18n::Text::Diary), selected_date),
        palette,
    );
}

fn render_preview_pane(f: &mut Frame, area: Rect, content: ratatui::text::Text<'_>, title: &str, palette: crate::settings::Palette) {
    let text = Paragraph::new(content)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.accent)),
        )
        .wrap(Wrap {
            trim: false,
        })
        .style(palette.base());

    f.render_widget(text, area);
}

fn render_calendar_grid(f: &mut Frame, area: Rect, model: &Model) {
    use chrono::{Datelike,
                 NaiveDate};

    let year = model.calendar_state.current_year;
    let month = model.calendar_state.current_month;

    // 요일 헤더
    let weekdays = match model.settings.language {
        | crate::settings::Language::Korean => ["일", "월", "화", "수", "목", "금", "토"],
        | crate::settings::Language::English => ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
    };
    let mut lines = vec![Line::from(
        weekdays
            .iter()
            .map(|&day| {
                let padding = 4 - unicode_width::UnicodeWidthStr::width(day);
                Span::raw(format!("{}{}{}", " ".repeat(padding / 2), day, " ".repeat(padding.div_ceil(2))))
            })
            .collect::<Vec<_>>(),
    )];

    // 월의 첫날
    let first_day = match NaiveDate::from_ymd_opt(year, month, 1) {
        | Some(date) => date,
        | None => return,
    };
    let weekday = first_day.weekday().num_days_from_sunday() as usize;

    // 달력 생성
    let next_month_first = match first_day.with_month(month + 1) {
        | Some(date) => date,
        | None => match first_day.with_year(year + 1) {
            | Some(next_year) => match next_year.with_month(1) {
                | Some(date) => date,
                | None => return,
            },
            | None => return,
        },
    };
    let days_in_month = match next_month_first.pred_opt() {
        | Some(last_day) => last_day.day(),
        | None => return,
    };

    let mut week = vec![Span::raw("    "); 7];
    let mut day = 1;

    // 첫 주 빈 칸 채우기
    for slot in week.iter_mut().take(7).skip(weekday) {
        match NaiveDate::from_ymd_opt(year, month, day) {
            | Some(date) => {
                *slot = format_day(day, date, model);
            },
            | None => {},
        }
        day += 1;
    }
    lines.push(Line::from(week.clone()));

    // 나머지 주
    while day <= days_in_month {
        week = vec![Span::raw("    "); 7];
        for slot in week.iter_mut().take(7) {
            if day <= days_in_month {
                match NaiveDate::from_ymd_opt(year, month, day) {
                    | Some(date) => {
                        *slot = format_day(day, date, model);
                    },
                    | None => {},
                }
                day += 1;
            }
        }
        lines.push(Line::from(week.clone()));
    }

    let calendar = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    f.render_widget(calendar, area);
}

fn format_day(day: u32, date: chrono::NaiveDate, model: &Model) -> Span<'static> {
    let has_entry = model.diary_entries.entries.contains(&date);
    let is_selected = date == model.calendar_state.selected_date;
    let is_today = date == model.today;

    let palette = crate::theme::palette(model.settings.theme);
    let mut style = palette.base();

    if has_entry {
        style = style.fg(palette.entry).add_modifier(Modifier::BOLD);
    }
    if is_selected {
        style = style.fg(palette.selected_fg).bg(palette.selected_bg);
    }
    if is_today {
        style = style.add_modifier(Modifier::UNDERLINED);
    }

    let marker = if has_entry { "●" } else { " " };
    Span::styled(format!("{:>2}{} ", day, marker), style)
}
