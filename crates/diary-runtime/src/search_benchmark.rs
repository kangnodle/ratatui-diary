use super::*;
use diary_application::{Model,
                        Screen,
                        settings::Keybindings};
use serde_json::{Value,
                 json};
use std::time::Instant;
use tempfile::TempDir;

const QUERIES: [&str; 3] = ["진로에 대한 불안", "파스타", "친구와 즐거운 시간"];

fn worker_result(worker: &Worker, id: u64) -> SearchOutput {
    let message = match worker.responses.recv_timeout(std::time::Duration::from_secs(30)) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not receive the worker response: {error:?}"),
    };
    let Msg::SearchFinished(received, result) = message else {
        panic!("unexpected response");
    };
    assert_eq!(received, id);
    let output = match result {
        | Ok(value) => value,
        | Err(error) => panic!("Search failed: {error:?}"),
    };
    assert!(output.notice.is_none(), "{:?}", output.notice);
    output
}

#[test]
#[ignore = "release preload benchmark; requires cached model"]
fn search_preload_benchmark() {
    if cfg!(debug_assertions) {
        panic!("run with --release");
    }
    let (_directory, storage) = fixture();
    let mut index = Index::default();
    let expected: Vec<_> = QUERIES
        .iter()
        .map(|q| match index.search(&storage, q) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not search the diaries: {error:?}"),
        })
        .collect();
    assert!(expected.iter().all(|r| r.notice.is_none()));
    drop(index);
    let mut trials = Vec::new();
    for _ in 0 .. 3 {
        let start = Instant::now();
        let lazy = match Worker::start(storage.clone()) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not start the search worker: {error:?}"),
        };
        match lazy.submit(1, QUERIES[0].into()) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not submit the search request: {error:?}"),
        };
        assert_eq!(worker_result(&lazy, 1), expected[0]);
        let lazy_first = elapsed(start);
        drop(lazy);
        let start = Instant::now();
        let prepared = match Worker::preload(storage.clone()) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not start preloading the search model: {error:?}"),
        };
        let schedule = elapsed(start);
        match prepared.ready.recv_timeout(std::time::Duration::from_secs(30)) {
            | Ok(Ok(_)) => (),
            | Ok(Err(error)) => panic!("Search model initialization failed: {error:?}"),
            | Err(error) => panic!("Could not receive the worker response: {error:?}"),
        };
        let ready = elapsed(start);
        let start = Instant::now();
        match prepared.submit(1, QUERIES[0].into()) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not submit the search request: {error:?}"),
        };
        assert_eq!(worker_result(&prepared, 1), expected[0]);
        let prepared_first = elapsed(start);
        for (i, query) in QUERIES.iter().enumerate().skip(1) {
            match prepared.submit(i as u64 + 1, (*query).into()) {
                | Ok(_) => (),
                | Err(error) => panic!("Could not submit the search request: {error:?}"),
            };
            assert_eq!(worker_result(&prepared, i as u64 + 1), expected[i]);
        }
        drop(prepared);
        let start = Instant::now();
        let immediate = match Worker::preload(storage.clone()) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not start preloading the search model: {error:?}"),
        };
        match immediate.submit(1, QUERIES[0].into()) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not submit the search request: {error:?}"),
        };
        assert_eq!(worker_result(&immediate, 1), expected[0]);
        let immediate_first = elapsed(start);
        trials.push(
            json!({"lazy_first_ms":lazy_first,"schedule_ms":schedule,"ready_ms":ready,"prepared_first_ms":prepared_first,"immediate_first_ms":immediate_first}),
        );
    }
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/search-preload.json");
    match std::fs::write(
        file,
        match serde_json::to_vec_pretty(&trials) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not serialize the benchmark data: {error:?}"),
        },
    ) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    println!(
        "{}",
        match serde_json::to_string_pretty(&trials) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not serialize the benchmark data: {error:?}"),
        }
    );
}

