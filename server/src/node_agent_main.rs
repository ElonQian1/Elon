//! elon-node-agent：用户 PC 端节点代理，将本机 LLM 算力贡献给 elon 平台。
//!
//! ## 使用方法
//!
//! ```bash
//! # Linux / macOS
//! export NODE_CLOUD_URL=ws://43.139.149.158:8080/agent/ws
//! export NODE_AGENT_ID=my-pc-node-1
//! export NODE_AGENT_SECRET=<64字符随机hex>
//! export NODE_OWNER_USER_ID=<你的 elon 用户 ID>
//! export NODE_OLLAMA_URL=http://localhost:11434   # 可选，默认值即此
//! ./elon-node-agent
//!
//! # Windows PowerShell
//! $env:NODE_CLOUD_URL = "ws://43.139.149.158:8080/agent/ws"
//! $env:NODE_AGENT_ID = "my-pc-node-1"
//! $env:NODE_AGENT_SECRET = "<64字符随机hex>"
//! $env:NODE_OWNER_USER_ID = "<你的 elon 用户 ID>"
//! .\elon-node-agent.exe
//! ```
//!
//! ## 工作流
//!
//! 1. 启动后扫描本机 Ollama / LM Studio / 自定义 OpenAI-compatible 端口
//! 2. 连接云端 /agent/ws，发送 Register + RegisterCapabilities
//! 3. 监听 LlmStreamRequest，转发给本地 LLM，流式返回 LlmStreamChunk + LlmStreamEnd
//! 4. 断线后自动重连（指数退避 2s ~ 60s）

use anyhow::{anyhow, Result};
use futures::{SinkExt, StreamExt};
use homecli_proto::{AgentToServer, ModelCapability, ServerToAgent, PROTO_VERSION};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

// ── 配置结构 ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct NodeConfig {
    cloud_url: String,
    /// 云端 HTTP/HTTPS 地址（用于 REST API 调用，如注册外部项目）。
    /// 默认从 cloud_url 派生：ws://X → http://X，wss://X → https://X。
    cloud_http_url: String,
    agent_id: String,
    agent_secret: String,
    owner_user_id: String,
    /// 可选：你的 elon 登录 token（用于本地 Web 管理页代理调用云端 API，例如注册外部项目）
    user_token: Option<String>,
    /// 本地 Ollama 地址
    ollama_url: String,
    /// 可选：LM Studio 地址
    lm_studio_url: Option<String>,
    /// 用户自定义 OpenAI-compatible 地址
    custom_url: Option<String>,
    /// 每 1k tokens 收取的平台积分（默认 0.1）
    price_per_1k: f64,
}

fn derive_http_url(ws_url: &str) -> String {
    if let Some(rest) = ws_url.strip_prefix("wss://") {
        format!("https://{}", rest.split('/').next().unwrap_or(rest))
    } else if let Some(rest) = ws_url.strip_prefix("ws://") {
        format!("http://{}", rest.split('/').next().unwrap_or(rest))
    } else {
        ws_url.to_string()
    }
}

impl NodeConfig {
    fn from_env() -> Result<Self> {
        let cloud_url = std::env::var("NODE_CLOUD_URL")
            .unwrap_or_else(|_| "ws://43.139.149.158:8080/agent/ws".into());
        let cloud_http_url = std::env::var("NODE_CLOUD_HTTP_URL")
            .unwrap_or_else(|_| derive_http_url(&cloud_url));
        let agent_id = std::env::var("NODE_AGENT_ID")
            .map_err(|_| anyhow!("必须设置 NODE_AGENT_ID 环境变量"))?;
        let agent_secret = std::env::var("NODE_AGENT_SECRET")
            .map_err(|_| anyhow!("必须设置 NODE_AGENT_SECRET 环境变量"))?;
        let owner_user_id = std::env::var("NODE_OWNER_USER_ID")
            .map_err(|_| anyhow!("必须设置 NODE_OWNER_USER_ID（你的 elon 用户 ID）"))?;
        let user_token = std::env::var("NODE_USER_TOKEN").ok().filter(|v| !v.is_empty());
        let ollama_url = std::env::var("NODE_OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".into());
        let lm_studio_url = std::env::var("NODE_LM_STUDIO_URL").ok().filter(|v| !v.is_empty());
        let custom_url = std::env::var("NODE_CUSTOM_LLM_URL").ok().filter(|v| !v.is_empty());
        let price_per_1k = std::env::var("NODE_PRICE_PER_1K")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.1f64);

