//! 发布抢占锁（self-claim release lock）
//!
//! 解决多台开发机 / 多个 AI 并发发版时的竞态死循环：
//!   - 推码者一旦推到 main，先向服务器 POST /api/release/claim 抢占发版权
//!   - 若没人在编译，服务器原子返回 `build` 指令 + 服务器生成的下一版本号 + 租约 token
//!   - 若有人在编译，服务器返回 `queue` 指令，告知当前 builder、已用时间、预计剩余
//!   - builder 每 5 分钟用 token 心跳一次刷新租约（默认 30 分钟 TTL）
//!   - builder 完成（成功/失败）后 POST /api/release/finish 释放锁并写入新版本号
//!   - 租约过期自动视为 idle，下一个 claim 直接接管
//!
//! 状态持久化到 `{data_dir}/release-state.json`，进程内用 tokio Mutex 串行化访问。
//! Server 通道（kind="server"）与 APK 通道（kind="apk"）互不阻塞，可并行发版。
//!
//! 版本号策略（v1）：
//!   - 服务器持久化最近一次发版的 versionName / versionCode
//!   - 启动时若状态文件不存在，从环境（自身 CARGO_PKG_VERSION + data_dir/app/version.json）冷启动
//!   - claim 默认自动 PATCH +1，可通过 `bump` 参数选择 minor / major / none
//!   - APK 通道每次 versionCode +1，versionName 按 bump 策略

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::types::AppState;

const STATE_FILE: &str = "release-state.json";
const DEFAULT_LEASE_SECS: i64 = 30 * 60;
const MAX_LEASE_SECS: i64 = 60 * 60;
const ESTIMATED_BUILD_SECS_SERVER: i64 = 5 * 60;
const ESTIMATED_BUILD_SECS_APK: i64 = 6 * 60;

// ───────────────────────────── 状态结构 ─────────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct LaneState {
    /// 当前活动 token；None 表示通道空闲
    active_token: Option<String>,
    /// 当前 builder 机器标识（uuid / mac / 主机名，用于程序对比）
    builder_id: Option<String>,
    /// 当前 builder 人类可读名（用于排队提示）
    builder_label: Option<String>,
    /// 正在 build 的提交 SHA
    sha: Option<String>,
    /// 服务器分配给当前 builder 的目标版本号（如 0.3.63）
    next_version_name: Option<String>,
    /// 服务器分配给当前 builder 的 versionCode（仅 APK）
    next_version_code: Option<i64>,
    /// 抢锁时间（unix 秒）
    claimed_at: Option<i64>,
    /// 上次心跳时间
    last_heartbeat: Option<i64>,
    /// 租约到期时间
    lease_expires_at: Option<i64>,
    /// 上一次完成的发版记录
    last_release: Option<LastRelease>,
    /// 已确认发版的最高 versionName（用于下次自动递增）
    last_published_version_name: Option<String>,
    /// 已确认发版的最高 versionCode（仅 APK）
    last_published_version_code: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LastRelease {
    success: bool,
    sha: String,
    version_name: Option<String>,
    version_code: Option<i64>,
    finished_at: i64,
    builder_label: Option<String>,
    error_message: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct ReleaseStateFile {
    server: LaneState,
    apk: LaneState,
}

impl LaneState {
    fn is_busy(&self, now: i64) -> bool {
        match (self.active_token.as_ref(), self.lease_expires_at) {
            (Some(_), Some(expires)) => expires > now,
            _ => false,
        }
    }

    fn clear(&mut self) {
        self.active_token = None;
        self.builder_id = None;
        self.builder_label = None;
        self.sha = None;
        self.next_version_name = None;
        self.next_version_code = None;
        self.claimed_at = None;
        self.last_heartbeat = None;
        self.lease_expires_at = None;
    }
}

// ───────────────────────────── Manager ─────────────────────────────

pub struct ReleaseManager {
    inner: Mutex<ReleaseStateFile>,
    path: PathBuf,
}

impl ReleaseManager {
    fn load_or_init(data_dir: &Path) -> Self {
        let path = data_dir.join(STATE_FILE);
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<ReleaseStateFile>(&s).ok())
            .unwrap_or_default();
        Self {
            inner: Mutex::new(state),
            path,
        }
    }

    fn persist(&self, state: &ReleaseStateFile) {
        let _ = (|| -> std::io::Result<()> {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = serde_json::to_string_pretty(state)?;
            let tmp = self.path.with_extension("json.tmp");
            std::fs::write(&tmp, content)?;
            std::fs::rename(&tmp, &self.path)?;
            Ok(())
        })()
        .inspect_err(|e| tracing::warn!("release state persist failed: {}", e));
    }
}

