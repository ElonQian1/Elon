//! Durable multi-surface release batch ledger.

use serde::{Deserialize, Serialize};

use crate::release_manager::{PublishLeaseEntry, ReleaseStateFile};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReleaseBatchStage {
    pub stage: String,
    pub kind: String,
    pub status: String,
    pub builder_id: String,
    pub builder_label: String,
    pub attempt: u32,
    pub requested_at: i64,
    pub last_heartbeat: i64,
    pub lease_expires_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReleaseBatchLedger {
    pub batch_id: String,
    pub sha: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub stages: Vec<ReleaseBatchStage>,
}

pub(crate) fn default_batch_id(sha: &str) -> String {
    format!("release-{}", sha.trim())
}

pub(crate) fn default_stage(kind: &str) -> &'static str {
    match kind {
        "server" => "server",
        "node_agent" => "windows_node",
        "apk" => "android_apk",
        _ => "unknown",
    }
}

pub(crate) fn validate_batch_identity(
    state: &ReleaseStateFile,
    batch_id: &str,
    sha: &str,
) -> Result<(), &'static str> {
    if batch_id.trim().is_empty() || sha.trim().is_empty() {
        return Err("release batch id and sha are required");
    }
    if state
        .release_batches
        .iter()
        .any(|batch| batch.batch_id == batch_id && batch.sha != sha)
    {
        return Err("release batch id is already bound to another immutable sha");
    }
    Ok(())
}

pub(crate) fn record_claim(
    state: &mut ReleaseStateFile,
    lease: &PublishLeaseEntry,
    status: &str,
    now: i64,
) {
    record_stage(
        state,
        &lease.batch_id,
        &lease.sha,
        &lease.kind,
        &lease.stage,
        &lease.builder_id,
        &lease.builder_label,
        status,
        lease.lease_expires_at,
        None,
        now,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_stage(
    state: &mut ReleaseStateFile,
    batch_id: &str,
    sha: &str,
    kind: &str,
    stage: &str,
    builder_id: &str,
    builder_label: &str,
    status: &str,
    lease_expires_at: i64,
    error_message: Option<String>,
    now: i64,
) {
    let batch_index = state
        .release_batches
        .iter()
        .position(|batch| batch.batch_id == batch_id)
        .unwrap_or_else(|| {
            state.release_batches.push(ReleaseBatchLedger {
                batch_id: batch_id.to_string(),
                sha: sha.to_string(),
                status: "in_progress".to_string(),
                created_at: now,
                updated_at: now,
                stages: Vec::new(),
            });
            state.release_batches.len() - 1
        });
    let batch = &mut state.release_batches[batch_index];
    if let Some(current) = batch.stages.iter_mut().find(|item| item.stage == stage) {
        if current.builder_id != builder_id && matches!(status, "running" | "queued") {
            current.attempt = current.attempt.saturating_add(1);
        }
        current.kind = kind.to_string();
        current.status = normalize_status(status).to_string();
        current.builder_id = builder_id.to_string();
        current.builder_label = builder_label.to_string();
        current.last_heartbeat = now;
        current.lease_expires_at = lease_expires_at;
        current.error_message = error_message;
        current.completed_at = terminal_status(&current.status).then_some(now);
    } else {
        let status = normalize_status(status).to_string();
        batch.stages.push(ReleaseBatchStage {
            stage: stage.to_string(),
            kind: kind.to_string(),
            status: status.clone(),
            builder_id: builder_id.to_string(),
            builder_label: builder_label.to_string(),
            attempt: 1,
            requested_at: now,
            last_heartbeat: now,
            lease_expires_at,
            completed_at: terminal_status(&status).then_some(now),
            error_message,
        });
    }
    batch.updated_at = now;
    batch.status = batch_status(&batch.stages).to_string();
    if state.release_batches.len() > 100 {
        let remove = state.release_batches.len() - 100;
        state.release_batches.drain(..remove);
    }
}

pub(crate) fn expire_stage(state: &mut ReleaseStateFile, owner: &PublishLeaseEntry, now: i64) {
    record_stage(
        state,
        &owner.batch_id,
        &owner.sha,
        &owner.kind,
        &owner.stage,
        &owner.builder_id,
        &owner.builder_label,
        "expired",
        owner.lease_expires_at,
        Some("publish owner lease expired".to_string()),
        now,
    );
}

fn normalize_status(status: &str) -> &'static str {
    match status {
        "queued" => "queued",
        "running" => "running",
        "succeeded" => "succeeded",
        "failed" => "failed",
        "expired" => "expired",
        _ => "unknown",
    }
}

fn terminal_status(status: &str) -> bool {
    matches!(status, "succeeded" | "failed" | "expired")
}

fn batch_status(stages: &[ReleaseBatchStage]) -> &'static str {
    if stages
        .iter()
        .any(|stage| matches!(stage.status.as_str(), "failed" | "expired" | "unknown"))
    {
        "failed_closed"
    } else if stages.iter().all(|stage| stage.status == "succeeded") {
        "succeeded"
    } else {
        "in_progress"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immutable_batch_sha_and_stage_takeover_are_durable() {
        let mut state = ReleaseStateFile::default();
        let lease = PublishLeaseEntry {
            token: "one".into(),
            kind: "server".into(),
            sha: "sha-a".into(),
            batch_id: "batch-a".into(),
            stage: "pc_frontend".into(),
            builder_id: "builder-a".into(),
            builder_label: "A".into(),
            requested_at: 1,
            last_heartbeat: 1,
            lease_expires_at: 100,
        };
        record_claim(&mut state, &lease, "running", 1);
        assert!(validate_batch_identity(&state, "batch-a", "sha-a").is_ok());
        assert!(validate_batch_identity(&state, "batch-a", "sha-b").is_err());

        let mut takeover = lease.clone();
        takeover.builder_id = "builder-b".into();
        record_claim(&mut state, &takeover, "running", 2);
        assert_eq!(state.release_batches[0].stages[0].attempt, 2);
    }

    #[test]
    fn unknown_stage_status_fails_closed() {
        let mut state = ReleaseStateFile::default();
        record_stage(
            &mut state, "batch-a", "sha-a", "server", "server", "builder", "builder", "mystery",
            10, None, 1,
        );
        assert_eq!(state.release_batches[0].status, "failed_closed");
        assert_eq!(state.release_batches[0].stages[0].status, "unknown");
    }
}
