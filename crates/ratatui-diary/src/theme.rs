use diary_domain::settings::Theme;
use ratatui::style::{Color,
                     Style};
#[derive(Clone, Copy)]
pub struct Palette {
    pub fg: Color,
    pub bg: Color,
    pub accent: Color,
    pub entry: Color,
    pub selected_fg: Color,
    pub selected_bg: Color,
    pub error: Color,
    pub selection: Color,
    pub search: Color,
    pub current_search: Color,
}
pub fn palette(theme: Theme) -> Palette {
    let light = theme == Theme::Light;
    Palette {
        fg: match theme {
            | Theme::System => Color::Reset,
            | Theme::Dark => Color::Rgb(225, 230, 240),
            | Theme::Light => Color::Rgb(30, 35, 45),
        },
        bg: match theme {
            | Theme::System => Color::Reset,
            | Theme::Dark => Color::Rgb(24, 28, 36),
            | Theme::Light => Color::Rgb(250, 250, 248),
        },
        accent: if light { Color::Rgb(0, 85, 145) } else { Color::Cyan },
        entry: if light { Color::Rgb(20, 110, 50) } else { Color::Green },
        selected_fg: Color::White,
        selected_bg: Color::Rgb(35, 80, 160),
        error: if light { Color::Rgb(175, 20, 30) } else { Color::LightRed },
        selection: if light { Color::Rgb(190, 210, 240) } else { Color::DarkGray },
        search: Color::Yellow,
        current_search: Color::LightYellow,
    }
}
impl Palette {
    pub fn base(self) -> Style { Style::default().fg(self.fg).bg(self.bg) }
}