static MANAGER: OnceLock<Arc<ReleaseManager>> = OnceLock::new();

fn manager(state: &AppState) -> Arc<ReleaseManager> {
    MANAGER
        .get_or_init(|| Arc::new(ReleaseManager::load_or_init(&state.data_dir)))
        .clone()
}

// ───────────────────────────── 请求/响应 ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ClaimRequest {
    /// "server" | "apk"
    pub kind: String,
    pub sha: String,
    pub builder_id: String,
    #[serde(default)]
    pub builder_label: Option<String>,
    /// "patch" | "minor" | "major" | "none"，默认 patch
    #[serde(default)]
    pub bump: Option<String>,
    /// 客户端期望的租约秒数（最大 MAX_LEASE_SECS）
    #[serde(default)]
    pub lease_secs: Option<i64>,
    /// 客户端报告的当前 main 上版本号（用于 fall-back 计算 next_version）
    #[serde(default)]
    pub current_version_name: Option<String>,
    /// 同上，APK versionCode
    #[serde(default)]
    pub current_version_code: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum ClaimResponse {
    Build {
        token: String,
        kind: String,
        sha: String,
        next_version_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        next_version_code: Option<i64>,
        lease_expires_at: i64,
        claimed_at: i64,
    },
    Queue {
        kind: String,
        current_builder_label: Option<String>,
        current_sha: Option<String>,
        claimed_at: Option<i64>,
        lease_expires_at: Option<i64>,
        elapsed_secs: i64,
        estimated_remaining_secs: i64,
        last_heartbeat_age_secs: Option<i64>,
    },
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub kind: String,
    pub token: String,
    #[serde(default)]
    pub lease_secs: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    pub ok: bool,
    pub lease_expires_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct FinishRequest {
    pub kind: String,
    pub token: String,
    pub success: bool,
    pub sha: String,
    #[serde(default)]
    pub version_name: Option<String>,
    #[serde(default)]
    pub version_code: Option<i64>,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FinishResponse {
    pub ok: bool,
}

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LaneStatus {
    pub kind: String,
    pub busy: bool,
    pub current_builder_label: Option<String>,
    pub current_sha: Option<String>,
    pub claimed_at: Option<i64>,
    pub lease_expires_at: Option<i64>,
    pub last_heartbeat: Option<i64>,
    pub last_release: Option<LastRelease>,
    pub last_published_version_name: Option<String>,
    pub last_published_version_code: Option<i64>,
}

// ───────────────────────────── handlers ─────────────────────────────

fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": msg })))
}

fn parse_kind(raw: &str) -> Result<&'static str, (StatusCode, Json<serde_json::Value>)> {
    match raw {
        "server" => Ok("server"),
        "apk" => Ok("apk"),
        _ => Err(err(StatusCode::BAD_REQUEST, "kind must be 'server' or 'apk'")),
    }
}

fn lane_mut<'a>(state: &'a mut ReleaseStateFile, kind: &str) -> &'a mut LaneState {
    match kind {
        "server" => &mut state.server,
        _ => &mut state.apk,
    }
}

fn lane<'a>(state: &'a ReleaseStateFile, kind: &str) -> &'a LaneState {
    match kind {
        "server" => &state.server,
        _ => &state.apk,
    }
}

/// 解析 semver-ish 字符串 (X.Y.Z) → (X, Y, Z)。失败返回 None。
fn parse_semver(s: &str) -> Option<(u64, u64, u64)> {
    let mut parts = s.trim().split('.');
    let x: u64 = parts.next()?.parse().ok()?;
    let y: u64 = parts.next()?.parse().ok()?;
    let z: u64 = parts.next()?.parse().ok()?;
    Some((x, y, z))
}