        Ok(Self {
            cloud_url,
            cloud_http_url,
            agent_id,
            agent_secret,
            owner_user_id,
            user_token,
            ollama_url,
            lm_studio_url,
            custom_url,
            price_per_1k,
        })
    }
}

// ── 本地 LLM 扫描 ─────────────────────────────────────────────────────────────

/// Ollama /api/tags 响应
#[derive(Deserialize)]
struct OllamaTagsResp {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

/// OpenAI-compatible /v1/models 响应
#[derive(Deserialize)]
struct OpenAiModelsResp {
    data: Vec<OpenAiModel>,
}

#[derive(Deserialize)]
struct OpenAiModel {
    id: String,
}

async fn scan_ollama(base_url: &str, price: f64) -> Vec<ModelCapability> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    let url = format!("{}/api/tags", base_url);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<OllamaTagsResp>().await {
                return body
                    .models
                    .into_iter()
                    .map(|m| ModelCapability {
                        model_id: m.name.clone(),
                        display_name: m.name,
                        context_len: 4096,
                        provider: "ollama".into(),
                        price_per_1k_credits: price,
                    })
                    .collect();
            }
        }
        _ => {}
    }
    vec![]
}

async fn scan_openai_compat(base_url: &str, provider: &str, price: f64) -> Vec<ModelCapability> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    let url = format!("{}/v1/models", base_url);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<OpenAiModelsResp>().await {
                return body
                    .data
                    .into_iter()
                    .map(|m| ModelCapability {
                        model_id: m.id.clone(),
                        display_name: m.id,
                        context_len: 4096,
                        provider: provider.to_string(),
                        price_per_1k_credits: price,
                    })
                    .collect();
            }
        }
        _ => {}
    }
    vec![]
}

async fn discover_models(cfg: &NodeConfig) -> Vec<ModelCapability> {
    let mut models = Vec::new();

    let ollama = scan_ollama(&cfg.ollama_url, cfg.price_per_1k).await;
    if !ollama.is_empty() {
        info!("✅ Ollama: {} 个模型", ollama.len());
        models.extend(ollama);
    }

    if let Some(ref url) = cfg.lm_studio_url {
        let lm = scan_openai_compat(url, "lm_studio", cfg.price_per_1k).await;
        if !lm.is_empty() {
            info!("✅ LM Studio: {} 个模型", lm.len());
            models.extend(lm);
        }
    }

    if let Some(ref url) = cfg.custom_url {
        let custom = scan_openai_compat(url, "custom", cfg.price_per_1k).await;
        if !custom.is_empty() {
            info!("✅ 自定义 LLM: {} 个模型", custom.len());
            models.extend(custom);
        }
    }

    models
}

// ── LLM 推理（OpenAI-compatible 流式）────────────────────────────────────────

