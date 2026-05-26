//! 局域网 PC 种子节点模块
//!
//! 工作流：
//!   1. 开发 PC 在 publish-*.ps1 发布完成后，调用 lan-dist-client.ps1 注册产物
//!   2. lan-dist-client.ps1 写注册文件到 %TEMP%\lan-dist-registry\，
//!      并启动（或复用）共享后台守护进程 lan-dist-daemon
//!   3. 守护进程在本机启动 HTTP 服务（端口 7788），并向服务器 POST /app/lan-peer/register
//!      注册自身 LAN IP + port + dist_path（如 "/dist/elon/user-apk"）
//!   4. GET /app/version.json（在 peer_relay.rs 中）动态注入 LAN peer mirrors，priority=10
//!      Mirror URL 为 http://<lan_ip>:7788<dist_path>（可精确指向具体项目/产物）
//!   5. 手机 APK 收到 version.json 后，优先尝试 LAN 直连下载（4s 超时）
//!   6. LAN 连接失败 → 自动回落到手机P2P中继或服务器直链
//!
//! 多产物支持：守护进程单进程服务所有项目所有产物，按 URL 路径区分：
//!   - /dist/elon/user-apk     — elon 用户端 APK
//!   - /dist/elon/admin-apk    — elon 管理端 APK
//!   - /dist/bb64a/user-apk    — bb64a 用户端 APK
//!   - /dist/bb64a/windows-exe — bb64a Windows 客户端
//!
//! 过期策略：注册后 2 小时自动过期（注入 version.json 时过滤）

use std::{sync::Arc, time::Duration};

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::types::{AppState, LanPeerEntry};

// ─── 注册请求/响应 ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RegisterLanPeerRequest {
    /// PC 在局域网中的 IP 地址（如 192.168.1.100）
    pub lan_ip: String,
    /// PC 上 HTTP 文件服务器的端口（建议固定为 7788）
    pub port: u16,
    /// 该 PC 发布的产物版本号（APK versionCode 或其他整数版本）
    pub version_code: i64,
    /// 产物在本地 HTTP 服务器上的路径，如 "/dist/elon/user-apk"
    /// 不传则向后兼容，默认 "/apk"
    pub dist_path: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterLanPeerResponse {
    pub peer_id: String,
    /// 该注册的有效期（秒）
    pub expires_in: u64,
}

/// POST /app/lan-peer/register
/// 开发 PC 向服务器注册其局域网地址，服务器将其注入到 version.json mirrors 中。
pub async fn register_lan_peer(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterLanPeerRequest>,
) -> impl IntoResponse {
    // 基本校验
    if body.lan_ip.is_empty() || body.port == 0 || body.version_code <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "lan_ip、port、version_code 均为必填且需有效"})),
        )
            .into_response();
    }

    // 过滤 IP 格式（只接受合理的私有 IP）
    if !is_valid_lan_ip(&body.lan_ip) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "lan_ip 必须是合法的私有 IPv4 地址"})),
        )
            .into_response();
    }

    // 用 "lan-{ip}-{port}" 作为 peer_id，同一台 PC 重复注册会更新而非新增
    let peer_id = format!("lan-{}-{}", body.lan_ip.replace('.', "-"), body.port);

    let dist_path = body
        .dist_path
        .clone()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "/apk".to_string());

    let entry = LanPeerEntry {
        lan_ip: body.lan_ip.clone(),
        port: body.port,
        version_code: body.version_code,
        dist_path: dist_path.clone(),
        registered_at: std::time::Instant::now(),
    };

    {
        let mut reg = state.lan_peer_registry.write().await;
        // 清理已过期条目（顺手做，不用单独后台任务）
        reg.retain(|_, e| e.registered_at.elapsed() < LAN_PEER_TTL);
        reg.insert(peer_id.clone(), entry);
    }

    tracing::info!(
        "🖥️  LAN PC 种子注册: {} ({}:{}{}, versionCode={})",
        peer_id,
        body.lan_ip,
        body.port,
        dist_path,
        body.version_code
    );

    (
        StatusCode::OK,
        Json(RegisterLanPeerResponse {
            peer_id,
            expires_in: LAN_PEER_TTL.as_secs(),
        }),
    )
        .into_response()
}

// ─── 辅助 ───────────────────────────────────────────────────────────────────

/// LAN peer 的存活时间（2 小时）
pub const LAN_PEER_TTL: Duration = Duration::from_secs(2 * 60 * 60);

/// 校验 IP 是否是合法的私有 IPv4 地址（防止注入公网 IP）
fn is_valid_lan_ip(ip: &str) -> bool {
    let Ok(addr) = ip.parse::<std::net::Ipv4Addr>() else {
        return false;
    };
    // 私有地址段：10.x.x.x / 172.16-31.x.x / 192.168.x.x
    let octets = addr.octets();
    octets[0] == 10
        || (octets[0] == 172 && (16..=31).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 168)
}

/// 获取当前有效的 LAN peer mirrors（供 version.json 注入使用）
pub async fn get_active_lan_mirrors(
    state: &AppState,
    current_version_code: i64,
) -> Vec<serde_json::Value> {
    let mut reg = state.lan_peer_registry.write().await;
    // 清理过期条目
    reg.retain(|_, e| e.registered_at.elapsed() < LAN_PEER_TTL);

    reg.iter()
        .filter(|(_, e)| e.version_code >= current_version_code)
        .map(|(_, e)| {
            serde_json::json!({
                "url": format!("http://{}:{}{}", e.lan_ip, e.port, e.dist_path),
                "type": "lan",
                // priority=10，高于手机P2P中继的 priority=5，局域网直连最快优先
                "priority": 10
            })
        })
        .collect()
}