fn bump_semver(s: &str, mode: &str) -> String {
    let (x, y, z) = parse_semver(s).unwrap_or((0, 0, 0));
    let (nx, ny, nz) = match mode {
        "major" => (x + 1, 0, 0),
        "minor" => (x, y + 1, 0),
        "none" => (x, y, z),
        _ => (x, y, z + 1), // patch (默认)
    };
    format!("{}.{}.{}", nx, ny, nz)
}

/// 比较 semver，返回较大值
fn max_semver<'a>(a: &'a str, b: &'a str) -> &'a str {
    match (parse_semver(a), parse_semver(b)) {
        (Some(va), Some(vb)) => {
            if va >= vb {
                a
            } else {
                b
            }
        }
        (Some(_), None) => a,
        (None, Some(_)) => b,
        (None, None) => a,
    }
}

pub async fn claim_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<ClaimResponse>, (StatusCode, Json<serde_json::Value>)> {
    let kind = parse_kind(&req.kind)?;
    let bump = req.bump.as_deref().unwrap_or("patch").to_string();
    let lease_secs = req
        .lease_secs
        .unwrap_or(DEFAULT_LEASE_SECS)
        .clamp(60, MAX_LEASE_SECS);

    let mgr = manager(&state);
    let mut guard = mgr.inner.lock().await;
    let now = now_secs();

    // TTL 检查：如果租约已过期，自动释放
    {
        let l = lane_mut(&mut guard, kind);
        if let (Some(_), Some(exp)) = (l.active_token.as_ref(), l.lease_expires_at) {
            if exp <= now {
                tracing::info!(
                    "release lane '{}' lease expired (builder={:?} sha={:?}), auto-releasing",
                    kind,
                    l.builder_label,
                    l.sha
                );
                l.clear();
            }
        }
    }

    let lane_ref = lane(&guard, kind);
    if lane_ref.is_busy(now) {
        let claimed_at = lane_ref.claimed_at;
        let elapsed = claimed_at.map(|c| (now - c).max(0)).unwrap_or(0);
        let est = match kind {
            "server" => ESTIMATED_BUILD_SECS_SERVER,
            _ => ESTIMATED_BUILD_SECS_APK,
        };
        let remaining = (est - elapsed).max(0);
        let hb_age = lane_ref.last_heartbeat.map(|h| (now - h).max(0));
        return Ok(Json(ClaimResponse::Queue {
            kind: kind.to_string(),
            current_builder_label: lane_ref.builder_label.clone(),
            current_sha: lane_ref.sha.clone(),
            claimed_at,
            lease_expires_at: lane_ref.lease_expires_at,
            elapsed_secs: elapsed,
            estimated_remaining_secs: remaining,
            last_heartbeat_age_secs: hb_age,
        }));
    }

    // 通道空闲 → 分配版本号 + 写入 lock
    let token = uuid::Uuid::new_v4().to_string();
    let expires = now + lease_secs;

    let next_version_name: String;
    let next_version_code: Option<i64>;
    {
        let l = lane_mut(&mut guard, kind);

        // 基准 versionName：max(last_published, current_reported)
        let base_name = match (
            l.last_published_version_name.as_deref(),
            req.current_version_name.as_deref(),
        ) {
            (Some(a), Some(b)) => max_semver(a, b).to_string(),
            (Some(a), None) => a.to_string(),
            (None, Some(b)) => b.to_string(),
            (None, None) => "0.0.0".to_string(),
        };
        next_version_name = bump_semver(&base_name, &bump);

        // versionCode 仅 APK
        next_version_code = if kind == "apk" {
            let base_code = std::cmp::max(
                l.last_published_version_code.unwrap_or(0),
                req.current_version_code.unwrap_or(0),
            );
            Some(base_code + 1)
        } else {
            None
        };

        l.active_token = Some(token.clone());
        l.builder_id = Some(req.builder_id.clone());
        l.builder_label = req.builder_label.clone();
        l.sha = Some(req.sha.clone());
        l.next_version_name = Some(next_version_name.clone());
        l.next_version_code = next_version_code;
        l.claimed_at = Some(now);
        l.last_heartbeat = Some(now);
        l.lease_expires_at = Some(expires);
    }

    mgr.persist(&guard);

    tracing::info!(
        "release lane '{}' CLAIMED by {} (sha={}, next={}, code={:?}, token={})",
        kind,
        req.builder_label.as_deref().unwrap_or(&req.builder_id),
        &req.sha,
        &next_version_name,
        next_version_code,
        &token,
    );

    Ok(Json(ClaimResponse::Build {
        token,
        kind: kind.to_string(),
        sha: req.sha,
        next_version_name,
        next_version_code,
        lease_expires_at: expires,
        claimed_at: now,
    }))
}