/// 调用本地 LLM（OpenAI-compatible stream 接口），把 chunk 通过 out_tx 发回云端
async fn run_llm_inference(
    cfg: &NodeConfig,
    req_id: String,
    model: &str,
    messages: Vec<serde_json::Value>,
    max_tokens: Option<u32>,
    out_tx: mpsc::UnboundedSender<Message>,
) {
    // 选择端点
    let base_url = if model.contains('/') || cfg.lm_studio_url.is_some() {
        cfg.lm_studio_url.as_deref().unwrap_or(&cfg.ollama_url)
    } else {
        &cfg.ollama_url
    };

    // Ollama 使用 /api/chat，其余使用 /v1/chat/completions
    let endpoint = if base_url.contains(":11434") {
        format!("{}/api/chat", base_url)
    } else {
        format!("{}/v1/chat/completions", base_url)
    };

    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
    });
    if let Some(mt) = max_tokens {
        body["max_tokens"] = serde_json::json!(mt);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_default();

    let resp = match client.post(&endpoint).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamError {
                req_id,
                message: format!("LLM 请求失败: {e}"),
            }));
            return;
        }
    };

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let msg = resp.text().await.unwrap_or_default();
        let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamError {
            req_id,
            message: format!("LLM 错误 {status}: {msg}"),
        }));
        return;
    }

    // 读取 SSE 流
    let mut prompt_tokens = 0u32;
    let mut completion_tokens = 0u32;
    let mut finish_reason = "stop".to_string();
    let mut stream = resp.bytes_stream();

    let mut buf = String::new();
    while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
        let bytes = match chunk {
            Ok(b) => b,
            Err(e) => {
                warn!("LLM 流读取错误: {e}");
                break;
            }
        };
        buf.push_str(&String::from_utf8_lossy(&bytes));

        // SSE 每行 "data: {...}\n\n" 或 Ollama JSON lines
        loop {
            if let Some(pos) = buf.find('\n') {
                let line = buf[..pos].trim().to_string();
                buf.drain(..=pos);

                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }

                let json_str = line.strip_prefix("data: ").unwrap_or(&line);
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                    // OpenAI-compatible delta
                    if let Some(delta) = val
                        .pointer("/choices/0/delta/content")
                        .and_then(|v| v.as_str())
                    {
                        if !delta.is_empty() {
                            completion_tokens += 1; // 近似计数
                            let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamChunk {
                                req_id: req_id.clone(),
                                delta: delta.to_string(),
                            }));
                        }
                    }
                    // Ollama message.content
                    if let Some(content) = val
                        .pointer("/message/content")
                        .and_then(|v| v.as_str())
                    {
                        if !content.is_empty() {
                            completion_tokens += 1;
                            let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamChunk {
                                req_id: req_id.clone(),
                                delta: content.to_string(),
                            }));
                        }
                    }
                    // 完成信号
                    if let Some(r) = val.pointer("/choices/0/finish_reason").and_then(|v| v.as_str()) {
                        if !r.is_empty() && r != "null" {
                            finish_reason = r.to_string();
                        }
                    }
                    if val.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                        prompt_tokens = val
                            .pointer("/prompt_eval_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;
                        completion_tokens = val
                            .pointer("/eval_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(completion_tokens as u64) as u32;
                    }
                    // token usage from OpenAI response
                    if let Some(usage) = val.get("usage") {
                        prompt_tokens = usage
                            .get("prompt_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(prompt_tokens as u64) as u32;
                        completion_tokens = usage
                            .get("completion_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(completion_tokens as u64) as u32;
                    }
                }
            } else {
                break;
            }
        }
    }

    let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamEnd {
        req_id,
        prompt_tokens,
        completion_tokens,
        finish_reason,
    }));
}

fn ws_text(msg: &AgentToServer) -> Message {
    Message::Text(serde_json::to_string(msg).unwrap_or_default())
}

// ── 主连接循环 ────────────────────────────────────────────────────────────────

