//! Recoverable server-side home-chat tasks.
//!
//! The model must keep running when a browser connection disappears. Each
//! task keeps a bounded event journal and lets a later HTTP connection replay
//! that journal before receiving live events from the same request id.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::response::Response;
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::lm_chat_stream_support::stream_response;

const MAX_JOURNAL_EVENTS: usize = 8192;
const MAX_JOURNAL_BYTES: usize = 2 * 1024 * 1024;
const LIVE_EVENT_CAPACITY: usize = 512;
const TASK_RETENTION_SECS: u64 = 10 * 60;
const MAX_RETAINED_TASKS: usize = 256;

#[derive(Clone)]
struct JournalEvent {
    sequence: u64,
    data: String,
}

pub(crate) struct ChatStreamTask {
    sender: Mutex<Option<mpsc::Sender<String>>>,
    live_events: broadcast::Sender<JournalEvent>,
    journal: Mutex<Vec<JournalEvent>>,
    journal_bytes: Mutex<usize>,
    next_sequence: AtomicU64,
    finished: AtomicBool,
    finished_at_secs: AtomicU64,
}

static TASKS: OnceLock<Mutex<HashMap<String, Arc<ChatStreamTask>>>> = OnceLock::new();

fn tasks() -> &'static Mutex<HashMap<String, Arc<ChatStreamTask>>> {
    TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) async fn get_or_create(user_id: &str, request_id: &str) -> (Arc<ChatStreamTask>, bool) {
    let key = format!("{user_id}:{request_id}");
    let mut registry = tasks().lock().await;

    // Keep a short replay window for refresh/reconnect, but do not retain
    // every completed conversation in process memory forever.
    let now = unix_time_secs();
    registry.retain(|_, task| {
        let finished_at = task.finished_at_secs.load(Ordering::Acquire);
        finished_at == 0 || now.saturating_sub(finished_at) <= TASK_RETENTION_SECS
    });
    if registry.len() >= MAX_RETAINED_TASKS {
        if let Some(stale_key) = registry
            .iter()
            .find(|(_, task)| task.finished.load(Ordering::Acquire))
            .map(|(key, _)| key.clone())
        {
            registry.remove(&stale_key);
        }
    }
    if let Some(task) = registry.get(&key) {
        return (Arc::clone(task), false);
    }

    let (sender, receiver) = mpsc::channel(64);
    let (live_events, _) = broadcast::channel(LIVE_EVENT_CAPACITY);
    let task = Arc::new(ChatStreamTask {
        sender: Mutex::new(Some(sender)),
        live_events,
        journal: Mutex::new(Vec::new()),
        journal_bytes: Mutex::new(0),
        next_sequence: AtomicU64::new(0),
        finished: AtomicBool::new(false),
        finished_at_secs: AtomicU64::new(0),
    });
    registry.insert(key, Arc::clone(&task));
    tokio::spawn(Arc::clone(&task).pump(receiver));
    (task, true)
}

impl ChatStreamTask {
    pub(crate) async fn sender(&self) -> mpsc::Sender<String> {
        self.sender
            .lock()
            .await
            .as_ref()
            .expect("chat task sender requested after finish")
            .clone()
    }

    pub(crate) async fn finish(&self) {
        // Dropping the task-owned sender lets the pump drain all queued events
        // before marking the task finished. This guarantees a reconnect sees
        // the terminal event instead of closing one event too early.
        let _ = self.sender.lock().await.take();
    }

    pub(crate) fn response(self: &Arc<Self>) -> Response {
        let (sender, receiver) = mpsc::channel(128);
        tokio::spawn(Arc::clone(self).replay_and_follow(sender));
        stream_response(receiver)
    }

    async fn pump(self: Arc<Self>, mut receiver: mpsc::Receiver<String>) {
        while let Some(data) = receiver.recv().await {
            let event = JournalEvent {
                sequence: self.next_sequence.fetch_add(1, Ordering::AcqRel) + 1,
                data,
            };
            {
                let mut journal = self.journal.lock().await;
                let mut journal_bytes = self.journal_bytes.lock().await;
                *journal_bytes += event.data.len();
                journal.push(event.clone());
                while journal.len() > MAX_JOURNAL_EVENTS || *journal_bytes > MAX_JOURNAL_BYTES {
                    let removed = journal.remove(0);
                    *journal_bytes = journal_bytes.saturating_sub(removed.data.len());
                }
            }
            let _ = self.live_events.send(event);
        }
        self.finished.store(true, Ordering::Release);
        self.finished_at_secs
            .store(unix_time_secs(), Ordering::Release);
    }

    async fn replay_and_follow(self: Arc<Self>, sender: mpsc::Sender<String>) {
        // Subscribe before taking the snapshot. Any event racing with the
        // snapshot is received below and skipped only after its sequence is
        // known, so reconnects neither lose nor duplicate deltas.
        let mut live = self.live_events.subscribe();
        let snapshot = self.journal.lock().await.clone();
        let mut last_sequence = 0;
        for event in snapshot {
            last_sequence = event.sequence;
            if sender.send(event.data).await.is_err() {
                return;
            }
        }

        loop {
            if self.finished.load(Ordering::Acquire)
                && self.latest_sequence().await <= last_sequence
            {
                return;
            }
            match live.recv().await {
                Ok(event) if event.sequence > last_sequence => {
                    last_sequence = event.sequence;
                    if sender.send(event.data).await.is_err() {
                        return;
                    }
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let snapshot = self.journal.lock().await.clone();
                    for event in snapshot {
                        if event.sequence <= last_sequence {
                            continue;
                        }
                        last_sequence = event.sequence;
                        if sender.send(event.data).await.is_err() {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }

    async fn latest_sequence(&self) -> u64 {
        self.journal
            .lock()
            .await
            .last()
            .map(|event| event.sequence)
            .unwrap_or(0)
    }
}

fn unix_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
