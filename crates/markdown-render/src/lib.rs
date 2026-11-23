#[allow(unused_imports)]
use ratatui::style::Color;
use ratatui::text::{Line,
                    Text};
use termimad::{FmtText,
               MadSkin};

pub fn render_to_text(markdown: &str) -> Text<'static> {
    if markdown.is_empty() {
        return Text::from(vec![Line::from("")]);
    }

    let skin = create_skin();

    let fmt_text = match std::panic::catch_unwind(|| FmtText::from(&skin, markdown, None)) {
        | Ok(text) => text,
        | Err(_) => {
            let lines: Vec<Line> = markdown.lines().map(|line| Line::from(line.to_string())).collect();
            return Text::from(lines);
        },
    };

    convert_fmt_text_to_ratatui(fmt_text)
}

fn create_skin() -> MadSkin {
    use termimad::crossterm::style::{Attribute,
                                     Color as TermColor};

    let mut skin = MadSkin::default();

    skin.headers[0].compound_style.set_fg(TermColor::Yellow);
    skin.headers[0].compound_style.add_attr(Attribute::Bold);
    skin.headers[1].compound_style.set_fg(TermColor::Cyan);
    skin.headers[1].compound_style.add_attr(Attribute::Bold);
    skin.headers[2].compound_style.set_fg(TermColor::Green);
    skin.headers[2].compound_style.add_attr(Attribute::Bold);

    skin.bold.add_attr(Attribute::Bold);
    skin.italic.add_attr(Attribute::Italic);

    skin.inline_code.set_fg(TermColor::Green);

    skin.code_block.compound_style.set_fg(TermColor::Green);
    skin.code_block.compound_style.set_bg(TermColor::DarkGrey);

    skin
}

fn convert_fmt_text_to_ratatui(fmt_text: FmtText) -> Text<'static> {
    let rendered = format!("{}", fmt_text);

    let lines: Vec<Line> = rendered
        .lines()
        .map(|line| {
            let bytes = strip_ansi_escapes::strip(line);
            let clean_line = match String::from_utf8(bytes) {
                | Ok(s) => s,
                | Err(_) => line.to_string(),
            };

            Line::from(clean_line)
        })
        .collect();

    Text::from(lines)
}

pub fn to_ratatui_color(term_color: termimad::crossterm::style::Color) -> Color {
    use termimad::crossterm::style::Color as TermColor;

    match term_color {
        | TermColor::Black => Color::Black,
        | TermColor::DarkGrey => Color::DarkGray,
        | TermColor::Red => Color::Red,
        | TermColor::DarkRed => Color::Red,
        | TermColor::Green => Color::Green,
        | TermColor::DarkGreen => Color::Green,
        | TermColor::Yellow => Color::Yellow,
        | TermColor::DarkYellow => Color::Yellow,
        | TermColor::Blue => Color::Blue,
        | TermColor::DarkBlue => Color::Blue,
        | TermColor::Magenta => Color::Magenta,
        | TermColor::DarkMagenta => Color::Magenta,
        | TermColor::Cyan => Color::Cyan,
        | TermColor::DarkCyan => Color::Cyan,
        | TermColor::White => Color::White,
        | TermColor::Grey => Color::Gray,
        | TermColor::Rgb {
            r,
            g,
            b,
        } => Color::Rgb(r, g, b),
        | TermColor::AnsiValue(v) => Color::Indexed(v),
        | TermColor::Reset => Color::Reset,
    }
}