async fn run_session(cfg: &NodeConfig) -> Result<()> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut request = cfg.cloud_url.as_str().into_client_request()?;
    request
        .headers_mut()
        .insert("Authorization", format!("Bearer {}", cfg.agent_secret).parse()?);

    let (ws_stream, _) = connect_async(request).await?;
    info!("✅ 已连接到云端: {}", cfg.cloud_url);

    let (ws_write, mut ws_read) = ws_stream.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

    // 写任务
    let writer = tokio::spawn(async move {
        let mut sink = ws_write;
        while let Some(msg) = out_rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    // 扫描本地模型
    let models = discover_models(cfg).await;
    if models.is_empty() {
        warn!("⚠️  未发现本地 LLM，节点将以无模型状态上线（可后续发送 RegisterCapabilities 更新）");
    } else {
        info!(
            "🧠 发现 {} 个本地模型: {}",
            models.len(),
            models.iter().map(|m| m.model_id.as_str()).collect::<Vec<_>>().join(", ")
        );
    }

    // 发送 Register
    out_tx.send(ws_text(&AgentToServer::Register {
        agent_id: cfg.agent_id.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        proto_version: PROTO_VERSION,
        allowed_clis: vec![],
        allowed_cwds: vec![],
        owner_user_id: Some(cfg.owner_user_id.clone()),
    }))?;

    // 发送 RegisterCapabilities
    out_tx.send(ws_text(&AgentToServer::RegisterCapabilities {
        models: models.clone(),
    }))?;

    // WS ping 定时器
    let ping_tx = out_tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if ping_tx.send(Message::Ping(vec![])).is_err() {
                break;
            }
        }
    });

    let cfg_r = cfg.clone();
    let out_tx_r = out_tx.clone();
    // 读取服务器消息
    let read_result: Result<()> = async {
        while let Some(frame) = ws_read.next().await {
            let frame = frame.map_err(|e| anyhow!("ws read: {e}"))?;
            match frame {
                Message::Text(t) => {
                    let msg: ServerToAgent = match serde_json::from_str(&t) {
                        Ok(m) => m,
                        Err(e) => {
                            warn!("反序列化服务器消息失败: {e}: {t}");
                            continue;
                        }
                    };
                    match msg {
                        ServerToAgent::LlmStreamRequest {
                            req_id,
                            model,
                            messages,
                            max_tokens,
                        } => {
                            info!("📨 LLM 推理请求: {} model={}", req_id, model);
                            let cfg_c = cfg_r.clone();
                            let tx_c = out_tx_r.clone();
                            tokio::spawn(async move {
                                run_llm_inference(&cfg_c, req_id, &model, messages, max_tokens, tx_c).await;
                            });
                        }
                        ServerToAgent::Ping { nonce } => {
                            let _ = out_tx_r.send(ws_text(&AgentToServer::Pong { nonce }));
                        }
                        _ => {
                            // HttpRequest / CliPrompt / Exec — node-agent 不处理这些
                        }
                    }
                }
                Message::Pong(_) => {}
                Message::Close(_) => break,
                _ => {}
            }
        }
        Ok(())
    }
    .await;

    drop(out_tx);
    let _ = writer.await;
    read_result
}

async fn run_loop(cfg: NodeConfig, runtime: Arc<NodeRuntime>) {
    let mut backoff = Duration::from_secs(2);
    loop {
        runtime.set_connected(false, "连接中…").await;
        match run_session(&cfg).await {
            Ok(()) => {
                info!("连接正常断开，{:.1}s 后重连", backoff.as_secs_f32());
                runtime.set_connected(false, "已断开，等待重连").await;
                backoff = Duration::from_secs(2);
            }
            Err(e) => {
                warn!("连接错误: {e:#}，{:.1}s 后重连", backoff.as_secs_f32());
                runtime.set_connected(false, &format!("错误: {}", e)).await;
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(60));
    }
}

// ── 入口 ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = NodeConfig::from_env()?;
    info!("🚀 elon-node-agent {} 启动 (agent_id: {})", env!("CARGO_PKG_VERSION"), cfg.agent_id);
    info!("   云端: {}", cfg.cloud_url);
    info!("   Ollama: {}", cfg.ollama_url);
    info!("   积分价格: {} credits/1k tokens", cfg.price_per_1k);

    let runtime = Arc::new(NodeRuntime::new(cfg.clone()));
    spawn_admin_server(runtime.clone());

    run_loop(cfg, runtime).await;
    Ok(())
}

// ── 本地 Web 管理页 (端口可通过 NODE_ADMIN_PORT 配置，默认 7799) ──────────

#[derive(Default)]
struct NodeStatus {
    connected: bool,
    last_event: String,
    models_cached: Vec<ModelCapability>,
}

struct NodeRuntime {
    cfg: NodeConfig,
    status: RwLock<NodeStatus>,
}

impl NodeRuntime {
    fn new(cfg: NodeConfig) -> Self {
        Self { cfg, status: RwLock::new(NodeStatus::default()) }
    }

