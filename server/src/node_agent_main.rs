//! elon-node-agent：用户 PC 端节点代理，将本机 LLM 算力贡献给 elon 平台。
//!
//! ## 使用方法（普通用户：零配置）
//!
//! 1. 双击启动器（或直接运行 `elon-node-agent`），它会启动本地管理页 http://127.0.0.1:7799/
//! 2. 在管理页用 **账号 + 密码** 登录一次（也可直接粘贴 token）
//! 3. 节点自动向云端注册，生成 agent_id + secret 并持久化到本地配置文件
//! 4. 之后每次启动自动读取凭证、自动连接，无需再配置
//!
//! ## 高级用户（可选环境变量覆盖）
//!
//! ```bash
//! NODE_CLOUD_URL       云端 WebSocket 地址（默认 ws://43.139.149.158:8080/agent/ws）
//! NODE_USER_TOKEN      登录 token：设置后首次启动自动注册，无需网页登录
//! NODE_AGENT_ID        手动指定节点 ID（搭配 NODE_AGENT_SECRET 跳过自动注册）
//! NODE_AGENT_SECRET    手动指定密钥
//! NODE_OLLAMA_URL      本地 Ollama 地址（默认 http://localhost:11434）
//! ```
//!
//! ## 工作流
//!
//! 1. 启动后扫描本机 Ollama / LM Studio / 自定义 OpenAI-compatible 端口
//! 2. 有凭证 → 连接云端 /agent/ws，发送 Register + RegisterCapabilities
//! 3. 监听 LlmStreamRequest，转发给本地 LLM，流式返回 LlmStreamChunk + LlmStreamEnd
//! 4. 断线后自动重连（指数退避 2s ~ 60s）；未登录时等待网页登录

use anyhow::{anyhow, Result};
use futures::{SinkExt, StreamExt};
use homecli_proto::{AgentToServer, ModelCapability, ServerToAgent, PROTO_VERSION};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Notify, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

// ── 配置结构 ──────────────────────────────────────────────────────────────────

/// 静态运行配置（云端地址、本地模型地址、价格），均有合理默认值，普通用户无需配置。
#[derive(Clone)]
struct NodeConfig {
    cloud_url: String,
    /// 云端 HTTP/HTTPS 地址（用于 REST API 调用，如登录、注册节点、注册外部项目）。
    /// 默认从 cloud_url 派生：ws://X → http://X，wss://X → https://X。
    cloud_http_url: String,
    /// 本地 Ollama 地址
    ollama_url: String,
    /// 可选：LM Studio 地址
    lm_studio_url: Option<String>,
    /// 用户自定义 OpenAI-compatible 地址
    custom_url: Option<String>,
    /// 每 1k tokens 收取的平台积分（默认 0.1）
    price_per_1k: f64,
}

/// 节点凭证：由「一次登录」自动换取并持久化，普通用户永远不用手动填。
#[derive(Clone)]
struct Credentials {
    agent_id: String,
    agent_secret: String,
    owner_user_id: String,
    /// 用户的 elon 登录 token（用于代理调用云端 API，例如注册外部项目）
    user_token: Option<String>,
}

/// 持久化到磁盘的状态（`%APPDATA%\elon-node-agent\node.json` / `~/.config/elon-node-agent/node.json`）。
#[derive(Default, Serialize, Deserialize)]
struct PersistedState {
    agent_id: Option<String>,
    agent_secret: Option<String>,
    owner_user_id: Option<String>,
    user_token: Option<String>,
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

/// 本机名，作为节点 label / 登录设备名。
fn machine_label() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "pc".into())
}

/// 凭证持久化文件路径。
fn state_path() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    } else {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("elon-node-agent")
        .join("node.json")
}

