//! Pure passage splitting and ranking over supplied documents and vectors.
use diary_domain::search::SearchHit;
use regex::Regex;
use std::collections::BTreeMap;

pub(super) struct Passage {
    pub(super) line: usize,
    pub(super) col: usize,
    pub(super) text: String,
    pub(super) vector: Vec<f32>,
}

pub(super) struct Document {
    pub(super) content: String,
    pub(super) passages: Vec<Passage>,
}

pub(super) fn passages(content: &str) -> Vec<Passage> {
    let mut result = Vec::new();
    let mut text = String::new();
    let (mut start_line, mut start_col, mut count) = (0, 0, 0);
    for (line, source) in content.split('\n').enumerate() {
        let source = source.strip_suffix('\r').unwrap_or(source);
        if source.trim().is_empty() {
            if !text.is_empty() {
                result.push(Passage {
                    line: start_line,
                    col: start_col,
                    text: std::mem::take(&mut text),
                    vector: Vec::new(),
                });
                count = 0;
            }
            continue;
        }
        for (col, ch) in source.chars().enumerate() {
            if count >= 240 {
                result.push(Passage {
                    line: start_line,
                    col: start_col,
                    text: std::mem::take(&mut text),
                    vector: Vec::new(),
                });
                count = 0;
            }
            if text.is_empty() {
                (start_line, start_col) = (line, col);
            }
            text.push(ch);
            count += 1;
        }
        text.push('\n');
        count += 1;
    }
    if !text.is_empty() {
        result.push(Passage {
            line: start_line,
            col: start_col,
            text,
            vector: Vec::new(),
        });
    }
    result
}

fn position(content: &str, byte: usize) -> (usize, usize) {
    let prefix = &content[.. byte];
    (
        prefix.bytes().filter(|&b| b == b'\n').count(),
        prefix.rsplit('\n').next().unwrap_or("").chars().count(),
    )
}

pub(super) fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(a, b)| a * b).sum();
    let norm = (a.iter().map(|v| v * v).sum::<f32>() * b.iter().map(|v| v * v).sum::<f32>()).sqrt();
    if norm > 0.0 { dot / norm } else { 0.0 }
}

fn contains(passage: &Passage, line: usize, col: usize) -> bool {
    let newlines = passage.text.bytes().filter(|&b| b == b'\n').count();
    let end_col = if newlines == 0 {
        passage.col + passage.text.chars().count()
    } else {
        passage.text.rsplit('\n').next().unwrap_or("").chars().count()
    };
    (passage.line, passage.col) <= (line, col) && (line, col) < (passage.line + newlines, end_col)
}

pub(super) fn rank(documents: &BTreeMap<chrono::NaiveDate, Document>, pattern: &Regex, query: Option<&[f32]>) -> Vec<SearchHit> {
    let mut ranked = Vec::new();
    for (&date, document) in documents {
        let matches: Vec<_> = pattern.find_iter(&document.content).take(100).collect();
        let literal_positions: Vec<_> = matches.iter().map(|m| position(&document.content, m.start())).collect();
        for (m, &(line, col)) in matches.iter().zip(&literal_positions) {
            let before: String = document.content[.. m.start()]
                .chars()
                .rev()
                .take(100)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let after: String = document.content[m.start() ..].chars().take(400).collect();
            let excerpt = format!("{before}{after}");
            let similarity = query.map_or(0.0, |q| {
                document.passages.iter().find(|p| contains(p, line, col)).map_or(0.0, |p| cosine(q, &p.vector))
            });
            ranked.push((
                2.0 + similarity,
                SearchHit {
                    date,
                    line,
                    col,
                    excerpt,
                    literal: true,
                    anchor: m.as_str().to_string(),
                },
            ));
        }
        if let Some(query) = query {
            for passage in &document.passages {
                if literal_positions.iter().any(|&(line, col)| contains(passage, line, col)) {
                    continue;
                }
                let score = cosine(query, &passage.vector);
                if score.is_finite() && score > 0.0 {
                    let anchor = passage.text.trim_end_matches('\n').to_string();
                    ranked.push((
                        score,
                        SearchHit {
                            date,
                            line: passage.line,
                            col: passage.col,
                            excerpt: passage.text.clone(),
                            literal: false,
                            anchor,
                        },
                    ));
                }
            }
        }
    }
    ranked.sort_by(|(a, ah), (b, bh)| {
        b.total_cmp(a)
            .then_with(|| bh.date.cmp(&ah.date))
            .then_with(|| (ah.line, ah.col).cmp(&(bh.line, bh.col)))
    });
    let mut semantic = 0;
    ranked
        .into_iter()
        .map(|(_, hit)| hit)
        .filter(|hit| {
            if hit.literal {
                true
            } else {
                semantic += 1;
                semantic <= 20
            }
        })
        .take(100)
        .collect()
}
