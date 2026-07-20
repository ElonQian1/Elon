//! 多 builder 并发发布协调器（不阻塞编译，只分配版本号）
//!
//! 与最初 v0.3.65 的「互斥锁」语义不同，这里采用「版本号分配器」语义：
//!
//! - 任何 builder 都可以随时编译（不互斥），多人同时干活也 OK。
//! - 服务器保证每个 `claim` 拿到**唯一递增**的 `version_name` / `version_code`，
//!   两人同时 claim 不会拿到相同号码（in_flight 中的号码也算进 max）。
//! - 上传产物的并发安全由现有 `flock + .deployed-sha` CAS 处理；
//!   编完发现服务器已是更新 SHA 的后代，builder 自行放弃产物（让贤）。
//! - 版本号**完全脱离 git**：不再写回 `Cargo.toml` / `build.gradle`，
//!   编译时通过环境变量 `ELON_BUILD_VERSION(_NAME|_CODE)` 注入产物，
//!   release-state.json 是版本号的事实来源。
//!
//! 状态类型与版本号计算逻辑见 [`crate::release_manager`]。

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::release_manager::{
    bump_semver, clamp_lease, enqueue_global_publish, err, finish_global_publish, lane, lane_mut,
    manager, max_semver, now_secs, parse_kind, semver_ge, sweep_expired, sweep_global_expired,
    InFlightBuilder, Lane, LaneState, LastRelease, PublishAdmission, PublishLeaseEntry,
};

#[path = "release_claim_support.rs"]
mod support;
use crate::types::AppState;
use support::{
    adopt_legacy_batch_identity, claim_response_for_existing, ensure_manager_healthy,
    persist_error, public_in_flight, publish_token_status, validate_finish_identity,
};

// ===== 调参常量 =====

/// 默认 lease 时长（秒）。心跳一次刷新到这么长。
const DEFAULT_LEASE_SECS: i64 = 1800; // 30 min
/// `claim` 时如未给 `lease_secs`，按构建预估给的最低基线
const ESTIMATED_BUILD_SECS_SERVER: i64 = 3600;
const ESTIMATED_BUILD_SECS_APK: i64 = 3600;