pub async fn heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, (StatusCode, Json<serde_json::Value>)> {
    let kind = parse_kind(&req.kind)?;
    let lease_secs = req
        .lease_secs
        .unwrap_or(DEFAULT_LEASE_SECS)
        .clamp(60, MAX_LEASE_SECS);

    let mgr = manager(&state);
    let mut guard = mgr.inner.lock().await;
    let now = now_secs();
    let expires;
    {
        let l = lane_mut(&mut guard, kind);
        match l.active_token.as_deref() {
            Some(tok) if tok == req.token => {
                expires = now + lease_secs;
                l.last_heartbeat = Some(now);
                l.lease_expires_at = Some(expires);
            }
            _ => {
                return Err(err(
                    StatusCode::GONE,
                    "claim token invalid or lease released",
                ));
            }
        }
    }
    mgr.persist(&guard);
    Ok(Json(HeartbeatResponse {
        ok: true,
        lease_expires_at: expires,
    }))
}

pub async fn finish_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FinishRequest>,
) -> Result<Json<FinishResponse>, (StatusCode, Json<serde_json::Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    let mut guard = mgr.inner.lock().await;
    let now = now_secs();
    {
        let l = lane_mut(&mut guard, kind);
        match l.active_token.as_deref() {
            Some(tok) if tok == req.token => {
                let release = LastRelease {
                    success: req.success,
                    sha: req.sha.clone(),
                    version_name: req.version_name.clone(),
                    version_code: req.version_code,
                    finished_at: now,
                    builder_label: l.builder_label.clone(),
                    error_message: req.error_message.clone(),
                };
                // 成功时记入最高已发版版本
                if req.success {
                    if let Some(vn) = req.version_name.as_deref() {
                        let new_max = match l.last_published_version_name.as_deref() {
                            Some(prev) => max_semver(prev, vn).to_string(),
                            None => vn.to_string(),
                        };
                        l.last_published_version_name = Some(new_max);
                    }
                    if kind == "apk" {
                        if let Some(vc) = req.version_code {
                            let prev = l.last_published_version_code.unwrap_or(0);
                            l.last_published_version_code = Some(std::cmp::max(prev, vc));
                        }
                    }
                }
                l.clear();
                l.last_release = Some(release);
            }
            _ => {
                return Err(err(
                    StatusCode::GONE,
                    "claim token invalid or already released",
                ));
            }
        }
    }
    mgr.persist(&guard);
    tracing::info!(
        "release lane '{}' FINISHED (success={}, sha={}, version={:?}, code={:?})",
        kind,
        req.success,
        req.sha,
        req.version_name,
        req.version_code,
    );
    Ok(Json(FinishResponse { ok: true }))
}

pub async fn status_handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<StatusQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mgr = manager(&state);
    let guard = mgr.inner.lock().await;
    let now = now_secs();

    fn snapshot(kind: &str, l: &LaneState, now: i64) -> LaneStatus {
        LaneStatus {
            kind: kind.to_string(),
            busy: l.is_busy(now),
            current_builder_label: l.builder_label.clone(),
            current_sha: l.sha.clone(),
            claimed_at: l.claimed_at,
            lease_expires_at: l.lease_expires_at,
            last_heartbeat: l.last_heartbeat,
            last_release: l.last_release.clone(),
            last_published_version_name: l.last_published_version_name.clone(),
            last_published_version_code: l.last_published_version_code,
        }
    }

    if let Some(k) = q.kind.as_deref() {
        let kind = parse_kind(k)?;
        let snap = snapshot(kind, lane(&guard, kind), now);
        return Ok(Json(serde_json::to_value(snap).unwrap_or_default()));
    }
    let snap = serde_json::json!({
        "server": snapshot("server", &guard.server, now),
        "apk": snapshot("apk", &guard.apk, now),
        "now": now,
    });
    Ok(Json(snap))
}
