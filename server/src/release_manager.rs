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
use anyhow::{Context, Result};

/// 发布 lease 只证明进程仍有心跳；长构建靠续租，不靠一次占用数小时。
pub(crate) const MAX_LEASE_SECS: i64 = 180;

// ===== 持久化结构 =====

/// 正在进行的一次构建（in_flight）。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InFlightBuilder {
    pub token: String,
    pub builder_id: String,
    pub builder_label: String,
    pub sha: String,
    #[serde(default)]
    pub batch_id: String,
    #[serde(default)]
    pub stage: String,
    /// 服务器分配给这次构建的版本号
    pub assigned_version_name: String,
    /// APK 通道使用；server 通道为 None
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_version_code: Option<i64>,
    pub claimed_at: i64,
    pub last_heartbeat: i64,
    pub lease_expires_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishLeaseEntry {
    pub token: String,
    pub kind: String,
    pub sha: String,
    #[serde(default)]
    pub batch_id: String,
    #[serde(default)]
    pub stage: String,
    pub builder_id: String,
    pub builder_label: String,
    pub requested_at: i64,
    pub last_heartbeat: i64,
    pub lease_expires_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishCompletion {
    pub token: String,
    pub kind: String,
    pub sha: String,
    #[serde(default)]
    pub batch_id: String,
    #[serde(default)]
    pub stage: String,
    pub success: bool,
    pub coalesced: bool,
    pub finished_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalPublishState {
    /// 旧版单 owner 字段；首次状态操作时迁入 `owners`。
    #[serde(default)]
    pub owner: Option<PublishLeaseEntry>,
    /// 各发布通道可各自拥有一个 owner。
    #[serde(default)]
    pub owners: Vec<PublishLeaseEntry>,
    #[serde(default)]
    pub waiters: Vec<PublishLeaseEntry>,
    #[serde(default)]
    pub completed: Vec<PublishCompletion>,
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
    #[serde(default)]
    pub(crate) node_agent: LaneState,
    #[serde(default)]
    pub(crate) global_publish: GlobalPublishState,
    #[serde(default)]
    pub(crate) release_batches: Vec<crate::release_batch::ReleaseBatchLedger>,
}

// ===== 全局 Manager =====

pub struct ReleaseManager {
    pub(crate) inner: Mutex<ReleaseStateFile>,
    path: PathBuf,
    load_error: Option<String>,
}

impl ReleaseManager {
    pub(crate) fn load_or_init(data_dir: &Path) -> Arc<Self> {
        let path = data_dir.join("release-state.json");
        let (state, load_error) = if path.exists() {
            match std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))
                .and_then(|text| {
                    serde_json::from_str::<ReleaseStateFile>(&text)
                        .with_context(|| format!("parse {}", path.display()))
                }) {
                Ok(state) => (state, None),
                Err(error) => (ReleaseStateFile::default(), Some(error.to_string())),
            }
        } else {
            (ReleaseStateFile::default(), None)
        };
        Arc::new(Self {
            inner: Mutex::new(state),
            path,
            load_error,
        })
    }

    pub(crate) fn health_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    pub(crate) async fn persist(&self, state: &ReleaseStateFile) -> Result<()> {
        if let Some(error) = self.load_error.as_deref() {
            anyhow::bail!("release state failed closed: {error}");
        }
        let json = serde_json::to_string_pretty(state).context("serialize release state")?;
        crate::node_agent_atomic_file::write(&self.path, json.as_bytes())
            .with_context(|| format!("atomically replace {}", self.path.display()))?;
        Ok(())
    }
}

pub(crate) static MANAGER: OnceLock<Arc<ReleaseManager>> = OnceLock::new();

pub(crate) fn manager(state: &AppState) -> Arc<ReleaseManager> {
    MANAGER
        .get_or_init(|| ReleaseManager::load_or_init(&state.data_dir))
        .clone()
}

// ===== 内部辅助 =====

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum Lane {
    Server,
    Apk,
    NodeAgent,
}

impl Lane {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::Apk => "apk",
            Self::NodeAgent => "node_agent",
        }
    }
}

pub(crate) fn parse_kind(s: &str) -> Result<Lane, (StatusCode, Json<Value>)> {
    match s {
        "server" => Ok(Lane::Server),
        "apk" => Ok(Lane::Apk),
        "node_agent" => Ok(Lane::NodeAgent),
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
        Lane::NodeAgent => &state.node_agent,
    }
}

pub(crate) fn lane_mut(state: &mut ReleaseStateFile, k: Lane) -> &mut LaneState {
    match k {
        Lane::Server => &mut state.server,
        Lane::Apk => &mut state.apk,
        Lane::NodeAgent => &mut state.node_agent,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_lease_is_short_and_heartbeat_renewable() {
        assert_eq!(clamp_lease(14_400), 180);
        assert_eq!(clamp_lease(30), 60);
    }

    #[test]
    fn corrupt_release_state_fails_closed_instead_of_resetting() {
        let root = std::env::temp_dir().join(format!(
            "elon-release-corrupt-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("release-state.json"), b"{broken").unwrap();

        let manager = ReleaseManager::load_or_init(&root);

        assert!(manager.health_error().is_some());
        let state = manager.inner.try_lock().unwrap();
        assert!(state.global_publish.owner.is_none());
        assert!(state.global_publish.owners.is_empty());
    }
}