// ===== 请求 / 响应 =====

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRequest {
    pub kind: String, // "server" | "apk"
    pub sha: String,
    pub builder_id: String,
    pub builder_label: Option<String>,
    /// 期望递增位：major / minor / patch（仅 server 通道用）。默认 patch。
    pub bump: Option<String>,
    /// 客户端可选地告诉服务器自己看到的当前已发布版本号，
    /// 用于 release-state.json 空文件冷启动时的兜底。
    pub current_version_name: Option<String>,
    pub current_version_code: Option<i64>,
    pub lease_secs: Option<i64>,
    pub batch_id: Option<String>,
    pub stage: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClaimResponse {
    pub action: String, // build | wait | coalesced
    pub kind: String,
    pub token: String,
    pub sha: String,
    pub batch_id: String,
    pub stage: String,
    pub assigned_version_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_version_code: Option<i64>,
    pub claimed_at: i64,
    pub lease_expires_at: i64,
    pub in_flight_count: usize,
    pub queue_position: usize,
    pub coalesced: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<PublicPublishLeaseEntry>,
    pub waiter_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicPublishLeaseEntry {
    pub kind: String,
    pub sha: String,
    pub batch_id: String,
    pub stage: String,
    pub builder_id: String,
    pub builder_label: String,
    pub requested_at: i64,
    pub last_heartbeat: i64,
    pub lease_expires_at: i64,
}

impl From<&PublishLeaseEntry> for PublicPublishLeaseEntry {
    fn from(value: &PublishLeaseEntry) -> Self {
        Self {
            kind: value.kind.clone(),
            sha: value.sha.clone(),
            batch_id: value.batch_id.clone(),
            stage: value.stage.clone(),
            builder_id: value.builder_id.clone(),
            builder_label: value.builder_label.clone(),
            requested_at: value.requested_at,
            last_heartbeat: value.last_heartbeat,
            lease_expires_at: value.lease_expires_at,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatRequest {
    pub kind: String,
    pub token: String,
    pub lease_secs: Option<i64>,
    pub batch_id: Option<String>,
    pub stage: Option<String>,
    pub stage_status: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatResponse {
    pub ok: bool,
    pub lease_expires_at: i64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FinishRequest {
    pub kind: String,
    pub token: String,
    pub success: bool,
    pub sha: Option<String>,
    pub version_name: Option<String>,
    pub version_code: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FinishResponse {
    pub ok: bool,
    pub recorded: bool,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatusQuery {
    pub kind: Option<String>,
    pub token: Option<String>,
}

// ===== Handlers =====

pub async fn claim_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<ClaimResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    ensure_manager_healthy(&mgr)?;
    let mut guard = mgr.inner.lock().await;
    let original = guard.clone();
    let now = now_secs();

    for lane_kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        sweep_expired(lane_mut(&mut guard, lane_kind), now);
    }
    sweep_global_expired(&mut guard, now);

    let lease = clamp_lease(req.lease_secs.unwrap_or(match kind {
        Lane::Server => ESTIMATED_BUILD_SECS_SERVER,
        Lane::Apk => ESTIMATED_BUILD_SECS_APK,
        Lane::NodeAgent => ESTIMATED_BUILD_SECS_SERVER,
    }));

    let kind_name = kind.as_str();
    let batch_id = req
        .batch_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| crate::release_batch::default_batch_id(&req.sha));
    let stage = req
        .stage
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| crate::release_batch::default_stage(kind_name).to_string());
    crate::release_batch::validate_batch_identity(&guard, &batch_id, &req.sha).map_err(
        |message| {
            err(
                StatusCode::CONFLICT,
                "immutable-batch-sha-mismatch",
                message,
            )
        },
    )?;
    if let Some(completion) = guard.global_publish.completed.iter().rev().find(|item| {
        item.success
            && item.kind == kind_name
            && item.sha == req.sha
            && lane(&guard, kind)
                .last_release
                .as_ref()
                .is_some_and(|release| release.success && release.sha == req.sha)
    }) {
        let lane_ref = lane(&guard, kind);
        let response = ClaimResponse {
            action: "coalesced".to_string(),
            kind: kind_name.to_string(),
            token: String::new(),
            sha: completion.sha.clone(),
            batch_id: batch_id.clone(),
            stage: stage.clone(),
            assigned_version_name: lane_ref
                .last_release
                .as_ref()
                .map(|release| release.version_name.clone())
                .unwrap_or_default(),
            assigned_version_code: lane_ref
                .last_release
                .as_ref()
                .and_then(|release| release.version_code),
            claimed_at: completion.finished_at,
            lease_expires_at: completion.finished_at,
            in_flight_count: usize::from(guard.global_publish.owner.is_some())
                + guard.global_publish.waiters.len(),
            queue_position: 0,
            coalesced: true,
            owner: guard
                .global_publish
                .owner
                .as_ref()
                .map(PublicPublishLeaseEntry::from),
            waiter_count: guard.global_publish.waiters.len(),
        };
        return Ok(Json(response));
    }

    if let Some(existing) = guard.global_publish.owner.as_ref().filter(|item| {
        item.kind == kind_name
            && item.sha == req.sha
            && item.batch_id == batch_id
            && item.stage == stage
            && item.builder_id == req.builder_id
    }) {
        return Ok(Json(claim_response_for_existing(
            &guard, kind, existing, "build", 0,
        )));
    }
    if let Some((index, existing)) =
        guard
            .global_publish
            .waiters
            .iter()
            .enumerate()
            .find(|(_, item)| {
                item.kind == kind_name
                    && item.sha == req.sha
                    && item.batch_id == batch_id
                    && item.stage == stage
                    && item.builder_id == req.builder_id
            })
    {
        return Ok(Json(claim_response_for_existing(
            &guard,
            kind,
            existing,
            "wait",
            index + 1,
        )));
    }

    let bump_kind = req.bump.as_deref().unwrap_or("patch");

    // 计算下一个版本号：取 max(last_published, current_reported, in_flight 中最大) 然后 bump。
    let lane_ref = lane(&guard, kind);
    let mut candidates_name: Vec<String> = Vec::new();
    if let Some(v) = lane_ref.last_published_version_name.clone() {
        candidates_name.push(v);
    }
    if let Some(v) = req.current_version_name.clone() {
        candidates_name.push(v);
    }
    for b in &lane_ref.in_flight {
        candidates_name.push(b.assigned_version_name.clone());
    }
    let base_name = max_semver(&candidates_name).unwrap_or_else(|| "0.0.0".to_string());
    let assigned_version_name = if matches!(kind, Lane::NodeAgent) {
        req.current_version_name
            .clone()
            .unwrap_or_else(|| base_name.clone())
    } else {
        bump_semver(&base_name, bump_kind)
    };

    // APK 通道：versionCode 单调递增整数
    let assigned_version_code = if matches!(kind, Lane::Apk) {
        let mut candidates_code: Vec<i64> = Vec::new();
        if let Some(c) = lane_ref.last_published_version_code {
            candidates_code.push(c);
        }
        if let Some(c) = req.current_version_code {
            candidates_code.push(c);
        }
        for b in &lane_ref.in_flight {
            if let Some(c) = b.assigned_version_code {
                candidates_code.push(c);
            }
        }
        let base = candidates_code.into_iter().max().unwrap_or(0);
        Some(base + 1)
    } else {
        None
    };

    let token = Uuid::new_v4().to_string();
    let lease_expires_at = now + lease;
    let builder_label = req.builder_label.unwrap_or_else(|| req.builder_id.clone());

    let entry = InFlightBuilder {
        token: token.clone(),
        builder_id: req.builder_id.clone(),
        builder_label: builder_label.clone(),
        sha: req.sha.clone(),
        batch_id: batch_id.clone(),
        stage: stage.clone(),
        assigned_version_name: assigned_version_name.clone(),
        assigned_version_code,
        claimed_at: now,
        last_heartbeat: now,
        lease_expires_at,
    };

    lane_mut(&mut guard, kind).in_flight.push(entry);
    let lease_entry = PublishLeaseEntry {
        token: token.clone(),
        kind: kind_name.to_string(),
        sha: req.sha.clone(),
        batch_id: batch_id.clone(),
        stage: stage.clone(),
        builder_id: req.builder_id.clone(),
        builder_label,
        requested_at: now,
        last_heartbeat: now,
        lease_expires_at,
    };
    let (action, queue_position) = match enqueue_global_publish(&mut guard, lease_entry) {
        PublishAdmission::Owner => ("build", 0),
        PublishAdmission::Waiter { queue_position } => ("wait", queue_position),
    };
    if let Some(lease) = guard
        .global_publish
        .owner
        .as_ref()
        .filter(|lease| lease.token == token)
        .or_else(|| {
            guard
                .global_publish
                .waiters
                .iter()
                .find(|lease| lease.token == token)
        })
        .cloned()
    {
        crate::release_batch::record_claim(
            &mut guard,
            &lease,
            if action == "build" {
                "running"
            } else {
                "queued"
            },
            now,
        );
    }
    let in_flight_count =
        usize::from(guard.global_publish.owner.is_some()) + guard.global_publish.waiters.len();

    let resp = ClaimResponse {
        action: action.to_string(),
        kind: kind_name.to_string(),
        token,
        sha: req.sha,
        batch_id,
        stage,
        assigned_version_name,
        assigned_version_code,
        claimed_at: now,
        lease_expires_at,
        in_flight_count,
        queue_position,
        coalesced: false,
        owner: guard
            .global_publish
            .owner
            .as_ref()
            .map(PublicPublishLeaseEntry::from),
        waiter_count: guard.global_publish.waiters.len(),
    };

    if let Err(error) = mgr.persist(&guard).await {
        *guard = original;
        return Err(persist_error(error));
    }
    drop(guard);
    Ok(Json(resp))
}

pub async fn heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    ensure_manager_healthy(&mgr)?;
    let mut guard = mgr.inner.lock().await;
    let original = guard.clone();
    let now = now_secs();

    for lane_kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        sweep_expired(lane_mut(&mut guard, lane_kind), now);
    }
    sweep_global_expired(&mut guard, now);

    let lease = clamp_lease(req.lease_secs.unwrap_or(DEFAULT_LEASE_SECS));
    let new_expiry = now + lease;
    let mut found = false;
    if let Some(owner) = guard
        .global_publish
        .owner
        .as_mut()
        .filter(|item| item.token == req.token && item.kind == kind.as_str())
    {
        owner.last_heartbeat = now;
        owner.lease_expires_at = new_expiry;
        found = true;
    } else if let Some(waiter) = guard
        .global_publish
        .waiters
        .iter_mut()
        .find(|item| item.token == req.token && item.kind == kind.as_str())
    {
        waiter.last_heartbeat = now;
        waiter.lease_expires_at = new_expiry;
        found = true;
    }
    if !found {
        return Err(err(
            StatusCode::GONE,
            "token-not-active",
            "lease expired or unknown token",
        ));
    }
    let mut lease_entry = guard
        .global_publish
        .owner
        .as_ref()
        .filter(|item| item.token == req.token)
        .or_else(|| {
            guard
                .global_publish
                .waiters
                .iter()
                .find(|item| item.token == req.token)
        })
        .cloned()
        .ok_or_else(|| err(StatusCode::GONE, "token-not-active", "lease disappeared"))?;
    if lease_entry.batch_id.is_empty() {
        let Some(batch_id) = req
            .batch_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            return Err(err(
                StatusCode::CONFLICT,
                "legacy-batch-required",
                "legacy publish token must adopt its deterministic release batch",
            ));
        };
        if batch_id != crate::release_batch::default_batch_id(&lease_entry.sha) {
            return Err(err(
                StatusCode::CONFLICT,
                "legacy-batch-migration-refused",
                "legacy publish token may only adopt release-<immutable-sha>",
            ));
        }
        crate::release_batch::validate_batch_identity(&guard, batch_id, &lease_entry.sha)
            .map_err(|message| err(StatusCode::CONFLICT, "batch-sha-mismatch", message))?;
        adopt_legacy_batch_identity(&mut guard, kind, &req.token, batch_id);
        lease_entry.batch_id = batch_id.to_string();
        lease_entry.stage = crate::release_batch::default_stage(kind.as_str()).to_string();
    }
    if req
        .batch_id
        .as_deref()
        .is_some_and(|batch| batch.trim() != lease_entry.batch_id)
    {
        return Err(err(
            StatusCode::CONFLICT,
            "batch-mismatch",
            "heartbeat batch does not own token",
        ));
    }
    let stage = req.stage.as_deref().unwrap_or(&lease_entry.stage);
    crate::release_batch::record_stage(
        &mut guard,
        &lease_entry.batch_id,
        &lease_entry.sha,
        &lease_entry.kind,
        stage,
        &lease_entry.builder_id,
        &lease_entry.builder_label,
        req.stage_status.as_deref().unwrap_or("running"),
        new_expiry,
        None,
        now,
    );
    if let Some(item) = lane_mut(&mut guard, kind)
        .in_flight
        .iter_mut()
        .find(|item| item.token == req.token)
    {
        item.last_heartbeat = now;
        item.lease_expires_at = new_expiry;
    }
    let lease_expires_at = new_expiry;

    let resp = HeartbeatResponse {
        ok: true,
        lease_expires_at,
    };
    if let Err(error) = mgr.persist(&guard).await {
        *guard = original;
        return Err(persist_error(error));
    }
    drop(guard);
    Ok(Json(resp))
}

pub async fn finish_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FinishRequest>,
) -> Result<Json<FinishResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    ensure_manager_healthy(&mgr)?;
    let mut guard = mgr.inner.lock().await;
    let original = guard.clone();
    let now = now_secs();
    for lane_kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        sweep_expired(lane_mut(&mut guard, lane_kind), now);
    }
    sweep_global_expired(&mut guard, now);

    let owner = guard
        .global_publish
        .owner
        .as_ref()
        .filter(|owner| owner.token == req.token && owner.kind == kind.as_str())
        .cloned();
    let Some(owner) = owner else {
        return Err(err(
            StatusCode::CONFLICT,
            "not-publish-owner",
            "only the current global publish owner may finish this lease",
        ));
    };

    let lane_mut_ref = lane_mut(&mut guard, kind);
    let pos = lane_mut_ref
        .in_flight
        .iter()
        .position(|b| b.token == req.token);
    let removed = pos.map(|i| lane_mut_ref.in_flight.remove(i));

    let recorded = if let Some(b) = removed {
        if let Err((error_kind, error_message)) = validate_finish_identity(&b, &req) {
            lane_mut(&mut guard, kind).in_flight.push(b);
            return Err(err(StatusCode::CONFLICT, error_kind, error_message));
        }
        let final_vn = b.assigned_version_name.clone();
        let final_vc = b.assigned_version_code;
        let final_sha = b.sha.clone();
        let last = LastRelease {
            success: req.success,
            sha: final_sha,
            version_name: final_vn.clone(),
            version_code: final_vc,
            finished_at: now,
            builder_label: b.builder_label,
            error_message: req.error_message.clone(),
        };
        lane_mut_ref.last_release = Some(last);

        if req.success {
            // 单调推进 last_published_*
            let prev_vn = lane_mut_ref.last_published_version_name.clone();
            let new_vn = match prev_vn {
                Some(p) => {
                    if semver_ge(&final_vn, &p) {
                        final_vn
                    } else {
                        p
                    }
                }
                None => final_vn,
            };
            lane_mut_ref.last_published_version_name = Some(new_vn);
            if let Some(c) = final_vc {
                let new_c = lane_mut_ref
                    .last_published_version_code
                    .map(|prev| prev.max(c))
                    .unwrap_or(c);
                lane_mut_ref.last_published_version_code = Some(new_c);
            }
        }
        crate::release_batch::record_stage(
            &mut guard,
            &owner.batch_id,
            &owner.sha,
            &owner.kind,
            &owner.stage,
            &owner.builder_id,
            &owner.builder_label,
            if req.success { "succeeded" } else { "failed" },
            owner.lease_expires_at,
            req.error_message.clone(),
            now,
        );
        finish_global_publish(
            &mut guard,
            &owner,
            req.success,
            req.error_message.clone(),
            now,
        );
        true
    } else {
        false
    };

    let resp = FinishResponse { ok: true, recorded };
    if let Err(error) = mgr.persist(&guard).await {
        *guard = original;
        return Err(persist_error(error));
    }
    drop(guard);
    Ok(Json(resp))
}