fn load_persisted() -> PersistedState {
    std::fs::read_to_string(state_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_persisted(s: &PersistedState) {
    let p = state_path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(&p, json);
    }
}

impl PersistedState {
    fn from_creds(c: &Credentials) -> Self {
        Self {
            agent_id: Some(c.agent_id.clone()),
            agent_secret: Some(c.agent_secret.clone()),
            owner_user_id: Some(c.owner_user_id.clone()),
            user_token: c.user_token.clone(),
        }
    }
}

/// 从环境变量 / 持久化文件解析已有凭证；都没有时返回 None（需登录）。
/// 环境变量优先（供高级用户/服务器覆盖），否则用上次持久化的结果。
fn initial_credentials(persisted: &PersistedState) -> Option<Credentials> {
    let env_nonempty = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let agent_id = env_nonempty("NODE_AGENT_ID").or_else(|| persisted.agent_id.clone())?;
    let agent_secret = env_nonempty("NODE_AGENT_SECRET").or_else(|| persisted.agent_secret.clone())?;
    let owner_user_id = env_nonempty("NODE_OWNER_USER_ID")
        .or_else(|| persisted.owner_user_id.clone())
        .unwrap_or_default();
    let user_token = env_nonempty("NODE_USER_TOKEN").or_else(|| persisted.user_token.clone());
    Some(Credentials { agent_id, agent_secret, owner_user_id, user_token })
}

/// 用登录 token 调用云端 `POST /api/me/nodes/register`，自动换取节点 agent_id + secret。
async fn provision_node(cfg: &NodeConfig, token: &str) -> Result<Credentials> {
    let url = format!("{}/api/me/nodes/register", cfg.cloud_http_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&serde_json::json!({ "label": machine_label() }))
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("注册节点失败 {}: {}", status, body));
    }
    let j: serde_json::Value = resp.json().await?;
    let agent_id = j
        .get("agent_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("响应缺少 agent_id"))?
        .to_string();
    let agent_secret = j
        .get("agent_secret")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("响应缺少 agent_secret"))?
        .to_string();
    let owner_user_id = j
        .get("owner_user_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    Ok(Credentials {
        agent_id,
        agent_secret,
        owner_user_id,
        user_token: Some(token.to_string()),
    })
}

/// 账号 + 密码登录云端，换取 token。
async fn cloud_login(cfg: &NodeConfig, account: &str, password: &str) -> Result<String> {
    let url = format!("{}/api/auth/login", cfg.cloud_http_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "account": account,
            "password": password,
            "device_name": machine_label(),
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("登录失败 {}: {}", status, body));
    }
    let j: serde_json::Value = resp.json().await?;
    j.get("token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("登录响应缺少 token"))
}

