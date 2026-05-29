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
    bump_semver, clamp_lease, err, lane, lane_mut, manager, max_semver, now_secs, parse_kind,
    semver_ge, sweep_expired, InFlightBuilder, Lane, LaneState, LastRelease,
};
use crate::types::AppState;

// ===== 调参常量 =====

/// 默认 lease 时长（秒）。心跳一次刷新到这么长。
const DEFAULT_LEASE_SECS: i64 = 1800; // 30 min
/// `claim` 时如未给 `lease_secs`，按构建预估给的最低基线
const ESTIMATED_BUILD_SECS_SERVER: i64 = 300;
const ESTIMATED_BUILD_SECS_APK: i64 = 360;

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
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClaimResponse {
    pub action: &'static str, // 永远是 "build"
    pub kind: String,
    pub token: String,
    pub sha: String,
    pub assigned_version_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_version_code: Option<i64>,
    pub claimed_at: i64,
    pub lease_expires_at: i64,
    pub in_flight_count: usize,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatRequest {
    pub kind: String,
    pub token: String,
    pub lease_secs: Option<i64>,
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
}

// ===== Handlers =====

pub async fn claim_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<ClaimResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    let mut guard = mgr.inner.lock().await;
    let now = now_secs();

    sweep_expired(lane_mut(&mut guard, kind), now);

    let lease = clamp_lease(req.lease_secs.unwrap_or(match kind {
        Lane::Server => ESTIMATED_BUILD_SECS_SERVER,
        Lane::Apk => ESTIMATED_BUILD_SECS_APK,
    }));

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
    let assigned_version_name = bump_semver(&base_name, bump_kind);

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
        assigned_version_name: assigned_version_name.clone(),
        assigned_version_code,
        claimed_at: now,
        last_heartbeat: now,
        lease_expires_at,
    };

    let lane_mut_ref = lane_mut(&mut guard, kind);
    lane_mut_ref.in_flight.push(entry);
    let in_flight_count = lane_mut_ref.in_flight.len();

    let resp = ClaimResponse {
        action: "build",
        kind: req.kind,
        token,
        sha: req.sha,
        assigned_version_name,
        assigned_version_code,
        claimed_at: now,
        lease_expires_at,
        in_flight_count,
    };

    mgr.persist(&guard).await;
    drop(guard);
    Ok(Json(resp))
}

pub async fn heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    let mut guard = mgr.inner.lock().await;
    let now = now_secs();

    sweep_expired(lane_mut(&mut guard, kind), now);

    let lease = clamp_lease(req.lease_secs.unwrap_or(DEFAULT_LEASE_SECS));
    let lane_mut_ref = lane_mut(&mut guard, kind);
    let item = lane_mut_ref
        .in_flight
        .iter_mut()
        .find(|b| b.token == req.token);
    let Some(b) = item else {
        return Err(err(
            StatusCode::GONE,
            "token-not-active",
            "lease expired or unknown token",
        ));
    };
    b.last_heartbeat = now;
    b.lease_expires_at = now + lease;
    let lease_expires_at = b.lease_expires_at;

    let resp = HeartbeatResponse {
        ok: true,
        lease_expires_at,
    };
    mgr.persist(&guard).await;
    drop(guard);
    Ok(Json(resp))
}

pub async fn finish_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FinishRequest>,
) -> Result<Json<FinishResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    let mut guard = mgr.inner.lock().await;
    let now = now_secs();

    let lane_mut_ref = lane_mut(&mut guard, kind);
    let pos = lane_mut_ref
        .in_flight
        .iter()
        .position(|b| b.token == req.token);
    let removed = pos.map(|i| lane_mut_ref.in_flight.remove(i));

    let recorded = if let Some(b) = removed {
        let final_vn = req.version_name.clone().unwrap_or(b.assigned_version_name);
        let final_vc = req.version_code.or(b.assigned_version_code);
        let final_sha = req.sha.clone().unwrap_or(b.sha);
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
        true
    } else {
        false
    };

    let resp = FinishResponse { ok: true, recorded };
    mgr.persist(&guard).await;
    drop(guard);
    Ok(Json(resp))
}

pub async fn status_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<StatusQuery>,
) -> Json<Value> {
    let mgr = manager(&state);
    let mut guard = mgr.inner.lock().await;
    let now = now_secs();
    sweep_expired(&mut guard.server, now);
    sweep_expired(&mut guard.apk, now);
    // 不需要 persist：sweep 只是清掉过期 in_flight，下一次 claim 会持久化

    let render = |lane: &LaneState, name: &str| -> Value {
        json!({
            "kind": name,
            "inFlight": lane.in_flight,
            "lastRelease": lane.last_release,
            "lastPublishedVersionName": lane.last_published_version_name,
            "lastPublishedVersionCode": lane.last_published_version_code,
        })
    };

    let body = match q.kind.as_deref() {
        Some("server") => render(&guard.server, "server"),
        Some("apk") => render(&guard.apk, "apk"),
        _ => json!({
            "server": render(&guard.server, "server"),
            "apk": render(&guard.apk, "apk"),
            "now": now,
        }),
    };
    drop(guard);
    Json(body)
}