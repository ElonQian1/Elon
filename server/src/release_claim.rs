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

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::types::AppState;

// ===== 调参常量 =====

/// 默认 lease 时长（秒）。心跳一次刷新到这么长。
const DEFAULT_LEASE_SECS: i64 = 1800; // 30 min
/// 最长 lease 时长（秒）。客户端要求再长也截到这里。
const MAX_LEASE_SECS: i64 = 3600; // 1 h
/// `claim` 时如未给 `lease_secs`，按构建预估给的最低基线
const ESTIMATED_BUILD_SECS_SERVER: i64 = 300;
const ESTIMATED_BUILD_SECS_APK: i64 = 360;

// ===== 持久化结构 =====

/// 正在进行的一次构建（in_flight）。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InFlightBuilder {
    pub token: String,
    pub builder_id: String,
    pub builder_label: String,
    pub sha: String,
    /// 服务器分配给这次构建的版本号
    pub assigned_version_name: String,
    /// APK 通道使用；server 通道为 None
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_version_code: Option<i64>,
    pub claimed_at: i64,
    pub last_heartbeat: i64,
    pub lease_expires_at: i64,
}

/// 一次已完成发布的快照。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastRelease {
    pub success: bool,
    pub sha: String,
    pub version_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_code: Option<i64>,
    pub finished_at: i64,
    pub builder_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// 一个通道（server / apk）的状态。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneState {
    #[serde(default)]
    pub in_flight: Vec<InFlightBuilder>,
    #[serde(default)]
    pub last_release: Option<LastRelease>,
    /// 截至目前 finish(success=true) 中观察到的最大版本号。
    #[serde(default)]
    pub last_published_version_name: Option<String>,
    #[serde(default)]
    pub last_published_version_code: Option<i64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct ReleaseStateFile {
    #[serde(default)]
    server: LaneState,
    #[serde(default)]
    apk: LaneState,
}

// ===== 全局 Manager =====

pub struct ReleaseManager {
    inner: Mutex<ReleaseStateFile>,
    path: PathBuf,
}

impl ReleaseManager {
    fn load_or_init(data_dir: &Path) -> Arc<Self> {
        let path = data_dir.join("release-state.json");
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<ReleaseStateFile>(&s).ok())
            .unwrap_or_default();
        Arc::new(Self {
            inner: Mutex::new(state),
            path,
        })
    }

    async fn persist(&self, state: &ReleaseStateFile) {
        let json = match serde_json::to_string_pretty(state) {
            Ok(s) => s,
            Err(_) => return,
        };
        let tmp = self.path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &self.path);
        }
    }
}

static MANAGER: OnceLock<Arc<ReleaseManager>> = OnceLock::new();

fn manager(state: &AppState) -> Arc<ReleaseManager> {
    MANAGER
        .get_or_init(|| ReleaseManager::load_or_init(&state.data_dir))
        .clone()
}

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

// ===== 内部辅助 =====

#[derive(Copy, Clone, Debug)]
enum Lane {
    Server,
    Apk,
}

fn parse_kind(s: &str) -> Result<Lane, (StatusCode, Json<Value>)> {
    match s {
        "server" => Ok(Lane::Server),
        "apk" => Ok(Lane::Apk),
        other => Err(err(
            StatusCode::BAD_REQUEST,
            "bad-kind",
            &format!("unknown kind: {other}"),
        )),
    }
}

fn lane(state: &ReleaseStateFile, k: Lane) -> &LaneState {
    match k {
        Lane::Server => &state.server,
        Lane::Apk => &state.apk,
    }
}

fn lane_mut(state: &mut ReleaseStateFile, k: Lane) -> &mut LaneState {
    match k {
        Lane::Server => &mut state.server,
        Lane::Apk => &mut state.apk,
    }
}

fn sweep_expired(lane: &mut LaneState, now: i64) {
    lane.in_flight.retain(|b| b.lease_expires_at > now);
}

fn clamp_lease(secs: i64) -> i64 {
    secs.max(60).min(MAX_LEASE_SECS)
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn err(code: StatusCode, kind: &str, msg: &str) -> (StatusCode, Json<Value>) {
    (
        code,
        Json(json!({
            "error": kind,
            "message": msg,
        })),
    )
}

fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let mut parts = s.split('.');
    let a = parts.next()?.parse().ok()?;
    let b = parts.next()?.parse().ok()?;
    let c = parts.next()?.parse().ok()?;
    Some((a, b, c))
}

fn semver_ge(a: &str, b: &str) -> bool {
    match (parse_semver(a), parse_semver(b)) {
        (Some(x), Some(y)) => x >= y,
        _ => a >= b,
    }
}

fn max_semver(list: &[String]) -> Option<String> {
    let mut best: Option<(u32, u32, u32)> = None;
    let mut best_str: Option<String> = None;
    for s in list {
        if let Some(p) = parse_semver(s) {
            if best.map(|b| p > b).unwrap_or(true) {
                best = Some(p);
                best_str = Some(s.clone());
            }
        }
    }
    best_str
}

fn bump_semver(base: &str, kind: &str) -> String {
    let (a, b, c) = parse_semver(base).unwrap_or((0, 0, 0));
    match kind {
        "major" => format!("{}.0.0", a + 1),
        "minor" => format!("{}.{}.0", a, b + 1),
        _ => format!("{}.{}.{}", a, b, c + 1),
    }
}
