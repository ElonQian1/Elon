//! Sanitized, bounded provider-login journal used for restart recovery.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::node_agent_provider_auth_attempt::ProviderLoginAttempt;

const JOURNAL_SCHEMA: u32 = 2;
const MAX_ATTEMPTS: usize = 64;

#[derive(Clone)]
pub(crate) struct ProviderAuthAttemptStore {
    path: Arc<PathBuf>,
    attempts: Arc<Mutex<HashMap<String, PersistedAttempt>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JournalFile {
    schema_version: u32,
    attempts: Vec<PersistedAttempt>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedAttempt {
    login_id: String,
    provider_id: String,
    flow: String,
    state: String,
    request_id: Option<String>,
    verification_url: Option<String>,
    remote_compatible: bool,
    error: Option<String>,
    error_code: Option<String>,
    started_at_ms: u64,
    updated_at_ms: u64,
}

impl ProviderAuthAttemptStore {
    pub(crate) fn load(path: PathBuf) -> (Self, Vec<ProviderLoginAttempt>) {
        let parsed = read_journal(&path)
            .or_else(|| read_journal(&backup_path(&path)))
            .map(|journal| journal.attempts)
            .unwrap_or_default();
        let attempts = parsed
            .into_iter()
            .map(PersistedAttempt::recover)
            .collect::<Vec<_>>();
        let store = Self {
            path: Arc::new(path),
            attempts: Arc::new(Mutex::new(HashMap::new())),
        };
        for attempt in &attempts {
            store.upsert(attempt);
        }
        (store, attempts)
    }

    pub(crate) fn upsert(&self, attempt: &ProviderLoginAttempt) {
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(|lock| lock.into_inner());
        attempts.insert(
            attempt.login_id.clone(),
            PersistedAttempt::from_attempt(attempt),
        );
        if attempts.len() > MAX_ATTEMPTS {
            let mut oldest = attempts
                .values()
                .map(|attempt| (attempt.updated_at_ms, attempt.login_id.clone()))
                .collect::<Vec<_>>();
            oldest.sort_by_key(|entry| entry.0);
            for (_, id) in oldest.into_iter().take(attempts.len() - MAX_ATTEMPTS) {
                attempts.remove(&id);
            }
        }
        self.write_locked(&attempts);
    }

    pub(crate) fn remove(&self, ids: &[String]) {
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(|lock| lock.into_inner());
        for id in ids {
            attempts.remove(id);
        }
        self.write_locked(&attempts);
    }

    fn write_locked(&self, attempts: &HashMap<String, PersistedAttempt>) {
        if let Some(parent) = self.path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                tracing::warn!(error = %error, path = %parent.display(), "无法创建厂商登录日志目录");
                return;
            }
        }
        let mut values = attempts.values().cloned().collect::<Vec<_>>();
        values.sort_by_key(|attempt| attempt.started_at_ms);
        if let Ok(bytes) = serde_json::to_vec_pretty(&JournalFile {
            schema_version: JOURNAL_SCHEMA,
            attempts: values,
        }) {
            if let Some(current) = read_journal(self.path.as_ref()) {
                if let Ok(current) = serde_json::to_vec_pretty(&current) {
                    let _ = crate::node_agent_atomic_file::write(
                        &backup_path(self.path.as_ref()),
                        &current,
                    );
                }
            }
            if let Err(error) = crate::node_agent_atomic_file::write(self.path.as_ref(), &bytes) {
                tracing::warn!(error = %error, path = %self.path.display(), "无法持久化厂商登录日志");
            }
        }
    }
}

fn read_journal(path: &Path) -> Option<JournalFile> {
    std::fs::metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file() && metadata.len() <= 1024 * 1024)
        .and_then(|_| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<JournalFile>(&bytes).ok())
        .filter(|journal| journal.schema_version == JOURNAL_SCHEMA)
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.bak",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("provider-auth-attempts.json")
    ))
}

impl PersistedAttempt {
    fn from_attempt(attempt: &ProviderLoginAttempt) -> Self {
        Self {
            login_id: attempt.login_id.clone(),
            provider_id: attempt.provider_id.clone(),
            flow: attempt.flow.clone(),
            state: attempt.state.clone(),
            request_id: attempt.request_id.clone(),
            verification_url: attempt.verification_url.as_deref().and_then(sanitized_url),
            remote_compatible: attempt.remote_compatible,
            error: attempt.error.clone(),
            error_code: attempt.error_code.clone(),
            started_at_ms: attempt.started_at_ms,
            updated_at_ms: attempt.updated_at_ms,
        }
    }

