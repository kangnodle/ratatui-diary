use ratatui::{style::{Modifier,
                      Style},
              text::{Line,
                     Text}};

/// Render Org headlines using org-bullets' default, repeating bullet sequence.
pub fn render_to_text(org: &str) -> Text<'static> {
    const BULLETS: [&str; 4] = ["◉", "○", "✸", "✿"];
    let mut block_end = None;
    let mut lines = Vec::new();
    for line in org.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let keyword = line.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
        if let Some(end) = &block_end {
            if &keyword == end {
                block_end = None;
            }
            lines.push(Line::raw(line.to_owned()));
            continue;
        }
        if ["#+begin_src", "#+begin_example", "#+begin_export", "#+begin_comment"].contains(&keyword.as_str()) {
            block_end = Some(keyword.replacen("begin", "end", 1));
        }

        let level = line.bytes().take_while(|&b| b == b'*').count();
        if level > 0 && line[level ..].starts_with(' ') {
            lines.push(Line::styled(
                format!("{}{}{}", " ".repeat(level - 1), BULLETS[(level - 1) % BULLETS.len()], &line[level ..]),
                Style::default().add_modifier(Modifier::BOLD),
            ));
        } else {
            // shortcut: body stays raw; add an Org parser for rich rendering.
            lines.push(Line::raw(line.to_owned()));
        }
    }
    Text::from(lines)
}