impl NodeConfig {
    fn from_env() -> Result<Self> {
        let cloud_url = std::env::var("NODE_CLOUD_URL")
            .unwrap_or_else(|_| "ws://43.139.149.158:8080/agent/ws".into());
        let cloud_http_url = std::env::var("NODE_CLOUD_HTTP_URL")
            .unwrap_or_else(|_| derive_http_url(&cloud_url));
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

// ── CLI 执行（CliPrompt / Exec）────────────────────────────────────────────────

/// 检测本机有哪些 CLI 可用。
/// 检测本机可用的 CLI，返回 (cli名称, 完整路径) 对。
///
/// 两路并行扫描，取最优路径：
///   1. `where`/`which` 从 PATH 查（快，但受启动时 PATH 限制）
///   2. 直接扫描常见安装目录（健壮，不依赖 PATH 是否完整）
///
/// 路径优先级（Windows）：
///   a. 常见目录里找到的 .cmd（最可靠）
///   b. PATH 里找到的 .cmd 且不含 VS Code globalStorage
///   c. 其他非 globalStorage 路径
///   d. 兜底：任何找到的路径
fn detect_available_clis() -> Vec<(String, String)> {
    let candidates = ["copilot", "codex", "gh"];
    candidates.iter().filter_map(|name| {
        let mut candidates_paths: Vec<String> = Vec::new();

        // ── 1. where/which ────────────────────────────────────────────────
        let which_cmd = if cfg!(windows) { "where" } else { "which" };
        if let Ok(out) = std::process::Command::new(which_cmd).arg(name).output() {
            if out.status.success() {
                let from_path: Vec<String> = String::from_utf8_lossy(&out.stdout)
                    .lines().map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty()).collect();
                candidates_paths.extend(from_path);
            }
        }

        // ── 2. 直接扫描常见安装目录（不依赖 PATH）────────────────────────
        #[cfg(windows)]
        {
            let appdata = std::env::var("APPDATA").unwrap_or_default();
            let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
            let userprofile = std::env::var("USERPROFILE").unwrap_or_default();
            let common_dirs = [
                // npm global（最常见）
                format!("{}\\npm", appdata),
                // yarn global
                format!("{}\\Yarn\\bin", localappdata),
                // pnpm global
                format!("{}\\pnpm", appdata),
                // volta 管理的
                format!("{}\\.volta\\bin", userprofile),
                // nvm 管理的（n-v-m for Windows）
                format!("{}\\nvm", appdata),
                // GitHub CLI
                "C:\\Program Files\\GitHub CLI".to_string(),
                "C:\\Program Files (x86)\\GitHub CLI".to_string(),
                // Scoop
                format!("{}\\scoop\\shims", userprofile),
                // winget/直接装在 ProgramFiles
                "C:\\Program Files\\GitHub CLI".to_string(),
            ];
            for dir in &common_dirs {
                // 尝试 name.cmd / name.exe / name
                for ext in &[".cmd", ".exe", ""] {
                    let p = format!("{}\\{}{}", dir, name, ext);
                    if std::path::Path::new(&p).exists() {
                        if !candidates_paths.contains(&p) {
                            candidates_paths.push(p);
                        }
                    }
                }
            }
        }
        #[cfg(not(windows))]
        {
            let home = std::env::var("HOME").unwrap_or_default();
            let common_dirs = [
                "/usr/local/bin".to_string(),
                "/usr/bin".to_string(),
                format!("{}/.npm-global/bin", home),
                format!("{}/.local/bin", home),
                format!("{}/.yarn/bin", home),
                format!("{}/.volta/bin", home),
            ];
            for dir in &common_dirs {
                let p = format!("{}/{}", dir, name);
                if std::path::Path::new(&p).exists() {
                    if !candidates_paths.contains(&p) {
                        candidates_paths.push(p);
                    }
                }
            }
        }

        if candidates_paths.is_empty() { return None; }

        // ── 3. 选最优路径 ────────────────────────────────────────────────
        // 过滤掉 VS Code 内置路径（无法独立运行）
        let not_vscode = |p: &&String| {
            let lower = p.to_lowercase();
            !lower.contains("globalstorage") && !lower.contains("copilotcli\\copilot")
        };

        #[cfg(windows)]
        let best = candidates_paths.iter()
            // a. 常见目录里的 .cmd（最可靠）
            .find(|p| p.to_lowercase().ends_with(".cmd") && not_vscode(p))
            // b. PATH 里的非 VS Code .cmd
            .or_else(|| candidates_paths.iter().find(|p| p.to_lowercase().ends_with(".cmd")))
            // c. 任何非 VS Code 路径
            .or_else(|| candidates_paths.iter().find(not_vscode))
            // d. 兜底
            .or_else(|| candidates_paths.first());

        #[cfg(not(windows))]
        let best = candidates_paths.iter().find(not_vscode)
            .or_else(|| candidates_paths.first());

        best.cloned().map(|p| (name.to_string(), p))
    }).collect()
}

/// 执行 CliPrompt：启动 CLI 子进程，流式返回输出。
/// `bin` 是完整路径（如 `C:\Users\...\copilot`），`cli_name` 是原始名称（用于路由判断）。
async fn run_cli_prompt(
    req_id: String,
    bin: &str,
    cli_name: &str,
    extra_args: Vec<String>,
    cwd: Option<String>,
    prompt: String,
    out_tx: tokio::sync::mpsc::UnboundedSender<Message>,
) {
    use tokio::io::AsyncBufReadExt;

    // Windows 上 .cmd 文件必须通过 cmd /c 启动，否则 tokio::process::Command 无法直接执行
    let (actual_bin, actual_args_prefix): (&str, Vec<&str>) = if cfg!(windows) && bin.to_lowercase().ends_with(".cmd") {
        ("cmd", vec!["/c", bin])
    } else {
        (bin, vec![])
    };

    // 构建命令：copilot -p "<prompt>" 或 codex -p "<prompt>"
    let mut cmd = tokio::process::Command::new(actual_bin);
    for a in &actual_args_prefix {
        cmd.arg(a);
    }
    for a in &extra_args {
        cmd.arg(a);
    }
    // 常见 CLI 的 prompt 标志
    if cli_name == "codex" {
        cmd.arg("-p").arg(&prompt);
    } else if cli_name == "copilot" {
        // --allow-all: 非交互模式下自动确认所有工具调用，无需 stdin 确认
        // 等价于环境变量 COPILOT_ALLOW_ALL=1
        cmd.args(["--allow-all", "-p", &prompt]);
    } else {
        cmd.arg(&prompt);
    }
    if let Some(dir) = &cwd {
        cmd.current_dir(dir);
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = out_tx.send(ws_text(&AgentToServer::CliDone {
                req_id,
                exit_ok: false,
                error: Some(format!("无法启动 {} : {}", bin, e)),
            }));
            return;
        }
    };

    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    let mut stderr_lines = tokio::io::BufReader::new(stderr).lines();

