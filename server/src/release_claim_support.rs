//! Response shaping and fail-closed validation helpers for release claims.

use axum::{http::StatusCode, Json};
use serde_json::{json, Value};
use tokio::sync::MutexGuard;

use super::{
    err, lane, lane_mut, parse_kind, ClaimResponse, FinishRequest, InFlightBuilder, Lane,
    PublicPublishLeaseEntry,
};

pub(super) async fn persist_or_restore(
    manager: &crate::release_manager::ReleaseManager,
    guard: &mut MutexGuard<'_, crate::release_manager::ReleaseStateFile>,
    original: crate::release_manager::ReleaseStateFile,
) -> Result<(), (StatusCode, Json<Value>)> {
    if let Err(error) = manager.persist(guard).await {
        **guard = original;
        return Err(persist_error(error));
    }
    Ok(())
}

pub(super) fn adopt_legacy_batch_identity(
    state: &mut crate::release_manager::ReleaseStateFile,
    kind: Lane,
    token: &str,
    batch_id: &str,
) {
    let stage = crate::release_batch::default_stage(kind.as_str()).to_string();
    if let Some(owner) = state
        .global_publish
        .owner
        .as_mut()
        .filter(|entry| entry.token == token && entry.batch_id.is_empty())
    {
        owner.batch_id = batch_id.to_string();
        owner.stage = stage.clone();
    }
    if let Some(waiter) = state
        .global_publish
        .waiters
        .iter_mut()
        .find(|entry| entry.token == token && entry.batch_id.is_empty())
    {
        waiter.batch_id = batch_id.to_string();
        waiter.stage = stage.clone();
    }
    if let Some(build) = lane_mut(state, kind)
        .in_flight
        .iter_mut()
        .find(|entry| entry.token == token && entry.batch_id.is_empty())
    {
        build.batch_id = batch_id.to_string();
        build.stage = stage;
    }
}

pub(super) fn claim_response_for_existing(
    state: &crate::release_manager::ReleaseStateFile,
    kind: Lane,
    lease: &crate::release_manager::PublishLeaseEntry,
    action: &str,
    queue_position: usize,
) -> ClaimResponse {
    let build = lane(state, kind)
        .in_flight
        .iter()
        .find(|item| item.token == lease.token);
    ClaimResponse {
        action: action.to_string(),
        kind: kind.as_str().to_string(),
        token: lease.token.clone(),
        sha: lease.sha.clone(),
        batch_id: lease.batch_id.clone(),
        stage: lease.stage.clone(),
        assigned_version_name: build
            .map(|item| item.assigned_version_name.clone())
            .unwrap_or_default(),
        assigned_version_code: build.and_then(|item| item.assigned_version_code),
        claimed_at: lease.requested_at,
        lease_expires_at: lease.lease_expires_at,
        in_flight_count: usize::from(state.global_publish.owner.is_some())
            + state.global_publish.waiters.len(),
        queue_position,
        coalesced: false,
        owner: state
            .global_publish
            .owner
            .as_ref()
            .map(PublicPublishLeaseEntry::from),
        waiter_count: state.global_publish.waiters.len(),
    }
}

pub(super) fn publish_token_status(
    state: &crate::release_manager::ReleaseStateFile,
    token: &str,
) -> Value {
    if let Some(owner) = state
        .global_publish
        .owner
        .as_ref()
        .filter(|owner| owner.token == token)
    {
        let kind = parse_kind(&owner.kind).ok();
        let build = kind.and_then(|kind| {
            lane(state, kind)
                .in_flight
                .iter()
                .find(|item| item.token == token)
        });
        return json!({
            "action": "build", "token": token, "kind": owner.kind, "sha": owner.sha,
            "batchId": owner.batch_id, "stage": owner.stage,
            "assignedVersionName": build.map(|item| item.assigned_version_name.clone()),
            "assignedVersionCode": build.and_then(|item| item.assigned_version_code),
            "queuePosition": 0,
        });
    }
    if let Some((index, waiter)) = state
        .global_publish
        .waiters
        .iter()
        .enumerate()
        .find(|(_, waiter)| waiter.token == token)
    {
        return json!({
            "action": "wait", "token": token, "kind": waiter.kind, "sha": waiter.sha,
            "batchId": waiter.batch_id, "stage": waiter.stage, "queuePosition": index + 1,
            "owner": state.global_publish.owner.as_ref().map(PublicPublishLeaseEntry::from),
        });
    }
    if let Some(completion) = state
        .global_publish
        .completed
        .iter()
        .rev()
        .find(|completion| completion.token == token)
    {
        return json!({
            "action": if completion.success && completion.coalesced { "coalesced" } else { "finished" },
            "token": token, "kind": completion.kind, "sha": completion.sha,
            "success": completion.success, "coalesced": completion.coalesced,
            "errorMessage": completion.error_message,
        });
    }
    json!({"action": "unknown", "token": token})
}

