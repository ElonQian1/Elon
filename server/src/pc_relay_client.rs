//! PC 本地 relay 客户端：在本地模式下连接到云端 /agent/ws，
//! 接收 HttpRequest 消息并转发给本机 HTTP server，再把响应回传给云端。
//!
//! 通过 `RELAY_CLOUD_URL` 和 `ELON_AGENT_ID` / `ELON_AGENT_SECRET` 环境变量配置。
//!
//! 典型配置（start-local.ps1 会设置）：
//!   RELAY_CLOUD_URL  = wss://43.139.149.158:8080/agent/ws
//!   ELON_AGENT_ID    = elon-pc-1
//!   ELON_AGENT_SECRET = <64字符随机hex>
//!   LOCAL_SERVER_PORT = 7800

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures::{SinkExt, StreamExt};
use homecli_proto::{AgentToServer, ServerToAgent, PROTO_VERSION};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

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

async fn run_relay_loop(
    cloud_url: String,
    agent_id: String,
    agent_secret: String,
    local_port: u16,
) {
    let mut backoff = Duration::from_secs(2);
    loop {
        match run_relay_session(&cloud_url, &agent_id, &agent_secret, local_port).await {
            Ok(()) => {
                info!("[relay-client] 连接正常断开，{:.1}s 后重连", backoff.as_secs_f32());
            }
            Err(e) => {
                warn!("[relay-client] 连接错误: {e:#}，{:.1}s 后重连", backoff.as_secs_f32());
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(60));
    }
}

async fn run_relay_session(
    cloud_url: &str,
    agent_id: &str,
    agent_secret: &str,
    local_port: u16,
) -> Result<()> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    // into_client_request 会自动生成 Sec-WebSocket-Key 等握手头
    let mut request = cloud_url.into_client_request()?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", agent_secret).parse()?,
    );

    let (mut ws, _) = connect_async(request).await?;
    info!("[relay-client] 已连接到云端 {}", cloud_url);

    // 发送 Register 帧
    let register = AgentToServer::Register {
        agent_id: agent_id.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        proto_version: PROTO_VERSION,
        allowed_clis: vec![],
        allowed_cwds: vec![],
    };
    ws.send(Message::Text(serde_json::to_string(&register)?))
        .await?;
    info!("[relay-client] Register 发送完毕，等待请求...");

    let http_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(55))
        .build()?;

    let local_base = format!("http://127.0.0.1:{}", local_port);

    while let Some(frame) = ws.next().await {
        let frame = frame?;
        let text = match frame {
            Message::Text(t) => t,
            Message::Close(_) => break,
            Message::Ping(d) => {
                ws.send(Message::Pong(d)).await?;
                continue;
            }
            _ => continue,
        };

        let msg: ServerToAgent = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                warn!("[relay-client] 解析消息失败: {e}: {text}");
                continue;
            }
        };

        match msg {
            ServerToAgent::HttpRequest {
                req_id,
                method,
                path,
                headers,
                body_b64,
            } => {
                let url = format!("{}{}", local_base, path);
                let response = handle_http_request(
                    &http_client,
                    &req_id,
                    &method,
                    &url,
                    headers,
                    body_b64,
                )
                .await;
                let resp_text = serde_json::to_string(&response)?;
                ws.send(Message::Text(resp_text)).await?;
            }
            ServerToAgent::Ping { nonce } => {
                let pong = AgentToServer::Pong { nonce };
                ws.send(Message::Text(serde_json::to_string(&pong)?))
                    .await?;
            }
            // exec 命令在本地模式下暂不支持
            ServerToAgent::Exec { task_id, .. } => {
                let err = AgentToServer::TaskError {
                    task_id,
                    message: "本地 relay 模式不支持 exec 命令".into(),
                };
                ws.send(Message::Text(serde_json::to_string(&err)?))
                    .await?;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handle_http_request(
    client: &reqwest::Client,
    req_id: &str,
    method: &str,
    url: &str,
    headers: Vec<(String, String)>,
    body_b64: Option<String>,
) -> AgentToServer {
    let result = async {
        let mut builder = match method {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            m => return Err(anyhow!("不支持的方法: {m}")),
        };

        for (k, v) in &headers {
            builder = builder.header(k.as_str(), v.as_str());
        }

        if let Some(b64) = &body_b64 {
            let body = B64.decode(b64).map_err(|e| anyhow!("body base64 decode: {e}"))?;
            builder = builder.body(body);
        }

        let resp = builder.send().await?;
        let status = resp.status().as_u16();
        let resp_headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
            .collect();
        let body = resp.bytes().await?;
        let body_b64 = if body.is_empty() {
            None
        } else {
            Some(B64.encode(&body))
        };

        Ok(AgentToServer::HttpResponse {
            req_id: req_id.to_string(),
            status,
            headers: resp_headers,
            body_b64,
        })
    }
    .await;

    match result {
        Ok(resp) => resp,
        Err(e) => AgentToServer::HttpError {
            req_id: req_id.to_string(),
            message: e.to_string(),
        },
    }
}