fn fixture() -> (TempDir, Storage) {
    let directory = match TempDir::new() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
    };
    let storage = match Storage::with_dir(directory.path()) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
    };
    let start = match chrono::NaiveDate::from_ymd_opt(2025, 1, 1) {
        | Some(value) => value,
        | None => panic!("Invalid date in test fixture"),
    };
    let subjects = [
        "취업 준비",
        "새로운 프로젝트",
        "진학",
        "회사 생활",
        "가족의 기대",
        "이직",
        "자격증 공부",
        "면접",
        "창업",
        "졸업",
    ];
    let activities = [
        "공원에서 산책",
        "동네 카페에서 대화",
        "영화 관람",
        "같이 자전거 타기",
        "도서관에서 독서",
        "음악 감상",
        "친구 집 방문",
        "보드게임",
        "등산",
        "사진 촬영",
    ];
    for day in 0 .. 80 {
        let date = start + chrono::Duration::days(day);
        let source = format!(
            "{}에 대해 생각한 {day}번째 기록. 앞으로 어떤 일을 하며 살아야 할지 모르겠다. 미래가 걱정돼 잠이 오지 않는다.\n\n저녁에 파스타를 만들었다. 토마토 소스와 버섯을 넣었다. 요리 연습 {day}일째라 맛이 조금씩 좋아진다.\n\n친구와 {}을 했다. 오랜만에 웃고 이야기하며 편안했다. 다음에는 {} 이야기도 나누고 싶다.",
            subjects[day as usize % 10],
            activities[day as usize / 8],
            subjects[(day as usize + 3) % 10]
        );
        let source = if day % 2 == 0 { source.replace('\n', "\r\n") } else { source };
        if day % 7 == 0 {
            match std::fs::write(directory.path().join(format!("entries/{date}.md")), source) {
                | Ok(_) => (),
                | Err(error) => panic!("Could not write the file: {error:?}"),
            };
        } else {
            match storage.save(date, &source) {
                | Ok(_) => (),
                | Err(error) => panic!("Could not save the diary: {error:?}"),
            };
        }
    }
    (directory, storage)
}

fn elapsed(start: Instant) -> f64 { start.elapsed().as_secs_f64() * 1000.0 }

fn baseline_refresh(index: &mut Index, storage: &Storage) {
    let dates = match storage.scan_entries() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not list the diary entries: {error:?}"),
    };
    index.documents.retain(|date, _| dates.contains(date));
    for date in dates {
        let entry = match storage.load_entry(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load the diary entry: {error:?}"),
        };
        if index.documents.get(&date).is_some_and(|doc| doc.content == entry.content) {
            continue;
        }
        index.documents.insert(
            date,
            Document {
                passages: passages(&entry.content),
                content: entry.content,
            },
        );
    }
}

fn baseline_embed_documents(index: &mut Index) {
    let model = match index.model.as_mut() {
        | Some(value) => value,
        | None => panic!("Expected model to be present"),
    };
    for document in index.documents.values_mut() {
        for batch in document.passages.chunks_mut(32) {
            if batch.iter().all(|p| !p.vector.is_empty()) {
                continue;
            }
            let inputs: Vec<_> = batch.iter().map(|p| format!("passage: {}", p.text)).collect();
            index.embedding_calls += 1;
            index.embedded_passages += inputs.len();
            let vectors = match model.embed(inputs, Some(32)) {
                | Ok(value) => value,
                | Err(error) => panic!("Could not generate embeddings: {error:?}"),
            };
            assert_eq!(vectors.len(), batch.len());
            for (passage, vector) in batch.iter_mut().zip(vectors) {
                passage.vector = vector;
            }
        }
    }
}