pub(super) fn public_in_flight(item: &InFlightBuilder) -> Value {
    json!({
        "builderId": item.builder_id, "builderLabel": item.builder_label, "sha": item.sha,
        "batchId": item.batch_id, "stage": item.stage,
        "assignedVersionName": item.assigned_version_name,
        "assignedVersionCode": item.assigned_version_code, "claimedAt": item.claimed_at,
        "lastHeartbeat": item.last_heartbeat, "leaseExpiresAt": item.lease_expires_at,
    })
}

pub(super) fn ensure_manager_healthy(
    manager: &crate::release_manager::ReleaseManager,
) -> Result<(), (StatusCode, Json<Value>)> {
    if let Some(error) = manager.health_error() {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "release-state-failed-closed",
            error,
        ));
    }
    Ok(())
}

pub(super) fn persist_error(error: anyhow::Error) -> (StatusCode, Json<Value>) {
    err(
        StatusCode::SERVICE_UNAVAILABLE,
        "release-state-persist-failed",
        &error.to_string(),
    )
}

pub(super) fn validate_finish_identity<'a>(
    build: &InFlightBuilder,
    request: &FinishRequest,
) -> Result<(), (&'a str, &'a str)> {
    if request
        .sha
        .as_deref()
        .is_some_and(|sha| sha.trim() != build.sha)
    {
        return Err((
            "immutable-sha-mismatch",
            "finish sha must match the immutable sha captured by claim",
        ));
    }
    if request
        .version_name
        .as_deref()
        .is_some_and(|version| version.trim() != build.assigned_version_name)
    {
        return Err((
            "immutable-version-mismatch",
            "finish version must match the version captured by claim",
        ));
    }
    if request
        .version_code
        .is_some_and(|version| Some(version) != build.assigned_version_code)
    {
        return Err((
            "immutable-version-code-mismatch",
            "finish version code must match the version captured by claim",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release_manager::{InFlightBuilder, PublishLeaseEntry};

    #[test]
    fn legacy_token_adopts_deterministic_batch_across_global_and_lane_state() {
        let mut state = crate::release_manager::ReleaseStateFile::default();
        state.server.in_flight.push(InFlightBuilder {
            token: "token".into(),
            builder_id: "builder".into(),
            builder_label: "builder".into(),
            sha: "fixed-sha".into(),
            batch_id: String::new(),
            stage: String::new(),
            assigned_version_name: "1.2.3".into(),
            assigned_version_code: None,
            claimed_at: 1,
            last_heartbeat: 1,
            lease_expires_at: 100,
        });
        state.global_publish.owner = Some(PublishLeaseEntry {
            token: "token".into(),
            kind: "server".into(),
            sha: "fixed-sha".into(),
            batch_id: String::new(),
            stage: String::new(),
            builder_id: "builder".into(),
            builder_label: "builder".into(),
            requested_at: 1,
            last_heartbeat: 1,
            lease_expires_at: 100,
        });

        adopt_legacy_batch_identity(&mut state, Lane::Server, "token", "release-fixed-sha");

        let owner = state.global_publish.owner.as_ref().unwrap();
        assert_eq!(owner.batch_id, "release-fixed-sha");
        assert_eq!(owner.stage, "server");
        assert_eq!(state.server.in_flight[0].batch_id, "release-fixed-sha");
        assert_eq!(state.server.in_flight[0].stage, "server");
    }
}