    loop {
        tokio::select! {
            line = stdout_lines.next_line() => match line {
                Ok(Some(l)) => { let _ = out_tx.send(ws_text(&AgentToServer::CliChunk { req_id: req_id.clone(), text: l + "\n" })); }
                Ok(None) => break,
                Err(e) => { warn!("stdout 读取错误: {e}"); break; }
            },
            line = stderr_lines.next_line() => match line {
                Ok(Some(l)) => { let _ = out_tx.send(ws_text(&AgentToServer::CliChunk { req_id: req_id.clone(), text: l + "\n" })); }
                Ok(None) => {}
                Err(_) => {}
            },
        }
    }

    let exit_ok = child.wait().await.map(|s| s.success()).unwrap_or(false);
    let _ = out_tx.send(ws_text(&AgentToServer::CliDone { req_id, exit_ok, error: None }));
}

/// 执行 Exec：运行任意命令，流式返回 TaskStdout/TaskStderr/TaskExit。
async fn run_exec(
    task_id: String,
    cli: String,
    args: Vec<String>,
    cwd: String,
    env_vars: Vec<(String, String)>,
    out_tx: tokio::sync::mpsc::UnboundedSender<Message>,
) {
    use tokio::io::AsyncBufReadExt;

    let mut cmd = tokio::process::Command::new(&cli);
    cmd.args(&args).current_dir(&cwd);
    for (k, v) in &env_vars {
        cmd.env(k, v);
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = out_tx.send(ws_text(&AgentToServer::TaskError {
                task_id,
                message: format!("无法启动 {}: {}", cli, e),
            }));
            return;
        }
    };

    let pid = child.id().unwrap_or(0);
    let _ = out_tx.send(ws_text(&AgentToServer::TaskStarted { task_id: task_id.clone(), pid }));

    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    let mut stderr_lines = tokio::io::BufReader::new(stderr).lines();
    let mut stdout_done = false;
    let mut stderr_done = false;

    while !stdout_done || !stderr_done {
        tokio::select! {
            line = stdout_lines.next_line(), if !stdout_done => match line {
                Ok(Some(l)) => { let _ = out_tx.send(ws_text(&AgentToServer::TaskStdout { task_id: task_id.clone(), data: l + "\n" })); }
                Ok(None) => { stdout_done = true; }
                Err(e) => { warn!("stdout err: {e}"); stdout_done = true; }
            },
            line = stderr_lines.next_line(), if !stderr_done => match line {
                Ok(Some(l)) => { let _ = out_tx.send(ws_text(&AgentToServer::TaskStderr { task_id: task_id.clone(), data: l + "\n" })); }
                Ok(None) => { stderr_done = true; }
                Err(e) => { warn!("stderr err: {e}"); stderr_done = true; }
            },
        }
    }

    let code = child.wait().await.ok().and_then(|s| s.code());
    let _ = out_tx.send(ws_text(&AgentToServer::TaskExit { task_id, code }));
}

// ── 主连接循环 ────────────────────────────────────────────────────────────────