fn profile(index: &mut Index, storage: &Storage, query: &str, baseline: bool) -> (Value, Vec<SearchHit>) {
    let total = Instant::now();
    let start = Instant::now();
    if baseline {
        baseline_refresh(index, storage);
    } else {
        match index.refresh(storage) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not refresh the diary search index: {error:?}"),
        };
    }
    let refresh = elapsed(start);
    let start = Instant::now();
    match index.ensure_model() {
        | Ok(_) => (),
        | Err(error) => panic!("Could not load the embedding model: {error:?}"),
    };
    let model = elapsed(start);
    let calls = index.embedding_calls;
    let embedded = index.embedded_passages;
    let start = Instant::now();
    if baseline {
        baseline_embed_documents(index);
    } else {
        match index.embed_documents() {
            | Ok(_) => (),
            | Err(error) => panic!("Could not embed the diary passages: {error:?}"),
        };
    }
    let documents = elapsed(start);
    let start = Instant::now();
    if !baseline {
        index.persist(storage);
    }
    let persist = elapsed(start);
    let start = Instant::now();
    let vector = match index.embed_query(query) {
        | Ok(value) => value,
        | Err(error) => panic!("Could not embed the search query: {error:?}"),
    };
    let query_ms = elapsed(start);
    let start = Instant::now();
    let pattern = match RegexBuilder::new(&regex::escape(query)).case_insensitive(true).build() {
        | Ok(value) => value,
        | Err(error) => panic!("Could not compile the search pattern: {error:?}"),
    };
    let hits = rank(&index.documents, &pattern, Some(&vector));
    let ranking = elapsed(start);
    let total = elapsed(total);
    for keybindings in [Keybindings::Emacs, Keybindings::Helix] {
        for hit in &hits {
            let mut model = Model::new(hit.date, Default::default());
            model.settings.keybindings = keybindings;
            model.screen = Screen::Search;
            model.search.results = vec![hit.clone()];
            let command = match diary_application::update(&mut model, Msg::SearchSubmit) {
                | Some(value) => value,
                | None => panic!("Expected the application update to produce a command"),
            };
            let mut runtime = crate::Runtime::new(storage.clone(), None);
            runtime.execute(command, &mut model);
            assert!(!model.show_error_popup);
            assert_eq!(model.screen, Screen::Editor);
            assert_eq!((model.editor_state.cursor_line, model.editor_state.cursor_col), (hit.line, hit.col));
            assert_eq!(model.editor_state.date, hit.date);
        }
    }
    (
        json!({"refresh_ms":refresh,"model_ms":model,"documents_ms":documents,"persist_ms":persist,"query_ms":query_ms,"ranking_ms":ranking,"total_ms":total,
        "embedding_calls":index.embedding_calls-calls,"embedded_passages":index.embedded_passages-embedded}),
        hits,
    )
}

fn snapshot(hits: &[SearchHit]) -> Value {
    json!(
        hits.iter()
            .map(|h| json!({"date":h.date.to_string(),"line":h.line,"col":h.col,"literal":h.literal,"anchor":h.anchor,"excerpt":h.excerpt}))
            .collect::<Vec<_>>()
    )
}

#[test]
#[ignore = "release batch-size experiment; uses cached model and synthetic diaries"]
fn search_batch_sizes() {
    if cfg!(debug_assertions) {
        panic!("run with --release");
    }
    let (_directory, storage) = fixture();
    let mut index = Index::default();
    match index.refresh(&storage) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not refresh the diary search index: {error:?}"),
    };
    match index.ensure_model() {
        | Ok(_) => (),
        | Err(error) => panic!("Could not load the embedding model: {error:?}"),
    };
    let mut inputs: Vec<_> = index
        .documents
        .values()
        .flat_map(|doc| &doc.passages)
        .map(|p| format!("passage: {}", p.text))
        .collect();
    let original = inputs.clone();
    inputs.sort_by_key(|s| s.chars().count());
    let model = match index.model.as_mut() {
        | Some(value) => value,
        | None => panic!("Expected model to be present"),
    };
    let mut trials = Vec::new();
    for _ in 0 .. 3 {
        let start = Instant::now();
        for diary in original.chunks(3) {
            let vectors = match model.embed(diary, Some(32)) {
                | Ok(value) => value,
                | Err(error) => panic!("Could not generate embeddings: {error:?}"),
            };
            assert!(vectors.iter().all(|v| valid_vector(v)));
        }
        trials.push(json!({"kind":"per-diary","batch":3,"ms":elapsed(start)}));
        let start = Instant::now();
        let vectors = match model.embed(original.clone(), Some(32)) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not generate embeddings: {error:?}"),
        };
        assert!(vectors.iter().all(|v| valid_vector(v)));
        trials.push(json!({"kind":"mixed-length","batch":32,"ms":elapsed(start)}));
        for batch in [8, 16, 32] {
            let start = Instant::now();
            let vectors = match model.embed(inputs.clone(), Some(batch)) {
                | Ok(value) => value,
                | Err(error) => panic!("Could not generate embeddings: {error:?}"),
            };
            assert!(vectors.iter().all(|v| valid_vector(v)));
            trials.push(json!({"kind":"similar-length","batch":batch,"ms":elapsed(start)}));
        }
    }
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/search-batch-sizes.json");
    match std::fs::write(
        file,
        match serde_json::to_vec_pretty(&trials) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not serialize the benchmark data: {error:?}"),
        },
    ) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    println!(
        "{}",
        match serde_json::to_string_pretty(&trials) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not serialize the benchmark data: {error:?}"),
        }
    );
}

