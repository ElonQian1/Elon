//! 项目级任务调度器与 Codex 预热注册表（从 types.rs 抽出）。
//!
//! - `ProjectTaskScheduler` / `ProjectTaskPermit`: 按字符串 key 串行化共享项目动作
//!   （merge、publish 等），并发会话工作树则用不同 key 并行。
//! - `CodexPrewarmRegistry`: Codex CLI 原生 session 预热的冷却 + 活跃登记。

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

pub struct ProjectTaskScheduler {
    locks: AsyncMutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

pub struct ProjectTaskPermit {
    was_queued: bool,
    _guard: OwnedMutexGuard<()>,
}

pub struct CodexPrewarmRegistry {
    recent: AsyncMutex<HashMap<String, Instant>>,
    active: AsyncMutex<HashMap<String, bool>>,
}

impl CodexPrewarmRegistry {
    pub fn new() -> Self {
        Self {
            recent: AsyncMutex::new(HashMap::new()),
            active: AsyncMutex::new(HashMap::new()),
        }
    }

    pub async fn start_if_allowed(&self, key: &str, cooldown: Duration) -> bool {
        let now = Instant::now();
        let mut recent = self.recent.lock().await;
        recent.retain(|_, started_at| now.duration_since(*started_at) < cooldown);
        if recent.contains_key(key) {
            return false;
        }
        recent.insert(key.to_string(), now);
        drop(recent);

        let mut active = self.active.lock().await;
        if active.contains_key(key) {
            return false;
        }
        active.insert(key.to_string(), false);
        true
    }

    pub async fn cancel(&self, key: &str) {
        if let Some(cancelled) = self.active.lock().await.get_mut(key) {
            *cancelled = true;
        }
    }

    pub async fn finish(&self, key: &str) -> bool {
        self.active
            .lock()
            .await
            .remove(key)
            .map(|cancelled| !cancelled)
            .unwrap_or(true)
    }
}

impl ProjectTaskScheduler {
    pub fn new() -> Self {
        Self {
            locks: AsyncMutex::new(HashMap::new()),
        }
    }

    pub async fn acquire<F>(&self, project_id: &str, on_queued: F) -> ProjectTaskPermit
    where
        F: FnOnce(),
    {
        let lock = {
            let mut locks = self.locks.lock().await;
            locks
                .entry(project_id.to_string())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };

        match lock.clone().try_lock_owned() {
            Ok(guard) => ProjectTaskPermit {
                was_queued: false,
                _guard: guard,
            },
            Err(_) => {
                on_queued();
                let guard = lock.lock_owned().await;
                ProjectTaskPermit {
                    was_queued: true,
                    _guard: guard,
                }
            }
        }
    }
}

impl ProjectTaskPermit {
    pub fn was_queued(&self) -> bool {
        self.was_queued
    }
}