async fn run_session(
    cfg: &NodeConfig,
    creds: &Credentials,
    runtime: &Arc<NodeRuntime>,
) -> Result<()> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut request = cfg.cloud_url.as_str().into_client_request()?;
    request
        .headers_mut()
        .insert("Authorization", format!("Bearer {}", creds.agent_secret).parse()?);

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

    // 检测本机可用的 CLI（返回 (cli名, 完整路径)）
    let cli_pairs = detect_available_clis();
    let available_clis: Vec<String> = cli_pairs.iter().map(|(name, _)| name.clone()).collect();
    if !available_clis.is_empty() {
        info!("🛠  检测到本地 CLI: {}", cli_pairs.iter().map(|(n, p)| format!("{} ({})", n, p)).collect::<Vec<_>>().join(", "));
    }
    // 将完整路径存到 runtime，供 run_cli_prompt 使用
    runtime.set_cli_paths(cli_pairs.clone()).await;

    // 发送 Register
    out_tx.send(ws_text(&AgentToServer::Register {
        agent_id: creds.agent_id.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        proto_version: PROTO_VERSION,
        allowed_clis: available_clis,
        allowed_cwds: vec![],
        owner_user_id: Some(creds.owner_user_id.clone()),
    }))?;
    runtime.set_connected(true, "已连接，贡献算力中").await;

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
    // 读取服务器消息；同时监听凭证变更（登录/登出）以便重连或断开
    let read_result: Result<()> = async {
        loop {
            let frame = tokio::select! {
                _ = runtime.wake.notified() => {
                    info!("凭证已变更，断开当前会话以应用新状态");
                    break;
                }
                frame = ws_read.next() => match frame {
                    Some(f) => f.map_err(|e| anyhow!("ws read: {e}"))?,
                    None => break,
                },
            };
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
                        ServerToAgent::CliPrompt { req_id, cli, extra_args, cwd, prompt } => {
                            info!("📝 CliPrompt: {} cli={}", req_id, cli);
                            let tx_c = out_tx_r.clone();
                            let rt_c = runtime.clone();
                            tokio::spawn(async move {
                                let full_path = rt_c.resolve_cli(&cli).await;
                                run_cli_prompt(req_id, &full_path, &cli, extra_args, cwd, prompt, tx_c).await;
                            });
                        }
                        ServerToAgent::Exec { task_id, cli, args, cwd, env } => {
                            info!("⚙️  Exec: {} {}", cli, args.join(" "));
                            let tx_c = out_tx_r.clone();
                            tokio::spawn(async move {
                                run_exec(task_id, cli, args, cwd, env, tx_c).await;
                            });
                        }
                        _ => {
                            // 其他消息类型暂不处理
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

async fn run_loop(runtime: Arc<NodeRuntime>) {
    let mut backoff = Duration::from_secs(2);
    loop {
        let creds = match runtime.creds().await {
            Some(c) => c,
            None => {
                runtime
                    .set_connected(false, "未登录：请在管理页登录后开始贡献算力")
                    .await;
                // 等待登录事件唤醒（带 2s 超时轮询，避免错过通知）
                let _ = tokio::time::timeout(Duration::from_secs(2), runtime.wake.notified()).await;
                continue;
            }
        };
        runtime.set_connected(false, "连接中…").await;
        match run_session(&runtime.cfg, &creds, &runtime).await {
            Ok(()) => {
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
    let persisted = load_persisted();
    let mut creds = initial_credentials(&persisted);

    // 有登录 token 但还没有节点凭证 → 自动注册一次
    if creds.is_none() {
        let token = std::env::var("NODE_USER_TOKEN")
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| persisted.user_token.clone());
        if let Some(tok) = token {
            info!("检测到登录 token，正在自动注册节点…");
            match provision_node(&cfg, &tok).await {
                Ok(c) => {
                    info!("✅ 节点已自动注册: {}", c.agent_id);
                    save_persisted(&PersistedState::from_creds(&c));
                    creds = Some(c);
                }
                Err(e) => warn!("自动注册失败（可在管理页重新登录）: {e:#}"),
            }
        }
    }

    match &creds {
        Some(c) => info!(
            "🚀 elon-node-agent {} 启动 (agent_id: {})",
            env!("CARGO_PKG_VERSION"),
            c.agent_id
        ),
        None => info!(
            "🚀 elon-node-agent {} 启动（未登录，请打开管理页 http://127.0.0.1:7799/ 登录）",
            env!("CARGO_PKG_VERSION")
        ),
    }
    info!("   云端: {}", cfg.cloud_url);
    info!("   Ollama: {}", cfg.ollama_url);
    info!("   积分价格: {} credits/1k tokens", cfg.price_per_1k);

    let runtime = Arc::new(NodeRuntime::new(cfg, creds));
    spawn_admin_server(runtime.clone());

    run_loop(runtime).await;
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
    creds: RwLock<Option<Credentials>>,
    status: RwLock<NodeStatus>,
    /// CLI 名称 → 完整路径映射（启动时检测，避免 PATH 不完整导致 program not found）
    cli_paths: RwLock<Vec<(String, String)>>,
    /// 凭证变更（登录/登出）时唤醒 run_loop / 当前会话
    wake: Notify,
}

impl NodeRuntime {
    fn new(cfg: NodeConfig, creds: Option<Credentials>) -> Self {
        Self {
            cfg,
            creds: RwLock::new(creds),
            status: RwLock::new(NodeStatus::default()),
            cli_paths: RwLock::new(Vec::new()),
            wake: Notify::new(),
        }
    }

    async fn creds(&self) -> Option<Credentials> {
        self.creds.read().await.clone()
    }

    async fn set_cli_paths(&self, paths: Vec<(String, String)>) {
        *self.cli_paths.write().await = paths;
    }

    /// CLI 名称 → 完整路径（找不到就返回原名称作备用）
    async fn resolve_cli(&self, name: &str) -> String {
        self.cli_paths.read().await
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, p)| p.clone())
            .unwrap_or_else(|| name.to_string())
    }

    /// 更新凭证（同时持久化），并唤醒连接循环重新评估。
    async fn set_creds(&self, c: Option<Credentials>) {
        match &c {
            Some(c) => save_persisted(&PersistedState::from_creds(c)),
            None => save_persisted(&PersistedState::default()),
        }
        *self.creds.write().await = c;
        self.wake.notify_waiters();
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
            .route("/api/env-check", axum::routing::get(admin_env_check))
            .route("/api/install-env", axum::routing::post(admin_install_env))
            .route("/api/save-openai-key", axum::routing::post(admin_save_openai_key))
            .route("/api/login", axum::routing::post(admin_login))
            .route("/api/logout", axum::routing::post(admin_logout))
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
    let creds = rt.creds().await;
    let st = rt.status.read().await;
    axum::Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "logged_in": creds.is_some(),
        "agent_id": creds.as_ref().map(|c| c.agent_id.clone()),
        "owner_user_id": creds.as_ref().map(|c| c.owner_user_id.clone()),
        "user_token_configured": creds.as_ref().map(|c| c.user_token.is_some()).unwrap_or(false),
        "cloud_url": rt.cfg.cloud_url,
        "cloud_http_url": rt.cfg.cloud_http_url,
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
struct AdminLoginReq {
    /// 账号（手机号/邮箱），搭配 password 登录
    account: Option<String>,
    password: Option<String>,
    /// 或直接粘贴已有的 elon 登录 token
    token: Option<String>,
}

/// 本地管理页 → 登录并自动注册节点。
/// 流程：账号+密码换 token（或直接用粘贴的 token）→ 调用云端注册节点拿 agent_id+secret → 持久化 → 唤醒重连。
async fn admin_login(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<AdminLoginReq>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use axum::http::StatusCode;

    // 1) 取得 token：优先直接粘贴的 token，否则账号+密码登录
    let token = if let Some(t) = req.token.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        t
    } else {
        let account = req.account.unwrap_or_default();
        let account = account.trim();
        let password = req.password.unwrap_or_default();
        if account.is_empty() || password.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "ok": false, "error": "请填写账号和密码，或直接粘贴 token" })),
            );
        }
        match cloud_login(&rt.cfg, account, &password).await {
            Ok(t) => t,
            Err(e) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({ "ok": false, "error": format!("登录失败: {e}") })),
                );
            }
        }
    };

    // 2) 用 token 注册/换取节点凭证
    match provision_node(&rt.cfg, &token).await {
        Ok(c) => {
            let agent_id = c.agent_id.clone();
            rt.set_creds(Some(c)).await;
            (
                StatusCode::OK,
                axum::Json(serde_json::json!({ "ok": true, "agent_id": agent_id })),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({ "ok": false, "error": format!("注册节点失败: {e}") })),
        ),
    }
}

