#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub date: chrono::NaiveDate,
    pub line: usize,
    pub col: usize,
    pub excerpt: String,
    pub literal: bool,
    pub anchor: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchOutput {
    pub results: Vec<SearchHit>,
    pub notice: Option<String>,
}

impl SearchHit {
    /// Finds the closest occurrence of this hit’s anchor in the current entry.
    pub fn locate_in(&self, content: &str) -> Option<(usize, usize)> {
        if self.anchor.is_empty() {
            return None;
        }
        let content = content.replace("\r\n", "\n");
        let anchor = self.anchor.replace("\r\n", "\n");
        content
            .match_indices(&anchor)
            .map(|(byte, _)| {
                let prefix = &content[.. byte];
                (
                    prefix.bytes().filter(|&b| b == b'\n').count(),
                    prefix.rsplit('\n').next().unwrap_or("").chars().count(),
                )
            })
            .min_by_key(|&(line, col)| (line.abs_diff(self.line), col.abs_diff(self.col)))
    }
}
