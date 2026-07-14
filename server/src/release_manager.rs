//! 发布状态持久化层：状态类型、ReleaseManager 和版本号计算工具。
//!
//! 被 `release_claim`（HTTP handlers）引用；不含路由或请求/响应类型。

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::types::AppState;

/// 最长 lease 时长（秒）。客户端要求再长也截到这里。
pub(crate) const MAX_LEASE_SECS: i64 = 3600; // 1 h

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
pub(crate) struct ReleaseStateFile {
    #[serde(default)]
    pub(crate) server: LaneState,
    #[serde(default)]
    pub(crate) apk: LaneState,
}

// ===== 全局 Manager =====

pub struct ReleaseManager {
    pub(crate) inner: Mutex<ReleaseStateFile>,
    path: PathBuf,
}

impl ReleaseManager {
    pub(crate) fn load_or_init(data_dir: &Path) -> Arc<Self> {
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

    pub(crate) async fn persist(&self, state: &ReleaseStateFile) {
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

pub(crate) static MANAGER: OnceLock<Arc<ReleaseManager>> = OnceLock::new();

pub(crate) fn manager(state: &AppState) -> Arc<ReleaseManager> {
    MANAGER
        .get_or_init(|| ReleaseManager::load_or_init(&state.data_dir))
        .clone()
}

// ===== 内部辅助 =====

#[derive(Copy, Clone, Debug)]
pub(crate) enum Lane {
    Server,
    Apk,
}

pub(crate) fn parse_kind(s: &str) -> Result<Lane, (StatusCode, Json<Value>)> {
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

pub(crate) fn lane(state: &ReleaseStateFile, k: Lane) -> &LaneState {
    match k {
        Lane::Server => &state.server,
        Lane::Apk => &state.apk,
    }
}

pub(crate) fn lane_mut(state: &mut ReleaseStateFile, k: Lane) -> &mut LaneState {
    match k {
        Lane::Server => &mut state.server,
        Lane::Apk => &mut state.apk,
    }
}

pub(crate) fn sweep_expired(lane: &mut LaneState, now: i64) {
    lane.in_flight.retain(|b| b.lease_expires_at > now);
}

pub(crate) fn clamp_lease(secs: i64) -> i64 {
    secs.max(60).min(MAX_LEASE_SECS)
}

pub(crate) fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub(crate) fn err(code: StatusCode, kind: &str, msg: &str) -> (StatusCode, Json<Value>) {
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

pub(crate) fn semver_ge(a: &str, b: &str) -> bool {
    match (parse_semver(a), parse_semver(b)) {
        (Some(x), Some(y)) => x >= y,
        _ => a >= b,
    }
}

pub(crate) fn max_semver(list: &[String]) -> Option<String> {
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

pub(crate) fn bump_semver(base: &str, kind: &str) -> String {
    let (a, b, c) = parse_semver(base).unwrap_or((0, 0, 0));
    match kind {
        "major" => format!("{}.0.0", a + 1),
        "minor" => format!("{}.{}.0", a, b + 1),
        _ => format!("{}.{}.{}", a, b, c + 1),
    }
}