#[test]
#[ignore = "release performance benchmark; uses cached model and synthetic diaries"]
fn search_release_benchmark() {
    if cfg!(debug_assertions) {
        panic!("run with --release");
    }
    let phase = std::env::var("DIARY_SEARCH_BENCH_PHASE").unwrap_or_else(|_| "after".into());
    assert!(matches!(phase.as_str(), "before" | "after"));
    let baseline = phase == "before";
    let mut trials = Vec::new();
    let mut snapshots = Vec::new();
    for _ in 0 .. 3 {
        let (_directory, storage) = fixture();
        let mut index = Index::default();
        let mut measurements = Vec::new();
        let (cold, hits) = profile(&mut index, &storage, QUERIES[0], baseline);
        if !baseline {
            assert_eq!(cold["embedding_calls"], 15);
        }
        measurements.push(json!({"scenario":"cold", "metrics":cold}));
        snapshots.push(snapshot(&hits));
        for query in QUERIES {
            let (warm, hits) = profile(&mut index, &storage, query, baseline);
            measurements.push(json!({"scenario":"warm","query":query,"metrics":warm}));
            snapshots.push(snapshot(&hits));
        }
        drop(index);
        let mut index = Index::default();
        let (reopened, hits) = profile(&mut index, &storage, QUERIES[0], baseline);
        if !baseline {
            assert_eq!(reopened["embedded_passages"], 0);
        }
        measurements.push(json!({"scenario":"reopened", "metrics":reopened}));
        snapshots.push(snapshot(&hits));
        let date = match chrono::NaiveDate::from_ymd_opt(2025, 1, 2) {
            | Some(value) => value,
            | None => panic!("Invalid date in test fixture"),
        };
        let content = match storage.load(date) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not load the diary: {error:?}"),
        };
        match storage.save(date, &format!("변경된 기록\n\n{content}")) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not save the diary: {error:?}"),
        };
        let (edited, hits) = profile(&mut index, &storage, QUERIES[0], baseline);
        assert_eq!(edited["embedded_passages"], 4);
        measurements.push(json!({"scenario":"edited", "metrics":edited}));
        snapshots.push(snapshot(&hits));
        trials.push(measurements);
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
    if phase == "after" {
        let baseline: Value = match serde_json::from_slice(&std::fs::read(path.join("search-benchmark-before.json")).expect("run before phase first")) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not parse the benchmark baseline: {error:?}"),
        };
        assert_eq!(json!(snapshots), baseline["snapshots"], "result order, excerpts, or source positions changed");
    }
    let report = json!({"phase":phase,"diaries":80,"passages":240,"trials":trials,"snapshots":snapshots});
    let file = path.join(format!("search-benchmark-{phase}.json"));
    match std::fs::write(
        &file,
        match serde_json::to_vec_pretty(&report) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not serialize the benchmark data: {error:?}"),
        },
    ) {
        | Ok(_) => (),
        | Err(error) => panic!("Could not write the file: {error:?}"),
    };
    println!(
        "report: {}\n{}",
        file.display(),
        match serde_json::to_string_pretty(&report["trials"]) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not serialize the benchmark data: {error:?}"),
        }
    );
}
