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
pub(crate) const MAX_LEASE_SECS: i64 = 14_400; // 4 h; node-agent cross builds can be lengthy

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishLeaseEntry {
    pub token: String,
    pub kind: String,
    pub sha: String,
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
    pub success: bool,
    pub coalesced: bool,
    pub finished_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalPublishState {
    #[serde(default)]
    pub owner: Option<PublishLeaseEntry>,
    #[serde(default)]
    pub waiters: Vec<PublishLeaseEntry>,
    #[serde(default)]
    pub completed: Vec<PublishCompletion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PublishAdmission {
    Owner,
    Waiter { queue_position: usize },
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

pub(crate) fn sweep_global_expired(state: &mut ReleaseStateFile, now: i64) {
    let owner_expired = state
        .global_publish
        .owner
        .as_ref()
        .is_some_and(|owner| owner.lease_expires_at <= now);
    if owner_expired {
        if let Some(owner) = state.global_publish.owner.take() {
            remove_publish_token_from_lanes(state, &owner.token);
            state.global_publish.completed.push(PublishCompletion {
                token: owner.token,
                kind: owner.kind,
                sha: owner.sha,
                success: false,
                coalesced: false,
                finished_at: now,
                error_message: Some("publish owner lease expired".to_string()),
            });
        }
    }
    let expired_waiters = state
        .global_publish
        .waiters
        .iter()
        .filter(|waiter| waiter.lease_expires_at <= now)
        .map(|waiter| waiter.token.clone())
        .collect::<Vec<_>>();
    state
        .global_publish
        .waiters
        .retain(|waiter| waiter.lease_expires_at > now);
    for token in expired_waiters {
        remove_publish_token_from_lanes(state, &token);
    }
    if state.global_publish.owner.is_none() && !state.global_publish.waiters.is_empty() {
        state.global_publish.owner = Some(state.global_publish.waiters.remove(0));
    }
    if state.global_publish.completed.len() > 200 {
        let keep_from = state.global_publish.completed.len() - 200;
        state.global_publish.completed.drain(..keep_from);
    }
}

pub(crate) fn enqueue_global_publish(
    state: &mut ReleaseStateFile,
    entry: PublishLeaseEntry,
) -> PublishAdmission {
    if state.global_publish.owner.is_none() {
        state.global_publish.owner = Some(entry);
        PublishAdmission::Owner
    } else {
        state.global_publish.waiters.push(entry);
        PublishAdmission::Waiter {
            queue_position: state.global_publish.waiters.len(),
        }
    }
}

pub(crate) fn finish_global_publish(
    state: &mut ReleaseStateFile,
    owner: &PublishLeaseEntry,
    success: bool,
    error_message: Option<String>,
    now: i64,
) -> Vec<String> {
    state.global_publish.owner = None;
    state.global_publish.completed.push(PublishCompletion {
        token: owner.token.clone(),
        kind: owner.kind.clone(),
        sha: owner.sha.clone(),
        success,
        coalesced: false,
        finished_at: now,
        error_message,
    });

    let mut coalesced_tokens = Vec::new();
    if success {
        state.global_publish.waiters.retain(|waiter| {
            let same_release = waiter.kind == owner.kind && waiter.sha == owner.sha;
            if same_release {
                coalesced_tokens.push(waiter.token.clone());
            }
            !same_release
        });
        for token in &coalesced_tokens {
            remove_publish_token_from_lanes(state, token);
            state.global_publish.completed.push(PublishCompletion {
                token: token.clone(),
                kind: owner.kind.clone(),
                sha: owner.sha.clone(),
                success: true,
                coalesced: true,
                finished_at: now,
                error_message: None,
            });
        }
    }
    if state.global_publish.owner.is_none() && !state.global_publish.waiters.is_empty() {
        state.global_publish.owner = Some(state.global_publish.waiters.remove(0));
    }
    coalesced_tokens
}

fn remove_publish_token_from_lanes(state: &mut ReleaseStateFile, token: &str) {
    for kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        lane_mut(state, kind)
            .in_flight
            .retain(|item| item.token != token);
    }
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
    use std::sync::{Arc, Mutex as StdMutex};

    fn lease(token: &str, kind: &str, sha: &str, expires: i64) -> PublishLeaseEntry {
        PublishLeaseEntry {
            token: token.to_string(),
            kind: kind.to_string(),
            sha: sha.to_string(),
            builder_id: token.to_string(),
            builder_label: token.to_string(),
            requested_at: 1,
            last_heartbeat: 1,
            lease_expires_at: expires,
        }
    }

    #[test]
    fn global_publish_is_fifo_under_concurrent_claims() {
        let state = Arc::new(StdMutex::new(ReleaseStateFile::default()));
        let mut joins = Vec::new();
        for index in 0..8 {
            let state = state.clone();
            joins.push(std::thread::spawn(move || {
                let mut guard = state.lock().expect("publish test state");
                enqueue_global_publish(
                    &mut guard,
                    lease(
                        &format!("token-{index}"),
                        "server",
                        &format!("sha-{index}"),
                        100,
                    ),
                )
            }));
        }
        for join in joins {
            join.join().expect("claim thread");
        }
        let guard = state.lock().expect("publish test state");
        assert!(guard.global_publish.owner.is_some());
        assert_eq!(guard.global_publish.waiters.len(), 7);
        let requested = guard
            .global_publish
            .owner
            .iter()
            .chain(guard.global_publish.waiters.iter())
            .map(|item| item.token.clone())
            .collect::<Vec<_>>();
        assert_eq!(requested.len(), 8);
        let unique = requested.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(unique.len(), 8);
    }

    #[test]
    fn same_sha_waiters_coalesce_and_next_distinct_sha_is_promoted() {
        let mut state = ReleaseStateFile::default();
        let owner = lease("owner", "node_agent", "sha-a", 100);
        assert_eq!(
            enqueue_global_publish(&mut state, owner.clone()),
            PublishAdmission::Owner
        );
        enqueue_global_publish(&mut state, lease("same", "node_agent", "sha-a", 100));
        enqueue_global_publish(&mut state, lease("next", "apk", "sha-b", 100));

        let coalesced = finish_global_publish(&mut state, &owner, true, None, 10);

        assert_eq!(coalesced, vec!["same"]);
        assert_eq!(state.global_publish.owner.as_ref().unwrap().token, "next");
        assert!(state
            .global_publish
            .completed
            .iter()
            .any(|item| item.token == "same" && item.coalesced));
    }

    #[test]
    fn expired_owner_yields_to_first_live_waiter() {
        let mut state = ReleaseStateFile::default();
        state.global_publish.owner = Some(lease("expired", "server", "sha-a", 10));
        state.global_publish.waiters = vec![
            lease("first", "apk", "sha-b", 100),
            lease("second", "node_agent", "sha-c", 100),
        ];

        sweep_global_expired(&mut state, 11);

        assert_eq!(state.global_publish.owner.as_ref().unwrap().token, "first");
        assert_eq!(state.global_publish.waiters[0].token, "second");
    }
}