    async fn set_connected(&self, on: bool, evt: &str) {
        let mut s = self.status.write().await;
        s.connected = on;
        s.last_event = evt.to_string();
    }

    async fn set_models(&self, models: Vec<ModelCapability>) {
        self.status.write().await.models_cached = models;
    }
}

fn spawn_admin_server(runtime: Arc<NodeRuntime>) {
    let port: u16 = std::env::var("NODE_ADMIN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7799);
    let addr: std::net::SocketAddr = ([127, 0, 0, 1], port).into();
    tokio::spawn(async move {
        let app = axum::Router::new()
            .route("/", axum::routing::get(admin_index))
            .route("/api/status", axum::routing::get(admin_status))
            .route("/api/register-project", axum::routing::post(admin_register_project))
            .with_state(runtime);
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                info!("🖥️  本地管理页: http://127.0.0.1:{}/", port);
                if let Err(e) = axum::serve(listener, app).await {
                    warn!("admin server 退出: {e}");
                }
            }
            Err(e) => warn!("admin server 无法监听 {addr}: {e}"),
        }
    });
}

async fn admin_index() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("node_agent_admin.html"))
}

async fn admin_status(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let live = discover_models(&rt.cfg).await;
    rt.set_models(live.clone()).await;
    let st = rt.status.read().await;
    axum::Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "agent_id": rt.cfg.agent_id,
        "owner_user_id": rt.cfg.owner_user_id,
        "cloud_url": rt.cfg.cloud_url,
        "cloud_http_url": rt.cfg.cloud_http_url,
        "user_token_configured": rt.cfg.user_token.is_some(),
        "ollama_url": rt.cfg.ollama_url,
        "lm_studio_url": rt.cfg.lm_studio_url,
        "custom_url": rt.cfg.custom_url,
        "price_per_1k": rt.cfg.price_per_1k,
        "connected": st.connected,
        "last_event": st.last_event,
        "models": live,
    }))
}

#[derive(Deserialize)]
struct AdminRegisterReq {
    name: String,
    workspace_path: String,
    description: Option<String>,
}

/// 本地管理页 → 注册外部本地项目到云端。
/// 流程：
///   1. 在 PC 本地校验路径存在且为目录（这是关键 —— 服务器看不到 PC 路径）
///   2. 用 NODE_USER_TOKEN 调用云端 POST /api/projects/external，附带 node_id 让服务器跳过路径校验
async fn admin_register_project(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<AdminRegisterReq>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use axum::http::StatusCode;

    let name = req.name.trim();
    let path = req.workspace_path.trim();
    if name.is_empty() || path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "ok": false, "error": "name 和 workspace_path 不能为空" })),
        );
    }

    // 1) PC 本地校验
    let pb = std::path::Path::new(path);
    if !pb.exists() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": format!("PC 本地路径不存在: {}", path),
            })),
        );
    }
    if !pb.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "ok": false, "error": "workspace_path 必须是目录" })),
        );
    }

    // 2) 必须有 token 才能调用云端
    let token = match rt.cfg.user_token.as_ref() {
        Some(t) => t.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": "未配置 NODE_USER_TOKEN 环境变量。请在 APK 登录后从『我的』→『设置』复制 token 到 PC 节点启动环境中。",
                })),
            );
        }
    };

    // 3) 转发到云端
    let url = format!("{}/api/projects/external", rt.cfg.cloud_http_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "name": name,
        "workspace_path": path,
        "description": req.description,
        "node_id": rt.cfg.agent_id,
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    match client.post(&url).bearer_auth(&token).json(&body).send().await {
        Ok(resp) => {
            let status = resp.status();
            let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
            if status.is_success() {
                (StatusCode::OK, axum::Json(serde_json::json!({
                    "ok": true,
                    "cloud": json,
                })))
            } else {
                (StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                 axum::Json(serde_json::json!({
                    "ok": false,
                    "error": format!("云端返回 {}: {}", status, json),
                })))
            }
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": format!("调用云端失败: {}", e),
            })),
        ),
    }
}
