//! CPU-only embeddings with a per-diary persistent vector cache. A single
//! worker keeps inference off the UI thread.
use diary_application::{Msg,
                        search::{SearchHit,
                                 SearchOutput}};
use diary_storage::Storage;
use fastembed::{EmbeddingModel,
                TextEmbedding,
                TextInitOptions};
use regex::{Regex,
            RegexBuilder};
use serde::{Deserialize,
            Serialize};
use sha2::{Digest,
           Sha256};
use std::{collections::{BTreeMap,
                        BTreeSet},
          sync::mpsc::{self,
                       Receiver,
                       Sender},
          thread};

pub(crate) struct Worker {
    requests: Sender<(u64, String)>,
    pub responses: Receiver<Msg>,
    #[cfg(test)]
    pub ready: Receiver<Result<(), String>>,
}

impl Worker {
    pub fn start(storage: Storage) -> std::io::Result<Self> { Self::spawn(storage, None::<fn() -> Result<TextEmbedding, String>>) }

    pub fn preload(storage: Storage) -> std::io::Result<Self> { Self::spawn(storage, Some(load_model)) }

    pub(super) fn spawn(storage: Storage, loader: Option<impl FnOnce() -> Result<TextEmbedding, String> + Send + 'static>) -> std::io::Result<Self> {
        let (requests, incoming) = mpsc::channel::<(u64, String)>();
        let (outgoing, responses) = mpsc::channel();
        #[cfg(test)]
        let (readied, ready) = mpsc::channel();
        thread::Builder::new().name("diary-search".into()).spawn(move || {
            let mut index = Index::default();
            if let Some(loader) = loader {
                let model = loader();
                #[cfg(test)]
                let _ = readied.send(model.as_ref().map(|_| ()).map_err(Clone::clone));
                // A failed preload is retried by the first search, which owns
                // the error UI.
                index.model = model.ok();
            }
            while let Ok(mut request) = incoming.recv() {
                // Keep only the newest queued query after a slow model
                // download.
                while let Ok(newer) = incoming.try_recv() {
                    request = newer;
                }
                let result = index.search(&storage, &request.1);
                if outgoing.send(Msg::SearchFinished(request.0, result)).is_err() {
                    break;
                }
            }
        })?;
        Ok(Self {
            requests,
            responses,
            #[cfg(test)]
            ready,
        })
    }

    pub fn submit(&self, id: u64, query: String) -> Result<(), String> {
        self.requests
            .send((id, query))
            .map_err(|_| "검색 작업이 종료되었습니다 / Search worker stopped".into())
    }
}

fn load_model() -> Result<TextEmbedding, String> {
    let cache = dirs::cache_dir().ok_or("Cannot find local model cache directory")?.join("ratatui-diary/models");
    let options = TextInitOptions::new(EmbeddingModel::MultilingualE5Small)
        .with_cache_dir(cache)
        .with_show_download_progress(false)
        .with_intra_threads(2);
    TextEmbedding::try_new(options).map_err(|e| e.to_string())
}

struct Passage {
    line: usize,
    col: usize,
    text: String,
    vector: Vec<f32>,
}

struct Document {
    content: String,
    passages: Vec<Passage>,
}

// Bump when the model, tokenizer settings, pooling or passage splitting
// changes.
const CACHE_VERSION: &str = "fastembed-7.1.1/multilingual-e5-small/mean/max512/chars240/v1";
const DIMENSIONS: usize = 384;
const BATCH_SIZE: usize = 16;

#[derive(Serialize, Deserialize)]
struct CachedEmbeddings {
    version: String,
    fingerprint: [u8; 32],
    vectors: Vec<Vec<f32>>,
}

impl CachedEmbeddings {
    fn apply(self, content: &str, passages: &mut [Passage]) -> bool {
        if self.version != CACHE_VERSION
            || self.fingerprint != <[u8; 32]>::from(Sha256::digest(content.as_bytes()))
            || self.vectors.len() != passages.len()
            || !self.vectors.iter().all(|v| valid_vector(v))
        {
            return false;
        }
        for (passage, vector) in passages.iter_mut().zip(self.vectors) {
            passage.vector = vector;
        }
        true
    }
}

fn valid_vector(vector: &[f32]) -> bool {
    let norm = vector.iter().map(|v| v * v).sum::<f32>();
    vector.len() == DIMENSIONS && vector.iter().all(|v| v.is_finite()) && norm.is_finite() && norm > 0.0
}

#[derive(Default)]
struct Index {
    documents: BTreeMap<chrono::NaiveDate, Document>,
    pending_cache: BTreeSet<chrono::NaiveDate>,
    cache_notice: Option<String>,
    model: Option<TextEmbedding>,
    #[cfg(test)]
    embedding_calls: usize,
    #[cfg(test)]
    embedded_passages: usize,
}

