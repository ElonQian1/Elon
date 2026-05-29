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
use std::time::Duration;
use tokio::sync::mpsc;
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

    tokio::spawn(run_relay_loop(cloud_url, agent_id, agent_secret, local_port));
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
    let mut request = cloud_url.into_client_request()?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", agent_secret).parse()?,
    );

    let (ws_stream, _) = connect_async(request).await?;
    info!("[relay-client] 已连接到云端 {}", cloud_url);

    // 拆分读写，用 channel 让并发任务向 WS 写消息
    let (ws_write, mut ws_read) = ws_stream.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

    // 写任务：drain out_rx → ws_write
    let writer = tokio::spawn(async move {
        let mut sink = ws_write;
        while let Some(msg) = out_rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    // 发送 Register 帧
    let register = AgentToServer::Register {
        agent_id: agent_id.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        proto_version: PROTO_VERSION,
        allowed_clis: vec!["copilot".into(), "codex".into()],
        allowed_cwds: vec![],
    };
    out_tx.send(Message::Text(serde_json::to_string(&register)?))?;
    info!("[relay-client] Register 发送完毕，等待请求...");

    let http_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(55))
        .build()?;
    let local_base = format!("http://127.0.0.1:{}", local_port);

    // 读循环：每条消息都可能并发处理
    while let Some(frame) = ws_read.next().await {
        let frame = frame?;
        let text = match frame {
            Message::Text(t) => t,
            Message::Close(_) => break,
            Message::Ping(d) => {
                let _ = out_tx.send(Message::Pong(d));
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
                let client = http_client.clone();
                let tx = out_tx.clone();
                tokio::spawn(async move {
                    let resp =
                        handle_http_request(&client, &req_id, &method, &url, headers, body_b64)
                            .await;
                    let _ = tx.send(Message::Text(serde_json::to_string(&resp).unwrap()));
                });
            }

            ServerToAgent::CliPrompt {
                req_id,
                cli,
                extra_args,
                prompt,
            } => {
                let tx = out_tx.clone();
                tokio::spawn(handle_cli_prompt(req_id, cli, extra_args, prompt, tx));
            }

            ServerToAgent::Ping { nonce } => {
                let pong = AgentToServer::Pong { nonce };
                let _ = out_tx.send(Message::Text(serde_json::to_string(&pong)?));
            }

            // Exec 在本地 relay 模式下不支持（使用 CliPrompt 替代）
            ServerToAgent::Exec { task_id, .. } => {
                let err = AgentToServer::TaskError {
                    task_id,
                    message: "本地 relay 模式请使用 CliPrompt 代替 Exec".into(),
                };
                let _ = out_tx.send(Message::Text(serde_json::to_string(&err)?));
            }

            ServerToAgent::Cancel { .. } => {
                // TODO: 取消正在运行的 CLI 任务（当前忽略）
            }
        }
    }

    drop(out_tx);
    let _ = writer.await;
    Ok(())
}

// ── CliPrompt 处理 ────────────────────────────────────────────────────────────

async fn handle_cli_prompt(
    req_id: String,
    cli: String,
    extra_args: Vec<String>,
    prompt: String,
    out: mpsc::UnboundedSender<Message>,
) {
    info!("[relay-client] CliPrompt: cli={cli} req_id={req_id}");
    let (exit_ok, error) =
        match run_cli_and_stream(&req_id, &cli, &extra_args, &prompt, &out).await {
            Ok(ok) => (ok, None),
            Err(e) => {
                warn!("[relay-client] CLI 执行失败: {e:#}");
                (false, Some(e.to_string()))
            }
        };
    let done = AgentToServer::CliDone {
        req_id,
        exit_ok,
        error,
    };
    let _ = out.send(Message::Text(serde_json::to_string(&done).unwrap()));
}

async fn run_cli_and_stream(
    req_id: &str,
    cli: &str,
    extra_args: &[String],
    prompt: &str,
    out: &mpsc::UnboundedSender<Message>,
) -> Result<bool> {
    use tokio::io::AsyncBufReadExt;
    use tokio::process::Command;

    let mut cmd = Command::new(cli);
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.arg("-p").arg(prompt);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::null()); // 不转发 stderr（包含 stats/warning）
    cmd.kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| anyhow!("启动 {cli} 失败: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("无法获取 stdout"))?;

    // 流式转发 stdout
    let req_id_s = req_id.to_string();
    let out_clone = out.clone();
    let stream_task = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let chunk = AgentToServer::CliChunk {
                req_id: req_id_s.clone(),
                text: format!("{}\n", line),
            };
            if out_clone
                .send(Message::Text(serde_json::to_string(&chunk).unwrap()))
                .is_err()
            {
                break;
            }
        }
    });

    let status = child.wait().await?;
    let _ = stream_task.await;
    Ok(status.success())
}

// ── HTTP 请求转发 ─────────────────────────────────────────────────────────────

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