pub async fn status_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<StatusQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mgr = manager(&state);
    ensure_manager_healthy(&mgr)?;
    let mut guard = mgr.inner.lock().await;
    let original = guard.clone();
    let now = now_secs();
    sweep_expired(&mut guard.server, now);
    sweep_expired(&mut guard.apk, now);
    sweep_expired(&mut guard.node_agent, now);
    sweep_global_expired(&mut guard, now);

    let render = |lane: &LaneState, name: &str| -> Value {
        json!({
            "kind": name,
            "inFlight": lane.in_flight.iter().map(public_in_flight).collect::<Vec<_>>(),
            "lastRelease": lane.last_release,
            "lastPublishedVersionName": lane.last_published_version_name,
            "lastPublishedVersionCode": lane.last_published_version_code,
        })
    };

    let token_status = q
        .token
        .as_deref()
        .map(|token| publish_token_status(&guard, token));
    let global = json!({
        "owner": guard.global_publish.owner.as_ref().map(PublicPublishLeaseEntry::from),
        "waiters": guard.global_publish.waiters.iter().map(PublicPublishLeaseEntry::from).collect::<Vec<_>>(),
        "waiterCount": guard.global_publish.waiters.len(),
        "queuePolicy": "fifo",
        "coalescingKey": "kind+sha",
        "immutableReleaseSha": true,
        "batchIdentity": "batchId+sha",
    });
    let body = match q.kind.as_deref() {
        Some("server") => render(&guard.server, "server"),
        Some("apk") => render(&guard.apk, "apk"),
        Some("node_agent") => render(&guard.node_agent, "node_agent"),
        _ => json!({
            "server": render(&guard.server, "server"),
            "apk": render(&guard.apk, "apk"),
            "nodeAgent": render(&guard.node_agent, "node_agent"),
            "globalPublish": global,
            "tokenStatus": token_status,
            "releaseBatches": guard.release_batches,
            "stateHealth": "healthy",
            "now": now,
        }),
    };
    if let Err(error) = mgr.persist(&guard).await {
        *guard = original;
        return Err(persist_error(error));
    }
    drop(guard);
    Ok(Json(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build() -> InFlightBuilder {
        InFlightBuilder {
            token: "token".to_string(),
            builder_id: "builder".to_string(),
            builder_label: "builder".to_string(),
            sha: "fixed-sha".to_string(),
            batch_id: "release-fixed-sha".to_string(),
            stage: "android_apk".to_string(),
            assigned_version_name: "1.2.3".to_string(),
            assigned_version_code: Some(123),
            claimed_at: 1,
            last_heartbeat: 1,
            lease_expires_at: 100,
        }
    }

    fn finish(sha: &str, version_name: &str, version_code: i64) -> FinishRequest {
        FinishRequest {
            kind: "apk".to_string(),
            token: "token".to_string(),
            success: true,
            version_name: Some(version_name.to_string()),
            version_code: Some(version_code),
            sha: Some(sha.to_string()),
            error_message: None,
        }
    }

    #[test]
    fn finish_identity_is_immutable_after_claim() {
        assert!(validate_finish_identity(&build(), &finish("fixed-sha", "1.2.3", 123)).is_ok());
        assert_eq!(
            validate_finish_identity(&build(), &finish("other-sha", "1.2.3", 123))
                .unwrap_err()
                .0,
            "immutable-sha-mismatch"
        );
        assert_eq!(
            validate_finish_identity(&build(), &finish("fixed-sha", "1.2.4", 123))
                .unwrap_err()
                .0,
            "immutable-version-mismatch"
        );
        assert_eq!(
            validate_finish_identity(&build(), &finish("fixed-sha", "1.2.3", 124))
                .unwrap_err()
                .0,
            "immutable-version-code-mismatch"
        );
    }

    #[test]
    fn public_release_status_never_exposes_lease_tokens() {
        let build = build();
        let build_json = public_in_flight(&build);
        assert!(build_json.get("token").is_none());
        assert!(!build_json.to_string().contains("token"));

        let lease = PublishLeaseEntry {
            token: "secret-lease-token".to_string(),
            kind: "server".to_string(),
            sha: "fixed-sha".to_string(),
            batch_id: "release-fixed-sha".to_string(),
            stage: "server".to_string(),
            builder_id: "builder".to_string(),
            builder_label: "builder".to_string(),
            requested_at: 1,
            last_heartbeat: 2,
            lease_expires_at: 100,
        };
        let lease_json = serde_json::to_value(PublicPublishLeaseEntry::from(&lease))
            .expect("public lease should serialize");
        assert!(lease_json.get("token").is_none());
        assert!(!lease_json.to_string().contains("secret-lease-token"));
    }
}