    fn recover(self) -> ProviderLoginAttempt {
        let mut attempt = ProviderLoginAttempt {
            schema_version: JOURNAL_SCHEMA,
            login_id: self.login_id,
            provider_id: self.provider_id,
            flow: self.flow,
            state: self.state,
            request_id: self.request_id,
            verification_url: self.verification_url,
            user_code: None,
            auth_url: None,
            remote_compatible: self.remote_compatible,
            recovered: false,
            error: self.error,
            error_code: self.error_code,
            started_at_ms: self.started_at_ms,
            updated_at_ms: self.updated_at_ms,
        };
        attempt.recover_after_restart(crate::node_agent_provider_auth_runtime::now_ms());
        attempt
    }
}

fn sanitized_url(value: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(value).ok()?;
    if url.scheme() != "https" {
        return None;
    }
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string().chars().take(2048).collect())
}

pub(crate) fn default_journal_path(root: Option<&Path>) -> PathBuf {
    root.map(|root| root.join("control-plane/provider-auth-attempts.json"))
        .unwrap_or_else(|| {
            crate::node_agent_config::state_path()
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("provider-auth-attempts.json")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_urls_drop_query_credentials() {
        assert_eq!(
            sanitized_url("https://accounts.example/login?secret=value#token").as_deref(),
            Some("https://accounts.example/login")
        );
        assert_eq!(sanitized_url("http://accounts.example/login"), None);
    }

    #[test]
    fn active_attempt_recovers_as_terminal_without_secrets() {
        let path = std::env::temp_dir().join(format!(
            "elon_provider_attempt_{}.json",
            uuid::Uuid::new_v4().simple()
        ));
        let (store, _) = ProviderAuthAttemptStore::load(path.clone());
        store.upsert(&ProviderLoginAttempt {
            schema_version: 2,
            login_id: "login-1".to_string(),
            provider_id: "codex_cli".to_string(),
            flow: "device_code".to_string(),
            state: "waiting_for_user".to_string(),
            request_id: Some("request-1".to_string()),
            verification_url: Some("https://example.com/device?token=secret".to_string()),
            user_code: Some("TOP-SECRET".to_string()),
            auth_url: Some("https://example.com/login?secret=value".to_string()),
            remote_compatible: true,
            recovered: false,
            error: None,
            error_code: None,
            started_at_ms: 1,
            updated_at_ms: 1,
        });
        let bytes = std::fs::read_to_string(&path).unwrap();
        assert!(!bytes.contains("TOP-SECRET"));
        assert!(!bytes.contains("?token=secret"));
        assert!(!bytes.contains("auth_url"));

        let (_, recovered) = ProviderAuthAttemptStore::load(path.clone());
        assert_eq!(recovered[0].state, "failed");
        assert_eq!(recovered[0].error_code.as_deref(), Some("node_restarted"));
        assert!(recovered[0].recovered);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn corrupt_primary_recovers_from_the_last_valid_backup() {
        let path = std::env::temp_dir().join(format!(
            "elon_provider_attempt_backup_{}.json",
            uuid::Uuid::new_v4().simple()
        ));
        let (store, _) = ProviderAuthAttemptStore::load(path.clone());
        let mut attempt = ProviderLoginAttempt {
            schema_version: 2,
            login_id: "login-backup".to_string(),
            provider_id: "gemini_cli".to_string(),
            flow: "agent".to_string(),
            state: "completed".to_string(),
            request_id: Some("request-backup".to_string()),
            verification_url: None,
            user_code: None,
            auth_url: None,
            remote_compatible: false,
            recovered: false,
            error: None,
            error_code: None,
            started_at_ms: 1,
            updated_at_ms: 1,
        };
        store.upsert(&attempt);
        attempt.updated_at_ms = 2;
        store.upsert(&attempt);
        std::fs::write(&path, b"{broken").unwrap();

        let (_, recovered) = ProviderAuthAttemptStore::load(path.clone());
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].login_id, "login-backup");
        assert_eq!(recovered[0].state, "completed");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(backup_path(&path));
    }
}