/// 本地管理页 → 登出：清除本地凭证并断开。
async fn admin_logout(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    rt.set_creds(None).await;
    (axum::http::StatusCode::OK, axum::Json(serde_json::json!({ "ok": true })))
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

    // 2) 必须已登录（有凭证 + token）才能调用云端
    let creds = match rt.creds().await {
        Some(c) => c,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": "尚未登录，请先在页面顶部用账号密码登录。",
                })),
            );
        }
    };
    let token = match creds.user_token.as_ref() {
        Some(t) => t.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": "当前节点凭证不含登录 token，请在页面顶部重新登录。",
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
        "node_id": creds.agent_id,
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

// ── AI 编码工具 & Android 环境检查 / 安装 ────────────────────────────────────

/// 安装向导脚本（嵌入二进制，管理页触发时写到临时目录执行）
const SETUP_ENV_SCRIPT: &str = include_str!("../../scripts/setup-node-env.ps1");

/// 检查单个命令行工具是否可用（PATH + 常见安装目录双路扫描）。
fn tool_available(bin: &str) -> bool {
    let which_cmd = if cfg!(windows) { "where" } else { "which" };
    if std::process::Command::new(which_cmd)
        .arg(bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }
    // PATH 未命中时扫描常见安装目录
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let userprofile = std::env::var("USERPROFILE").unwrap_or_default();
        let dirs = [
            format!("{}\\npm", appdata),
            format!("{}\\Yarn\\bin", localappdata),
            format!("{}\\pnpm", appdata),
            format!("{}\\.volta\\bin", userprofile),
            "C:\\Program Files\\Git\\cmd".to_string(),
            "C:\\Program Files\\Git\\bin".to_string(),
            "C:\\Program Files\\nodejs".to_string(),
            "C:\\Program Files\\Ollama".to_string(),
            // JDK 常见路径
            "C:\\Program Files\\Eclipse Adoptium".to_string(),
            "C:\\Program Files\\Microsoft\\jdk-17.0.0.35-hotspot\\bin".to_string(),
        ];
        for dir in &dirs {
            for ext in &[".cmd", ".exe", ""] {
                let p = std::path::PathBuf::from(dir).join(format!("{}{}", bin, ext));
                if p.exists() { return true; }
            }
        }
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        for dir in &[
            "/usr/local/bin".to_string(),
            "/usr/bin".to_string(),
            format!("{}/.npm-global/bin", home),
            format!("{}/.local/bin", home),
            format!("{}/.volta/bin", home),
        ] {
            if std::path::Path::new(&format!("{}/{}", dir, bin)).exists() {
                return true;
            }
        }
    }
    false
}

