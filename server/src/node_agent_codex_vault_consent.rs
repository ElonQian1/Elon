//! Explicit-consent and idempotency boundary for user-initiated vault changes.
//!
//! The journal stores only operation metadata. It never stores auth.json,
//! provider tokens, cloud responses, or account identifiers.

use axum::{http::StatusCode, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MAX_OPERATIONS: usize = 128;
const RETENTION: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VaultOperation {
    Backup,
    Restore,
    RestoreShared,
    ClearLocal,
    DeleteCloud,
}

impl VaultOperation {
    fn id(self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::Restore => "restore",
            Self::RestoreShared => "restore_shared",
            Self::ClearLocal => "clear_local",
            Self::DeleteCloud => "delete_cloud",
        }
    }

    fn confirmation(self) -> &'static str {
        match self {
            Self::Backup => "BACKUP_CODEX_VAULT",
            Self::Restore => "RESTORE_CODEX_VAULT",
            Self::RestoreShared => "RESTORE_SHARED_CODEX_VAULT",
            Self::ClearLocal => "CLEAR_MANAGED_CODEX_HOME",
            Self::DeleteCloud => "DELETE_CLOUD_CODEX_VAULT",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct VaultOperationRequest {
    pub(crate) request_id: String,
    #[serde(default)]
    pub(crate) explicit_consent: bool,
    pub(crate) confirmation: Option<String>,
    pub(crate) purpose: Option<String>,
}

#[derive(Clone, Debug)]
struct OperationEntry {
    operation: &'static str,
    request_fingerprint: String,
    state: &'static str,
    started_at_ms: u64,
    updated_at_ms: u64,
    error_code: Option<&'static str>,
}

#[derive(Default)]
struct OperationJournal {
    entries: HashMap<String, OperationEntry>,
    order: VecDeque<String>,
}

pub(crate) enum BeginOperation {
    Started(OperationGuard),
    Replay(StatusCode, Json<Value>),
}

pub(crate) struct OperationGuard {
    request_id: String,
    finished: bool,
}

impl OperationGuard {
    pub(crate) fn complete(mut self) {
        finish(&self.request_id, "completed", None);
        self.finished = true;
    }

    pub(crate) fn fail(mut self, error_code: &'static str) {
        finish(&self.request_id, "failed", Some(error_code));
        self.finished = true;
    }
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        if !self.finished {
            finish(
                &self.request_id,
                "failed",
                Some("operation_interrupted_before_receipt"),
            );
        }
    }
}

pub(crate) fn routes() -> Router<std::sync::Arc<crate::NodeRuntime>> {
    Router::new().route("/api/codex-vault/operations", get(list_operations))
}

async fn list_operations() -> Json<Value> {
    let mut journal = journal().lock().unwrap_or_else(|lock| lock.into_inner());
    cleanup(&mut journal);
    let operations = journal
        .order
        .iter()
        .rev()
        .filter_map(|request_id| {
            journal
                .entries
                .get(request_id)
                .map(|entry| (request_id, entry))
        })
        .map(|(request_id, entry)| operation_json(request_id, entry))
        .collect::<Vec<_>>();
    Json(json!({
        "ok": true,
        "schema": "elon.codex_vault_operation_journal.v1",
        "retention_hours": 24,
        "max_operations": MAX_OPERATIONS,
        "secrets_persisted": false,
        "operations": operations,
    }))
}

pub(crate) fn begin_operation(
    operation: VaultOperation,
    request: &VaultOperationRequest,
    context: Option<&str>,
) -> Result<BeginOperation, (StatusCode, Json<Value>)> {
    validate_consent(operation, request)?;
    let request_fingerprint = fingerprint(operation, request, context);
    let now = now_ms();
    let mut journal = journal().lock().unwrap_or_else(|lock| lock.into_inner());
    cleanup(&mut journal);
    if let Some(existing) = journal.entries.get(&request.request_id) {
        if existing.operation != operation.id() {
            return Err(error_response(
                StatusCode::CONFLICT,
                "request_id_operation_conflict",
                "request_id 已用于其他保险箱操作",
            ));
        }
        if existing.request_fingerprint != request_fingerprint {
            return Err(error_response(
                StatusCode::CONFLICT,
                "request_id_payload_conflict",
                "request_id 的请求内容与首次操作不一致",
            ));
        }
        let status = match existing.state {
            "running" => StatusCode::ACCEPTED,
            "completed" => StatusCode::OK,
            _ => StatusCode::CONFLICT,
        };
        return Ok(BeginOperation::Replay(
            status,
            Json(json!({
                "ok": existing.state == "completed",
                "replayed": true,
                "operation": operation_json(&request.request_id, existing),
            })),
        ));
    }
    while journal.entries.len() >= MAX_OPERATIONS {
        if !evict_oldest_terminal(&mut journal) {
            return Err(error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "vault_operation_capacity_exhausted",
                "当前保险箱操作过多，请等待正在运行的操作结束",
            ));
        }
    }
    journal.order.push_back(request.request_id.clone());
    journal.entries.insert(
        request.request_id.clone(),
        OperationEntry {
            operation: operation.id(),
            request_fingerprint,
            state: "running",
            started_at_ms: now,
            updated_at_ms: now,
            error_code: None,
        },
    );
    Ok(BeginOperation::Started(OperationGuard {
        request_id: request.request_id.clone(),
        finished: false,
    }))
}