impl Index {
    fn refresh(&mut self, storage: &Storage) -> Result<(), String> {
        let dates = storage.scan_entries().map_err(|e| e.to_string())?;
        self.cache_notice = None;
        if let Err(error) = storage.prune_search_cache(&dates) {
            self.cache_error(error);
        }
        self.documents.retain(|date, _| dates.contains(date));
        self.pending_cache.retain(|date| dates.contains(date));
        for date in dates {
            let entry = storage.load_entry(date).map_err(|e| format!("{date}: {e}"))?;
            if self.documents.get(&date).is_some_and(|doc| doc.content == entry.content) {
                continue;
            }
            let mut passages = passages(&entry.content);
            let cached = match storage.load_search_cache(date) {
                | Ok(Some(source)) => serde_json::from_str::<CachedEmbeddings>(&source)
                    .ok()
                    .is_some_and(|cache| cache.apply(&entry.content, &mut passages)),
                | Ok(None) => false,
                | Err(error) => {
                    self.cache_error(error);
                    false
                },
            };
            if !cached {
                self.pending_cache.insert(date);
            }
            self.documents.insert(
                date,
                Document {
                    content: entry.content,
                    passages,
                },
            );
        }
        Ok(())
    }

    fn ensure_model(&mut self) -> Result<(), String> {
        if self.model.is_none() {
            self.model = Some(load_model()?);
        }
        Ok(())
    }

    fn embed_documents(&mut self) -> Result<(), String> {
        let model = self.model.as_mut().unwrap();
        let mut missing: Vec<_> = self
            .documents
            .values_mut()
            .flat_map(|doc| &mut doc.passages)
            .filter(|p| p.vector.is_empty())
            .collect();
        // Similar lengths reduce padding work; references retain the original
        // source positions.
        missing.sort_by_cached_key(|p| p.text.chars().count());
        for batch in missing.chunks_mut(BATCH_SIZE) {
            let inputs: Vec<_> = batch.iter().map(|p| format!("passage: {}", p.text)).collect();
            #[cfg(test)]
            {
                self.embedding_calls += 1;
                self.embedded_passages += inputs.len();
            }
            let vectors = model.embed(inputs, Some(BATCH_SIZE)).map_err(|e| e.to_string())?;
            if vectors.len() != batch.len() || !vectors.iter().all(|v| valid_vector(v)) {
                return Err("Invalid passage embeddings".into());
            }
            for (passage, vector) in batch.iter_mut().zip(vectors) {
                passage.vector = vector;
            }
        }
        Ok(())
    }

    fn cache_error(&mut self, error: impl std::fmt::Display) {
        self.cache_notice = Some(format!(
            "임베딩 캐시 사용 불가 · 검색은 계속됩니다 / Embedding cache unavailable · search continues: {error}"
        ));
    }

    fn persist(&mut self, storage: &Storage) {
        for date in self.pending_cache.clone() {
            let document = &self.documents[&date];
            if !document.passages.iter().all(|p| valid_vector(&p.vector)) {
                continue;
            }
            let cache = CachedEmbeddings {
                version: CACHE_VERSION.to_string(),
                fingerprint: Sha256::digest(document.content.as_bytes()).into(),
                vectors: document.passages.iter().map(|p| p.vector.clone()).collect(),
            };
            match serde_json::to_string(&cache)
                .map_err(std::io::Error::other)
                .and_then(|s| storage.save_search_cache(date, &s))
            {
                | Ok(()) => {
                    self.pending_cache.remove(&date);
                },
                | Err(error) => self.cache_error(error),
            }
        }
    }

    fn embed_query(&mut self, query: &str) -> Result<Vec<f32>, String> {
        let model = self.model.as_mut().unwrap();
        model
            .embed(vec![format!("query: {query}")], Some(1))
            .map_err(|e| e.to_string())?
            .pop()
            .ok_or_else(|| "No query embedding".into())
    }

    fn search(&mut self, storage: &Storage, query: &str) -> Result<SearchOutput, String> {
        self.refresh(storage)?;
        if self.documents.values().all(|doc| doc.passages.is_empty()) {
            return Ok(SearchOutput::default());
        }
        let pattern = RegexBuilder::new(&regex::escape(query))
            .case_insensitive(true)
            .build()
            .map_err(|e| e.to_string())?;
        let embedding = self.ensure_model().and_then(|()| self.embed_documents()).and_then(|()| {
            self.persist(storage);
            self.embed_query(query)
        });
        let (vector, notice) = match embedding {
            | Ok(vector) => (Some(vector), self.cache_notice.clone()),
            | Err(error) => (
                None,
                Some(format!(
                    "의미 검색 사용 불가 · 문자열 결과만 표시 / Semantic search unavailable · literal results only: {error}"
                )),
            ),
        };
        Ok(SearchOutput {
            results: rank(&self.documents, &pattern, vector.as_deref()),
            notice,
        })
    }
}