/// 检查 Android SDK 是否配置好（platforms/android-34 + build-tools/34.0.0）。
fn android_sdk_ready() -> bool {
    let candidates: Vec<String> = [
        std::env::var("ANDROID_HOME").ok(),
        std::env::var("ANDROID_SDK_ROOT").ok(),
        // Windows 默认路径
        #[cfg(windows)]
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| format!("{}\\Android\\Sdk", p)),
        #[cfg(not(windows))]
        Some(format!(
            "{}/android-sdk",
            std::env::var("HOME").unwrap_or_default()
        )),
    ]
    .into_iter()
    .flatten()
    .collect();

    candidates.iter().any(|base| {
        std::path::Path::new(base)
            .join("platforms")
            .join("android-34")
            .exists()
    })
}

/// 检查 Gradle 阿里云镜像是否已配置。
fn gradle_mirror_ok() -> bool {
    let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .unwrap_or_default();
    let init = std::path::PathBuf::from(&home)
        .join(".gradle")
        .join("init.gradle");
    if !init.exists() { return false; }
    std::fs::read_to_string(&init)
        .map(|s| s.contains("maven.aliyun.com"))
        .unwrap_or(false)
}

/// GET /api/env-check — 返回各工具安装状态。
async fn admin_env_check(
    axum::extract::State(_rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(|| {
        serde_json::json!({
            "git":          tool_available("git"),
            "java":         tool_available("java"),
            "node":         tool_available("node"),
            "npm":          tool_available("npm"),
            "codex":        tool_available("codex"),
            "android_sdk":  android_sdk_ready(),
            "gradle_mirror": gradle_mirror_ok(),
            "ollama":       tool_available("ollama"),
            "openai_key":   std::env::var("OPENAI_API_KEY")
                                .map(|k| k.starts_with("sk-"))
                                .unwrap_or(false),
        })
    })
    .await
    .unwrap_or_else(|_| serde_json::json!({}));
    axum::Json(result)
}

/// POST /api/install-env — Windows 上弹出安装向导窗口。
async fn admin_install_env(
    axum::extract::State(_rt): axum::extract::State<Arc<NodeRuntime>>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use axum::http::StatusCode;

    #[cfg(windows)]
    {
        let tmp = std::env::temp_dir().join("elon-setup-node-env.ps1");
        if let Err(e) = std::fs::write(&tmp, SETUP_ENV_SCRIPT) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": format!("写入临时脚本失败: {e}")
                })),
            );
        }
        // 优先使用 exe 同目录的脚本（可能是更新版）
        let script_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("setup-node-env.ps1")))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| tmp.to_string_lossy().to_string());

        match std::process::Command::new("powershell")
            .args(["-ExecutionPolicy", "Bypass", "-File", &script_path])
            .spawn()
        {
            Ok(_) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "ok": true,
                    "msg": "安装脚本已在新窗口启动，按提示操作完成后刷新本页查看结果"
                })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": format!("启动脚本失败: {e}")
                })),
            ),
        }
    }
    #[cfg(not(windows))]
    {
        (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": "自动安装向导仅限 Windows。Linux 用户请手动执行：\nbash scripts/setup-node-env.sh\n（或参照文档手动安装 git / jdk17 / node / codex / android-sdk）"
            })),
        )
    }
}

