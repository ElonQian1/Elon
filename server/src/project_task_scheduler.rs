//! 项目级任务调度器与 Codex 预热注册表（从 types.rs 抽出）。
//!
//! - `ProjectTaskScheduler` / `ProjectTaskPermit`: 按字符串 key 串行化共享项目动作
//!   （merge、publish 等），并发会话工作树则用不同 key 并行。
//! - `CodexPrewarmRegistry`: Codex CLI 原生 session 预热的冷却 + 活跃登记。
//! - `RouteASessionLeaseRegistry`: Route A 本机节点工作区/CLI 可用性的短期热状态。

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

#[derive(Debug, Clone)]
pub struct RouteASessionLeaseSnapshot {
    pub project_id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub agent_id: String,
    pub workspace_path: String,
    pub connected_at: u64,
    pub ttl_secs: u64,
    pub age_ms: u128,
}

struct RouteASessionLeaseEntry {
    project_id: String,
    user_id: String,
    conversation_id: String,
    agent_id: String,
    workspace_path: String,
    connected_at: u64,
    ttl: Duration,
    created_at: Instant,
    expires_at: Instant,
}

pub struct RouteASessionLeaseRegistry {
    leases: AsyncMutex<HashMap<String, RouteASessionLeaseEntry>>,
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

impl RouteASessionLeaseRegistry {
    pub fn new() -> Self {
        Self {
            leases: AsyncMutex::new(HashMap::new()),
        }
    }

    pub async fn get_valid(
        &self,
        key: &str,
        connected_at: u64,
    ) -> Option<RouteASessionLeaseSnapshot> {
        let now = Instant::now();
        let mut leases = self.leases.lock().await;
        let is_invalid = leases
            .get(key)
            .map(|entry| entry.connected_at != connected_at || entry.expires_at <= now)
            .unwrap_or(false);
        if is_invalid {
            leases.remove(key);
            return None;
        }
        leases.get(key).map(|entry| entry.snapshot(now))
    }

    pub async fn record_verified(
        &self,
        key: String,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
        agent_id: &str,
        workspace_path: &str,
        connected_at: u64,
        ttl: Duration,
    ) -> RouteASessionLeaseSnapshot {
        let now = Instant::now();
        let entry = RouteASessionLeaseEntry {
            project_id: project_id.to_string(),
            user_id: user_id.to_string(),
            conversation_id: conversation_id.to_string(),
            agent_id: agent_id.to_string(),
            workspace_path: workspace_path.to_string(),
            connected_at,
            ttl,
            created_at: now,
            expires_at: now + ttl,
        };
        let snapshot = entry.snapshot(now);
        self.leases.lock().await.insert(key, entry);
        snapshot
    }

    pub async fn invalidate(&self, key: &str) {
        self.leases.lock().await.remove(key);
    }
}

impl RouteASessionLeaseEntry {
    fn snapshot(&self, now: Instant) -> RouteASessionLeaseSnapshot {
        RouteASessionLeaseSnapshot {
            project_id: self.project_id.clone(),
            user_id: self.user_id.clone(),
            conversation_id: self.conversation_id.clone(),
            agent_id: self.agent_id.clone(),
            workspace_path: self.workspace_path.clone(),
            connected_at: self.connected_at,
            ttl_secs: self.ttl.as_secs(),
            age_ms: now.duration_since(self.created_at).as_millis(),
        }
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
                // 排队等待，最多等待 30 分钟；超时则强制获取（前一个任务可能已经异常卡死）
                match tokio::time::timeout(Duration::from_secs(30 * 60), lock.lock_owned()).await {
                    Ok(guard) => ProjectTaskPermit {
                        was_queued: true,
                        _guard: guard,
                    },
                    Err(_) => {
                        // 超时：说明前一个任务已超 30 分钟，强行创建新锁获取权限
                        tracing::warn!(key = %project_id, "project lock wait timeout (30m), forcing new slot");
                        let fresh_lock = Arc::new(AsyncMutex::new(()));
                        self.locks
                            .lock()
                            .await
                            .insert(project_id.to_string(), fresh_lock.clone());
                        let guard = fresh_lock.lock_owned().await;
                        ProjectTaskPermit {
                            was_queued: true,
                            _guard: guard,
                        }
                    }
                }
            }
        }
    }

    pub async fn try_acquire(&self, project_id: &str) -> Option<ProjectTaskPermit> {
        let lock = {
            let mut locks = self.locks.lock().await;
            locks
                .entry(project_id.to_string())
                .or_insert_with(|| Arc::new(AsyncMutex::new(())))
                .clone()
        };

        lock.try_lock_owned().ok().map(|guard| ProjectTaskPermit {
            was_queued: false,
            _guard: guard,
        })
    }
}

impl ProjectTaskPermit {
    pub fn was_queued(&self) -> bool {
        self.was_queued
    }

    pub fn mark_queued(mut self) -> Self {
        self.was_queued = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::RouteASessionLeaseRegistry;
    use std::time::Duration;

    #[tokio::test]
    async fn route_a_session_lease_reuses_valid_entry() {
        let registry = RouteASessionLeaseRegistry::new();
        let key = "p|u|c|node|workspace";
        registry
            .record_verified(
                key.to_string(),
                "p",
                "u",
                "c",
                "node",
                "workspace",
                100,
                Duration::from_secs(60),
            )
            .await;

        let snapshot = registry.get_valid(key, 100).await.unwrap();
        assert_eq!(snapshot.project_id, "p");
        assert_eq!(snapshot.user_id, "u");
        assert_eq!(snapshot.conversation_id, "c");
        assert_eq!(snapshot.agent_id, "node");
        assert_eq!(snapshot.workspace_path, "workspace");
        assert_eq!(snapshot.connected_at, 100);
        assert_eq!(snapshot.ttl_secs, 60);
    }

    #[tokio::test]
    async fn route_a_session_lease_expires_after_node_reconnect() {
        let registry = RouteASessionLeaseRegistry::new();
        let key = "p|u|c|node|workspace";
        registry
            .record_verified(
                key.to_string(),
                "p",
                "u",
                "c",
                "node",
                "workspace",
                100,
                Duration::from_secs(60),
            )
            .await;

        assert!(registry.get_valid(key, 101).await.is_none());
        assert!(registry.get_valid(key, 100).await.is_none());
    }
}