#[cfg(test)]
#[path = "search_benchmark.rs"]
mod benchmark;

fn passages(content: &str) -> Vec<Passage> {
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
            // Bound Korean passages too, rather than silently truncating long
            // paragraphs at the tokenizer limit.
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

fn cosine(a: &[f32], b: &[f32]) -> f32 {
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

fn rank(documents: &BTreeMap<chrono::NaiveDate, Document>, pattern: &Regex, query: Option<&[f32]>) -> Vec<SearchHit> {
    let mut ranked = Vec::new();
    for (&date, document) in documents {
        let matches: Vec<_> = pattern.find_iter(&document.content).take(100).collect();
        let literal_positions: Vec<_> = matches.iter().map(|m| position(&document.content, m.start())).collect();
        // Literal matches use the original document, including phrases crossing
        // embedding chunk boundaries.
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fill_vectors(index: &mut Index) {
        for document in index.documents.values_mut() {
            for passage in &mut document.passages {
                passage.vector = vec![1.0; DIMENSIONS];
            }
        }
    }

    #[test]
    fn persistent_cache_survives_restart_but_not_same_length_edits_or_deletions() {
        let directory = TempDir::new().unwrap();
        let storage = Storage::with_dir(directory.path()).unwrap();
        let date = chrono::NaiveDate::default();
        storage.save(date, "비밀 기록\r\n\r\n다음 문단").unwrap();
        let mut index = Index::default();
        index.refresh(&storage).unwrap();
        fill_vectors(&mut index);
        index.persist(&storage);
        assert!(index.cache_notice.is_none());
        let source = storage.load_search_cache(date).unwrap().unwrap();
        assert!(!source.contains("비밀"));
        let mut reopened = Index::default();
        reopened.refresh(&storage).unwrap();
        assert_eq!(reopened.documents[&date].passages[0].vector, index.documents[&date].passages[0].vector);
        assert!(reopened.pending_cache.is_empty());
        storage.save(date, "다른 기록\r\n\r\n다음 문단").unwrap();
        reopened.refresh(&storage).unwrap();
        assert!(reopened.documents[&date].passages.iter().all(|p| p.vector.is_empty()));
        fill_vectors(&mut reopened);
        reopened.persist(&storage);
        storage.delete(date).unwrap();
        reopened.refresh(&storage).unwrap();
        assert!(reopened.documents.is_empty());
        assert!(storage.load_search_cache(date).unwrap().is_none());
        let other_directory = TempDir::new().unwrap();
        let other_storage = Storage::with_dir(other_directory.path()).unwrap();
        assert!(other_storage.load_search_cache(date).unwrap().is_none());
    }

    #[test]
    fn invalid_cache_is_rebuilt_and_cache_write_failure_preserves_search_data() {
        let directory = TempDir::new().unwrap();
        let storage = Storage::with_dir(directory.path()).unwrap();
        let date = chrono::NaiveDate::default();
        storage.save(date, "원문 유지").unwrap();
        let mut index = Index::default();
        index.refresh(&storage).unwrap();
        fill_vectors(&mut index);
        index.persist(&storage);
        let valid = storage.load_search_cache(date).unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&valid).unwrap();
        let mut version = parsed.clone();
        version["version"] = "old-model".into();
        let mut fingerprint = parsed.clone();
        fingerprint["fingerprint"][0] = (fingerprint["fingerprint"][0].as_u64().unwrap() ^ 255).into();
        let mut shape = parsed.clone();
        shape["vectors"] = serde_json::json!([[1.0]]);
        let mut zero = parsed.clone();
        zero["vectors"] = serde_json::json!([vec![0.0; DIMENSIONS]]);
        for source in [
            "broken json".to_string(),
            version.to_string(),
            fingerprint.to_string(),
            shape.to_string(),
            zero.to_string(),
        ] {
            storage.save_search_cache(date, &source).unwrap();
            let mut reopened = Index::default();
            reopened.refresh(&storage).unwrap();
            assert!(reopened.documents[&date].passages[0].vector.is_empty());
            assert!(reopened.pending_cache.contains(&date));
            fill_vectors(&mut reopened);
            reopened.persist(&storage);
            assert_eq!(storage.load_search_cache(date).unwrap().unwrap(), valid);
        }
        let path = directory.path().join(format!("entries/.search-cache/{date}.json"));
        let original = std::fs::metadata(&path).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        std::fs::set_permissions(&path, readonly).unwrap();
        index.pending_cache.insert(date);
        index.persist(&storage);
        assert!(index.cache_notice.is_some());
        assert!(index.pending_cache.contains(&date));
        assert_eq!(storage.load(date).unwrap(), "원문 유지");
        assert_eq!(index.documents[&date].passages[0].vector.len(), DIMENSIONS);
        std::fs::set_permissions(path, original).unwrap();
    }

    #[test]
    fn chunks_preserve_korean_positions_and_do_not_drop_long_lines() {
        let content = format!("제목\r\n\r\n{}\r\n끝", "가".repeat(500));
        let chunks = passages(&content);
        assert_eq!((chunks[1].line, chunks[1].col), (2, 0));
        assert_eq!((chunks[2].line, chunks[2].col), (2, 240));
        assert_eq!((chunks[3].line, chunks[3].col), (2, 480));
        assert_eq!(chunks.iter().map(|p| p.text.matches('가').count()).sum::<usize>(), 500);
        assert!(chunks.last().unwrap().text.contains('끝'));
    }

    #[test]
    fn hybrid_ranking_keeps_exact_matches_and_finds_semantic_only_passages() {
        let date = chrono::NaiveDate::default();
        let content = format!("{}MiXeD\n\n전혀 다른 표현", "가".repeat(239));
        let mut chunks = passages(&content);
        for p in &mut chunks {
            p.vector = if p.text.contains("다른") { vec![1.0, 0.0] } else { vec![0.0, 1.0] };
        }
        let documents = BTreeMap::from([(
            date,
            Document {
                content,
                passages: chunks,
            },
        )]);
        let pattern = RegexBuilder::new("mixed").case_insensitive(true).build().unwrap();
        let hits = rank(&documents, &pattern, Some(&[1.0, 0.0]));
        assert!(hits[0].literal);
        assert_eq!((hits[0].line, hits[0].col), (0, 239));
        assert_eq!(hits[0].anchor, "MiXeD");
        assert!(hits.iter().any(|h| !h.literal && h.excerpt.contains("다른")));
        let literal_only = rank(&documents, &pattern, None);
        assert_eq!(literal_only.len(), 1);
        assert!(literal_only[0].literal);
        assert_eq!(cosine(&[0.0], &[0.0]), 0.0);
    }

    #[test]
    fn refresh_reuses_unchanged_vectors_and_tracks_external_edits_and_deletions() {
        let directory = TempDir::new().unwrap();
        let storage = Storage::with_dir(directory.path()).unwrap();
        let date = chrono::NaiveDate::default();
        storage.save(date, "원래 내용").unwrap();
        let mut index = Index::default();
        index.refresh(&storage).unwrap();
        index.documents.get_mut(&date).unwrap().passages[0].vector = vec![1.0];
        index.refresh(&storage).unwrap();
        assert_eq!(index.documents[&date].passages[0].vector, [1.0]);
        storage.save(date, "변경된 내용").unwrap();
        index.refresh(&storage).unwrap();
        assert!(index.documents[&date].passages[0].vector.is_empty());
        storage.delete(date).unwrap();
        index.refresh(&storage).unwrap();
        assert!(index.documents.is_empty());
        std::fs::write(directory.path().join(format!("entries/{date}.md")), "legacy").unwrap();
        index.refresh(&storage).unwrap();
        assert_eq!(index.documents[&date].content, "legacy");
    }

    #[test]
    #[ignore = "downloads the multilingual model once; run explicitly to verify real CPU inference"]
    fn real_korean_semantic_search() {
        let directory = TempDir::new().unwrap();
        let storage = Storage::with_dir(directory.path()).unwrap();
        let date = chrono::NaiveDate::default();
        storage
            .save(
                date,
                "앞으로 어떤 일을 하며 살아야 할지 모르겠다. 내 미래가 걱정돼 잠이 오지 않는다.\n\n저녁에 파스타를 만들었다. 토마토 소스가 맛있었다.",
            )
            .unwrap();
        let mut index = Index::default();
        let output = index.search(&storage, "진로에 대한 불안").unwrap();
        assert!(output.notice.is_none(), "{:?}", output.notice);
        assert!(!output.results[0].literal);
        assert!(output.results[0].excerpt.contains("미래"));
        assert!(storage.load_search_cache(date).unwrap().is_some());
        let mut reopened = Index::default();
        let cached_output = reopened.search(&storage, "진로에 대한 불안").unwrap();
        assert_eq!(cached_output, output);
        assert_eq!(reopened.embedded_passages, 0);
        assert_eq!(reopened.embedding_calls, 0);
        let output = index.search(&storage, "파스타").unwrap();
        assert!(output.results[0].literal);
        assert_eq!(output.results[0].line, 2);
    }
}