#[derive(Deserialize)]
struct SaveOpenAiKeyReq {
    api_key: String,
}

/// POST /api/save-openai-key — 保存 OPENAI_API_KEY 到进程环境变量及 node-agent.env。
async fn admin_save_openai_key(
    axum::extract::State(_rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<SaveOpenAiKeyReq>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use axum::http::StatusCode;

    let key = req.api_key.trim().to_string();
    if key.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "ok": false, "error": "API Key 不能为空" })),
        );
    }
    if !key.starts_with("sk-") {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": "API Key 格式不正确，需以 sk- 开头"
            })),
        );
    }

    // 当前进程立即生效（Codex CLI 子进程会继承）
    std::env::set_var("OPENAI_API_KEY", &key);

    // 持久化到 exe 同目录的 node-agent.env
    if let Some(env_file) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("node-agent.env")))
    {
        upsert_env_file(&env_file, "OPENAI_API_KEY", &key);
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "ok": true,
            "msg": "OPENAI_API_KEY 已保存，Codex CLI 立即可用"
        })),
    )
}

/// 更新或追加 key=value 到 .env 文件（注释行也会被激活）。
fn upsert_env_file(path: &std::path::Path, key: &str, value: &str) {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let prefix = format!("{}=", key);
    let mut found = false;
    let new_lines: Vec<String> = existing
        .lines()
        .map(|line| {
            let stripped = line.trim_start_matches('#').trim_start();
            if stripped.starts_with(&prefix) {
                found = true;
                format!("{}={}", key, value)
            } else {
                line.to_string()
            }
        })
        .collect();

    let mut content = new_lines.join("\n");
    if !found {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("{}={}\n", key, value));
    } else if !content.ends_with('\n') {
        content.push('\n');
    }
    let _ = std::fs::write(path, content);
}
