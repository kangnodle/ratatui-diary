use crate::theme::Palette;
use ratatui::text::{Line,
                    Span,
                    Text};
use unicode_width::{UnicodeWidthChar,
                    UnicodeWidthStr};

/// Existing localized help uses `key:description`, separated by pipes or
/// newlines.
pub(crate) fn build(help: &str, width: u16, palette: Palette) -> Text<'static> {
    let pairs: Vec<_> = help
        .split(['|', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|entry| entry.rsplit_once(':').unwrap_or(("", entry)))
        .collect();
    let key_width = pairs.iter().map(|(key, _)| key.width()).max().unwrap_or(0);
    let description_width = pairs.iter().map(|(_, description)| description.width()).max().unwrap_or(0);
    let width = usize::from(width);
    let columns = if pairs.len() > 1 && width >= 80.max((key_width + 2 + description_width) * 2 + 4) {
        2
    } else {
        1
    };
    let column_width = width.saturating_sub((columns - 1) * 4) / columns;
    let key_width = key_width.min(column_width.saturating_sub(3));
    let description_width = column_width.saturating_sub(key_width + 2).max(1);
    let mut lines = Vec::new();
    for row in pairs.chunks(columns) {
        let cells: Vec<_> = row
            .iter()
            .map(|(key, description)| (wrap(key, key_width.max(1)), wrap(description, description_width)))
            .collect();
        let height = cells.iter().map(|(key, description)| key.len().max(description.len())).max().unwrap_or(0);
        for i in 0 .. height {
            let mut spans = Vec::new();
            for (column, (keys, descriptions)) in cells.iter().enumerate() {
                let key = keys.get(i).map(String::as_str).unwrap_or("");
                let description = descriptions.get(i).map(String::as_str).unwrap_or("");
                spans.push(Span::styled(key.to_owned(), palette.base().fg(palette.accent)));
                spans.push(Span::raw(" ".repeat(key_width.saturating_sub(key.width()) + 2)));
                spans.push(Span::raw(description.to_owned()));
                if column + 1 < cells.len() {
                    spans.push(Span::raw(" ".repeat(description_width.saturating_sub(description.width()) + 4)));
                }
            }
            lines.push(Line::from(spans));
        }
    }
    Text::from(lines).style(palette.base())
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    let mut used = 0;
    for ch in text.chars() {
        let char_width = ch.width().unwrap_or(0);
        if used + char_width > width && used > 0 {
            let line = lines.last_mut().unwrap();
            let rest = if let Some(space) = line.rfind(' ') {
                let rest = line.split_off(space + 1);
                line.pop();
                rest
            } else {
                String::new()
            };
            used = rest.width();
            lines.push(rest);
        }
        if ch == ' ' && used == 0 {
            continue;
        }
        lines.last_mut().unwrap().push(ch);
        used += char_width;
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{settings::Theme,
                theme::palette};

    #[test]
    fn columns_alignment_and_key_only_color() {
        let palette = palette(Theme::Dark);
        let help = "F2:Settings | Enter:Edit | C-g:Cancel";
        let narrow = build(help, 24, palette);
        let wide = build(help, 80, palette);
        assert_eq!(narrow.height(), 3);
        assert_eq!(wide.height(), 2);
        assert_eq!(narrow.lines[0].to_string(), "F2     Settings");
        assert_eq!(wide.lines[0].to_string().find("Enter"), Some(42));
        assert_eq!(wide.lines[0].spans[0].style.fg, Some(palette.accent));
        assert_eq!(wide.lines[0].spans[2].style.fg, None);
    }

    #[test]
    fn colon_keys_unicode_and_unbound_notes_survive() {
        let help = ":w:저장 | ::명령 | 정규식 검색 | Enter:확정";
        let text = build(help, 16, palette(Theme::Light));
        assert_eq!(text.lines[0].spans[0].content, ":w");
        assert_eq!(text.lines[1].spans[0].content, ":");
        assert!(text.lines.iter().all(|line| line.width() <= 16));
        let descriptions = text.lines.iter().map(|line| line.spans[2].content.as_ref()).collect::<String>();
        assert!(descriptions.contains("정규식검색"));
        for width in 0 ..= 80 {
            build(help, width, palette(Theme::System));
        }
    }
}
