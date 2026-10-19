//! 최상위 렌더링 디스패처 + 공용 오버레이(에러 팝업).

use crate::{app::{Model,
                  Screen},
            calendar,
            editor};
use ratatui::{Frame,
              layout::{Constraint,
                       Direction,
                       Layout,
                       Rect},
              style::Style,
              widgets::{Block,
                        Borders,
                        Clear,
                        Paragraph,
                        Wrap}};

pub fn view(f: &mut Frame, model: &Model) {
    f.render_widget(Block::default().style(model.settings.theme.palette().base()), f.area());
    match model.screen {
        | Screen::Calendar => calendar::view::render(f, model),
        | Screen::Editor => editor::view::render(f, model),
    }

    if model.show_settings {
        render_settings(f, model);
    }

    // 에러 팝업
    if model.show_error_popup {
        render_error_popup(f, model);
    }
}

fn render_error_popup(f: &mut Frame, model: &Model) {
    let area = centered_rect(60, 20, f.area());

    let error_msg = match model.error_message.as_deref() {
        | Some(msg) => msg,
        | None => model.settings.language.tr(crate::i18n::Text::UnknownError),
    };
    let popup = Paragraph::new(error_msg)
        .block(
            Block::default()
                .title(model.settings.language.tr(crate::i18n::Text::ErrorTitle))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(model.settings.theme.palette().error)),
        )
        .style(model.settings.theme.palette().base())
        .wrap(Wrap {
            trim: true,
        });

    f.render_widget(Clear, area);
    f.render_widget(popup, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_settings(f: &mut Frame, model: &Model) {
    use crate::settings::Theme;
    let lang = model.settings.language;
    let palette = model.settings.theme.palette();
    let area = centered_rect(80, 60, f.area());
    let theme = match model.settings.theme {
        | Theme::System => lang.tr(crate::i18n::Text::System),
        | Theme::Dark => lang.tr(crate::i18n::Text::Dark),
        | Theme::Light => lang.tr(crate::i18n::Text::Light),
    };
    let values = [
        format!("{}: {}", lang.tr(crate::i18n::Text::Language), lang.tr(crate::i18n::Text::LanguageName)),
        format!("{}: {}", lang.tr(crate::i18n::Text::Theme), theme),
    ];
    let mut lines: Vec<ratatui::text::Line> = values
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            let style = if i == model.settings_row {
                Style::default().fg(palette.selected_fg).bg(palette.selected_bg)
            } else {
                palette.base()
            };
            ratatui::text::Line::styled(format!("{} {}", if i == model.settings_row { ">" } else { " " }, value), style)
        })
        .collect();
    lines.push("".into());
    lines.push(lang.tr(crate::i18n::Text::SettingsNavigation).into());
    lines.push(lang.tr(crate::i18n::Text::SettingsHelp).into());
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .style(palette.base())
            .wrap(Wrap {
                trim: false,
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(lang.tr(crate::i18n::Text::Settings))
                    .border_style(Style::default().fg(palette.accent)),
            ),
        area,
    );
}