fn validate_consent(
    operation: VaultOperation,
    request: &VaultOperationRequest,
) -> Result<(), (StatusCode, Json<Value>)> {
    if !valid_request_id(&request.request_id) {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request_id",
            "必须提供 8-128 位安全 request_id",
        ));
    }
    if !request.explicit_consent
        || request.confirmation.as_deref() != Some(operation.confirmation())
    {
        return Err(error_response(
            StatusCode::PRECONDITION_REQUIRED,
            "explicit_vault_consent_required",
            "保险箱写入、恢复或删除操作必须由用户明确确认",
        ));
    }
    Ok(())
}

fn valid_request_id(value: &str) -> bool {
    let value = value.trim();
    (8..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn fingerprint(
    operation: VaultOperation,
    request: &VaultOperationRequest,
    context: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    for part in [
        operation.id(),
        request.purpose.as_deref().unwrap_or_default(),
        context.unwrap_or_default(),
    ] {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn finish(request_id: &str, state: &'static str, error_code: Option<&'static str>) {
    let mut journal = journal().lock().unwrap_or_else(|lock| lock.into_inner());
    if let Some(entry) = journal.entries.get_mut(request_id) {
        entry.state = state;
        entry.error_code = error_code;
        entry.updated_at_ms = now_ms();
    }
}

fn cleanup(journal: &mut OperationJournal) {
    let cutoff = now_ms().saturating_sub(RETENTION.as_millis() as u64);
    journal
        .entries
        .retain(|_, entry| entry.updated_at_ms >= cutoff);
    journal
        .order
        .retain(|request_id| journal.entries.contains_key(request_id));
    trim_to_limit(journal);
}

fn trim_to_limit(journal: &mut OperationJournal) {
    while journal.order.len() > MAX_OPERATIONS {
        if !evict_oldest_terminal(journal) {
            break;
        }
    }
}

fn evict_oldest_terminal(journal: &mut OperationJournal) -> bool {
    let Some(position) = journal.order.iter().position(|request_id| {
        journal
            .entries
            .get(request_id)
            .is_some_and(|entry| entry.state != "running")
    }) else {
        return false;
    };
    if let Some(request_id) = journal.order.remove(position) {
        journal.entries.remove(&request_id);
    }
    true
}

fn operation_json(request_id: &str, entry: &OperationEntry) -> Value {
    json!({
        "request_id": request_id,
        "operation": entry.operation,
        "state": entry.state,
        "started_at_ms": entry.started_at_ms,
        "updated_at_ms": entry.updated_at_ms,
        "error_code": entry.error_code,
    })
}

fn error_response(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({"ok": false, "code": code, "error": message})),
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn journal() -> &'static Mutex<OperationJournal> {
    static JOURNAL: OnceLock<Mutex<OperationJournal>> = OnceLock::new();
    JOURNAL.get_or_init(|| Mutex::new(OperationJournal::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, confirmation: &str) -> VaultOperationRequest {
        VaultOperationRequest {
            request_id: id.to_string(),
            explicit_consent: true,
            confirmation: Some(confirmation.to_string()),
            purpose: None,
        }
    }

    #[test]
    fn phase2_contract_consent_is_fail_closed_and_exact_replay_is_safe() {
        let mut missing = request("vault-test-missing", "WRONG");
        missing.explicit_consent = false;
        assert!(begin_operation(VaultOperation::Backup, &missing, None).is_err());

        let valid = request("vault-test-replay", "BACKUP_CODEX_VAULT");
        let BeginOperation::Started(guard) =
            begin_operation(VaultOperation::Backup, &valid, None).unwrap()
        else {
            panic!("first request must start");
        };
        guard.complete();
        assert!(matches!(
            begin_operation(VaultOperation::Backup, &valid, None).unwrap(),
            BeginOperation::Replay(StatusCode::OK, _)
        ));
        let mut changed = valid.clone();
        changed.purpose = Some("different-purpose".to_string());
        assert!(begin_operation(VaultOperation::Backup, &changed, None).is_err());
        assert!(begin_operation(VaultOperation::DeleteCloud, &valid, None).is_err());
    }
}
