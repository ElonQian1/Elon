//! PC 本地 relay 客户端：在本地模式下连接到云端 /agent/ws，
//! 接收 HttpRequest / CliPrompt 消息并在本机处理，把结果回传给云端。
//!
//! 支持并发：多个请求同时处理，不互相阻塞。
//!
//! 通过环境变量配置（start-local.ps1 会设置）：
//!   RELAY_CLOUD_URL   = ws://43.139.149.158:8080/agent/ws
//!   ELON_AGENT_ID     = elon-pc-1
//!   ELON_AGENT_SECRET = <64字符随机hex>
//!   LOCAL_SERVER_PORT = 7800

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures::{SinkExt, StreamExt};
use homecli_proto::{AgentToServer, ServerToAgent, PROTO_VERSION};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, Mutex},
    task::AbortHandle,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

/// WS ping 间隔：每 30s 向云端发一次 WS-level Ping，检测 zombie 连接
const WS_PING_INTERVAL: Duration = Duration::from_secs(30);
/// 读超时：90s 内如果未收到任何 WS frame（包括 Pong），视为 zombie，强制断开重连
const WS_READ_TIMEOUT: Duration = Duration::from_secs(90);

type RunningCliTasks = Arc<Mutex<HashMap<String, AbortHandle>>>;

/// 从环境变量读取 relay 配置，启动后台连接循环（自动重连）
pub fn spawn_if_configured() {
    let cloud_url = match std::env::var("RELAY_CLOUD_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };
    let agent_id = std::env::var("ELON_AGENT_ID").unwrap_or_else(|_| "elon-pc-1".into());
    let agent_secret = match std::env::var("ELON_AGENT_SECRET") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            warn!("[relay-client] RELAY_CLOUD_URL 已设置但缺少 ELON_AGENT_SECRET，跳过连接");
            return;
        }
    };
    let local_port: u16 = std::env::var("LOCAL_SERVER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7800);

    info!(
        "[relay-client] 启动反向代理 agent: {} → {}",
        agent_id, cloud_url
    );

    tokio::spawn(run_relay_loop(
        cloud_url,
        agent_id,
        agent_secret,
        local_port,
    ));
}


#[path = "pc_relay_client_impl.rs"]
mod relay_impl;
use self::relay_impl::*;
