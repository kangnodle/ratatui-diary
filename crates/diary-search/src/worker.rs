use super::{Index,
            load_model};
use diary_domain::search::SearchOutput;
use diary_storage::Storage;
use fastembed::TextEmbedding;
use std::{sync::mpsc::{self,
                       Receiver,
                       Sender},
          thread};

pub struct Response {
    pub revision: u64,
    pub result: Result<SearchOutput, String>,
}

pub struct Worker {
    requests: Sender<(u64, String)>,
    responses: Receiver<Response>,
    #[cfg(test)]
    pub(crate) ready: Receiver<Result<(), String>>,
}

impl Worker {
    pub fn start(storage: Storage) -> std::io::Result<Self> { Self::spawn(storage, None::<fn() -> Result<TextEmbedding, String>>) }

    pub fn preload(storage: Storage) -> std::io::Result<Self> { Self::spawn(storage, Some(load_model)) }

    fn spawn(storage: Storage, loader: Option<impl FnOnce() -> Result<TextEmbedding, String> + Send + 'static>) -> std::io::Result<Self> {
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
                index.model = model.ok();
            }
            while let Ok(mut request) = incoming.recv() {
                while let Ok(newer) = incoming.try_recv() {
                    request = newer;
                }
                let result = index.search(&storage, &request.1);
                if outgoing
                    .send(Response {
                        revision: request.0,
                        result,
                    })
                    .is_err()
                {
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

    pub fn try_response(&self) -> Result<Response, mpsc::TryRecvError> { self.responses.try_recv() }

    #[cfg(test)]
    pub(crate) fn response_timeout(&self, timeout: std::time::Duration) -> Result<Response, mpsc::RecvTimeoutError> { self.responses.recv_timeout(timeout) }

    pub fn submit(&self, id: u64, query: String) -> Result<(), String> {
        self.requests
            .send((id, query))
            .map_err(|_| "검색 작업이 종료되었습니다 / Search worker stopped".into())
    }
}

#[cfg(test)]
mod preload_tests {
    use super::*;
    use std::{sync::{Arc,
                     atomic::{AtomicUsize,
                              Ordering},
                     mpsc},
              time::Duration};
    use tempfile::TempDir;

    #[test]
    fn paused_preload_runs_once_off_thread() {
        let directory = match TempDir::new() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
        };
        let storage = match Storage::with_dir(directory.path()) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
        };
        let (entered, started) = mpsc::channel();
        let (release, paused) = mpsc::channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let loaded = calls.clone();
        let worker = match Worker::spawn(
            storage,
            Some(move || {
                loaded.fetch_add(1, Ordering::SeqCst);
                match entered.send(std::thread::current().id()) {
                    | Ok(_) => (),
                    | Err(error) => panic!("Could not send the worker message: {error:?}"),
                };
                let _ = paused.recv();
                Err("simulated preload failure".into())
            }),
        ) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not start the test search worker: {error:?}"),
        };
        assert_ne!(
            match started.recv_timeout(Duration::from_secs(3)) {
                | Ok(value) => value,
                | Err(error) => panic!("Could not receive the worker response: {error:?}"),
            },
            std::thread::current().id()
        );
        assert!(matches!(worker.try_response(), Err(mpsc::TryRecvError::Empty)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        match release.send(()) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not send the worker message: {error:?}"),
        };
        let ready = match worker.ready.recv_timeout(Duration::from_secs(3)) {
            | Ok(ready) => ready,
            | Err(error) => panic!("Could not receive the worker response: {error:?}"),
        };
        assert_eq!(ready, Err("simulated preload failure".into()));
    }

    #[test]
    fn queries_during_preload_share_the_worker_and_keep_only_the_latest_query() {
        let directory = match TempDir::new() {
            | Ok(value) => value,
            | Err(error) => panic!("Could not create a temporary directory: {error:?}"),
        };
        let storage = match Storage::with_dir(directory.path()) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not initialize diary storage: {error:?}"),
        };
        let (entered, started) = mpsc::channel();
        let (release, paused) = mpsc::channel();
        let worker = match Worker::spawn(
            storage,
            Some(move || {
                match entered.send(()) {
                    | Ok(_) => (),
                    | Err(error) => panic!("Could not send the worker message: {error:?}"),
                };
                let _ = paused.recv();
                Err("simulated preload failure".into())
            }),
        ) {
            | Ok(value) => value,
            | Err(error) => panic!("Could not start the test search worker: {error:?}"),
        };
        match started.recv_timeout(Duration::from_secs(3)) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not receive the worker response: {error:?}"),
        };
        for id in 1 ..= 3 {
            assert!(worker.submit(id, format!("query {id}")).is_ok());
        }
        assert!(worker.responses.try_recv().is_err());
        match release.send(()) {
            | Ok(_) => (),
            | Err(error) => panic!("Could not send the worker message: {error:?}"),
        };
        match worker.ready.recv_timeout(Duration::from_secs(3)) {
            | Ok(Err(_)) => (),
            | Ok(Ok(())) => panic!("Expected the search model preload to fail"),
            | Err(error) => panic!("Could not receive the worker response: {error:?}"),
        };
        assert!(matches!(
            match worker.responses.recv_timeout(Duration::from_secs(3)) {
                | Ok(value) => value,
                | Err(error) => panic!("Could not receive the worker response: {error:?}"),
            },
            Response {
                revision: 3,
                result: Ok(_)
            }
        ));
        assert!(worker.responses.try_recv().is_err());
    }
}
