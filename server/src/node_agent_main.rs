// server/src/node_agent_main.rs

#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
use futures::{SinkExt, StreamExt};
use homecli_proto::{
    AgentToServer, CliWorkspaceStatus, ModelCapability, NodeHardwareProfile, ServerToAgent,
    PROTO_VERSION,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, watch, Notify, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

use node_agent_cli_done::{
    cli_done_message, cli_prompt_accepted, duplicate_cli_prompt_done, latest_codex_session_id,
};
use node_agent_cli_env::apply_env;
use node_agent_env::{env_flag, node_agent_env_file_path};
use node_agent_registration::provision_node;

const CLOUD_WS_READ_TIMEOUT: Duration = Duration::from_secs(35);

mod agent_runtime_error_summary;
mod cli_usage;
#[allow(dead_code)]
mod errors;
mod git_command_error;
mod node_agent_active_task;
mod node_agent_active_task_registry;
mod node_agent_admin_open;
mod node_agent_admin_status;
mod node_agent_api_runtime_config;
mod node_agent_api_runtime_tools;
mod node_agent_cli_done;
mod node_agent_cli_env;
#[cfg(test)]
mod node_agent_cli_prompt_timeout_tests;
mod node_agent_cli_pty;
mod node_agent_cli_security;
mod node_agent_cli_session_bridge;
mod node_agent_cli_session_bridge_capabilities;
mod node_agent_cli_sidecar;
mod node_agent_cli_sidecar_admin;
mod node_agent_cli_sidecar_io;
mod node_agent_cli_sidecar_runner;
#[cfg(test)]
mod node_agent_cli_sidecar_runner_tests;
mod node_agent_client_diagnostic_logs;
mod node_agent_client_diagnostics;
mod node_agent_client_install_status;
mod node_agent_client_maintenance;
mod node_agent_cloud_net;
mod node_agent_codex_approval;
mod node_agent_codex_session;
mod node_agent_codex_vault; mod node_agent_codex_vault_active; mod node_agent_codex_child_env; mod node_agent_codex_vault_emergency;
mod node_agent_download_router;
mod node_agent_env;
mod node_agent_file_info;
mod node_agent_file_range;
mod node_agent_full_access;
mod node_agent_install_env;
mod node_agent_lifecycle;
mod node_agent_local_admin;
mod node_agent_local_pc_frontend;
mod node_agent_program_resolver;
mod node_agent_project_agent_recovery;
mod node_agent_project_agent_runs;
mod node_agent_project_manifest_identity;
mod node_agent_project_picker;
mod node_agent_project_profile;
mod node_agent_project_profile_node;
mod node_agent_project_profile_python;
mod node_agent_proxy;
mod node_agent_registration;
mod node_agent_route_c_status;
mod node_agent_runtime_approval;
mod node_agent_runtime_events;
mod node_agent_server_runtime;
#[cfg(test)]
mod node_agent_task_approval_cleanup_tests;
mod node_agent_task_approval_snapshot;
mod node_agent_task_journal;
mod node_agent_task_journal_api;
mod node_agent_task_journal_events;
mod node_agent_task_journal_lock;
#[cfg(test)]
mod node_agent_task_journal_recovery_tests;
#[cfg(test)]
mod node_agent_task_lifecycle_pressure_tests;
mod node_agent_task_resume;
mod node_agent_task_resume_sidecar;
#[cfg(test)]
mod node_agent_task_resume_sidecar_tests;
mod node_agent_tool_approval;
mod node_agent_tool_guard;
mod node_agent_workspace_match;
mod node_agent_workspace_modules;
mod node_agent_write_preview;
mod node_agent_ws_control_queue;
#[cfg(windows)]
mod node_client_launcher;
mod node_hardware_probe;
mod pc_storage_git_http;
mod pc_storage_repo;
mod pc_workspace_git_remote;
mod pc_workspace_provisioner;
mod project_default_docs;
mod project_docs_scan;
mod project_git_worktree_audit;
mod project_landing;
mod project_workspace_inspect;
mod tools_patch;
mod windows_doctor;

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
    install_id: Option<String>,
    agent_id: Option<String>,
    agent_secret: Option<String>,
    owner_user_id: Option<String>,
    user_token: Option<String>,
    storage_enabled: Option<bool>,
    storage_root: Option<String>,
    storage_git_base_url: Option<String>,
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
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
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

fn ensure_install_id(persisted: &mut PersistedState) -> String {
    if let Some(existing) = persisted
        .install_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return existing.to_string();
    }
    let install_id = format!("ins_{}", uuid::Uuid::new_v4().simple());
    persisted.install_id = Some(install_id.clone());
    install_id
}

impl PersistedState {
    fn from_parts(
        install_id: &str,
        c: Option<&Credentials>,
        storage: &pc_storage_repo::StorageSettings,
    ) -> Self {
        Self {
            install_id: Some(install_id.to_string()),
            agent_id: c.map(|c| c.agent_id.clone()),
            agent_secret: c.map(|c| c.agent_secret.clone()),
            owner_user_id: c.map(|c| c.owner_user_id.clone()),
            user_token: c.and_then(|c| c.user_token.clone()),
            storage_enabled: Some(storage.enabled),
            storage_root: storage.root_path.clone(),
            storage_git_base_url: storage.git_base_url.clone(),
        }
    }
}

/// 从环境变量 / 持久化文件解析已有凭证；都没有时返回 None（需登录）。
/// 环境变量优先（供高级用户/服务器覆盖），否则用上次持久化的结果。
fn initial_credentials(persisted: &PersistedState) -> Option<Credentials> {
    let env_nonempty = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let agent_id = env_nonempty("NODE_AGENT_ID").or_else(|| persisted.agent_id.clone())?;
    let agent_secret =
        env_nonempty("NODE_AGENT_SECRET").or_else(|| persisted.agent_secret.clone())?;
    let owner_user_id = env_nonempty("NODE_OWNER_USER_ID")
        .or_else(|| persisted.owner_user_id.clone())
        .unwrap_or_default();
    let user_token = env_nonempty("NODE_USER_TOKEN").or_else(|| persisted.user_token.clone());
    Some(Credentials {
        agent_id,
        agent_secret,
        owner_user_id,
        user_token,
    })
}

fn initial_storage_settings(persisted: &PersistedState) -> pc_storage_repo::StorageSettings {
    let env_nonempty = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    let root_path = env_nonempty("NODE_STORAGE_ROOT")
        .or_else(|| env_nonempty("ELON_STORAGE_ROOT"))
        .or_else(|| persisted.storage_root.clone());
    let git_base_url = env_nonempty("NODE_STORAGE_GIT_BASE_URL")
        .or_else(|| env_nonempty("ELON_STORAGE_GIT_BASE_URL"))
        .or_else(|| persisted.storage_git_base_url.clone());
    let enabled = env_flag("NODE_STORAGE_ENABLED")
        .or_else(|| env_flag("ELON_STORAGE_ENABLED"))
        .or(persisted.storage_enabled)
        .unwrap_or(false);
    pc_storage_repo::StorageSettings {
        enabled,
        root_path: root_path.or_else(|| {
            enabled.then(|| {
                pc_storage_repo::default_storage_root()
                    .to_string_lossy()
                    .to_string()
            })
        }),
        git_base_url,
    }
}

/// 账号 + 密码登录云端，换取 token。
async fn cloud_login(cfg: &NodeConfig, account: &str, password: &str) -> Result<String> {
    let url = format!(
        "{}/api/auth/login",
        cfg.cloud_http_url.trim_end_matches('/')
    );
    let client = node_agent_cloud_net::direct_cloud_client_or_default(Duration::from_secs(15));
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
        let cloud_http_url =
            std::env::var("NODE_CLOUD_HTTP_URL").unwrap_or_else(|_| derive_http_url(&cloud_url));
        let ollama_url =
            std::env::var("NODE_OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".into());
        let lm_studio_url = std::env::var("NODE_LM_STUDIO_URL")
            .ok()
            .filter(|v| !v.is_empty());
        let custom_url = std::env::var("NODE_CUSTOM_LLM_URL")
            .ok()
            .filter(|v| !v.is_empty());
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
        while let Some(pos) = buf.find('\n') {
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
                if let Some(content) = val.pointer("/message/content").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        completion_tokens += 1;
                        let _ = out_tx.send(ws_text(&AgentToServer::LlmStreamChunk {
                            req_id: req_id.clone(),
                            delta: content.to_string(),
                        }));
                    }
                }
                // 完成信号
                if let Some(r) = val
                    .pointer("/choices/0/finish_reason")
                    .and_then(|v| v.as_str())
                {
                    if !r.is_empty() && r != "null" {
                        finish_reason = r.to_string();
                    }
                }
                if val.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                    prompt_tokens = val
                        .pointer("/prompt_eval_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    completion_tokens =
                        val.pointer("/eval_count")
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
                        .unwrap_or(completion_tokens as u64)
                        as u32;
                }
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

fn truncate_cli_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let tail: String = trimmed
        .chars()
        .rev()
        .take(max_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("...{}", tail)
}

fn cli_done_error(cli_name: &str, stdout_text: &str, stderr_text: &str) -> String {
    let mut parts = vec![format!("{cli_name} 进程退出失败")];
    let stderr = truncate_cli_text(stderr_text, 2000);
    if !stderr.is_empty() {
        parts.push(format!("stderr:\n{stderr}"));
    }
    let stdout = truncate_cli_text(stdout_text, 1200);
    if !stdout.is_empty() {
        parts.push(format!("stdout:\n{stdout}"));
    }
    parts.join("\n\n")
}

// ── CLI 执行（CliPrompt / Exec）────────────────────────────────────────────────

const CLI_PROBE_STALE_MS: u128 = 30_000;
const CODEX_RUN_CHECK_TIMEOUT: Duration = Duration::from_millis(900);
const GENERIC_CLI_RUN_CHECK_TIMEOUT: Duration = Duration::from_millis(700);

#[derive(Clone, Debug, Serialize)]
struct LocalCliToolStatus {
    name: String,
    label: &'static str,
    path: Option<String>,
    version: Option<String>,
    installed: bool,
    runnable: bool,
    logged_in: Option<bool>,
    available: bool,
    status: String,
    detail: Option<String>,
    reason: Option<String>,
    diagnosis: Option<String>,
    fix_hint: Option<String>,
    fix_action: String,
    backend: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct LocalCliProbeSnapshot {
    refreshed_at_ms: Option<u128>,
    tools: Vec<LocalCliToolStatus>,
}

impl Default for LocalCliProbeSnapshot {
    fn default() -> Self {
        Self {
            refreshed_at_ms: None,
            tools: ["codex", "copilot", "claude", "gemini"]
                .into_iter()
                .map(|name| LocalCliToolStatus {
                    name: name.to_string(),
                    label: local_cli_display_label(name),
                    path: None,
                    version: None,
                    installed: false,
                    runnable: false,
                    logged_in: if name == "codex" { Some(false) } else { None },
                    available: false,
                    status: "checking".to_string(),
                    detail: Some("正在后台检测，不阻塞 Win 端启动".to_string()),
                    reason: Some("checking".to_string()),
                    diagnosis: None,
                    fix_hint: None,
                    fix_action: "wait".to_string(),
                    backend: "cli",
                })
                .collect(),
        }
    }
}

impl LocalCliProbeSnapshot {
    fn available_pairs(&self) -> Vec<(String, String)> {
        self.tools
            .iter()
            .filter(|tool| tool.available)
            .filter_map(|tool| {
                tool.path
                    .as_ref()
                    .map(|path| (tool.name.clone(), path.clone()))
            })
            .collect()
    }

    fn available_names(&self) -> Vec<String> {
        self.available_pairs()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    fn codex_status(&self) -> Option<LocalCliToolStatus> {
        self.tools.iter().find(|tool| tool.name == "codex").cloned()
    }

    fn is_stale(&self) -> bool {
        self.refreshed_at_ms
            .map(|ms| now_epoch_ms().saturating_sub(ms) > CLI_PROBE_STALE_MS)
            .unwrap_or(true)
    }
}

/// 检测本机可用的 CLI，返回 (cli名称, 完整路径) 对。
fn local_cli_display_label(name: &str) -> &'static str {
    match name.trim().to_ascii_lowercase().as_str() {
        "codex" => "Codex",
        "copilot" => "Copilot",
        "claude" => "Claude",
        "gemini" => "Gemini",
        _ => "本机 AI CLI",
    }
}

fn probe_local_clis() -> LocalCliProbeSnapshot {
    let tools = ["codex", "copilot", "claude", "gemini"]
        .into_iter()
        .map(probe_local_cli)
        .collect();
    LocalCliProbeSnapshot {
        refreshed_at_ms: Some(now_epoch_ms()),
        tools,
    }
}

fn probe_local_cli(name: &str) -> LocalCliToolStatus {
    let best_path = best_cli_path(name);
    match name {
        "codex" => probe_codex_cli(best_path),
        _ => probe_generic_cli(name, best_path),
    }
}

fn probe_generic_cli(name: &str, best_path: Option<PathBuf>) -> LocalCliToolStatus {
    let label = local_cli_display_label(name);
    let Some(path) = best_path else {
        return LocalCliToolStatus {
            name: name.to_string(),
            label,
            path: None,
            version: None,
            installed: false,
            runnable: false,
            logged_in: None,
            available: false,
            status: "not_installed".to_string(),
            detail: Some(format!("{label} CLI 未安装或不在 PATH 中")),
            reason: Some("not_found".to_string()),
            diagnosis: Some(format!("未在 PATH 和常见安装目录中找到 {label} 命令。")),
            fix_hint: Some("请安装对应 CLI，或把它的 bin 目录加入 PATH 后重新检测。".to_string()),
            fix_action: "install".to_string(),
            backend: "cli",
        };
    };
    let run = quick_command_status(&path, &["--version"], GENERIC_CLI_RUN_CHECK_TIMEOUT);
    let runnable = run.success || run.timed_out;
    LocalCliToolStatus {
        name: name.to_string(),
        label,
        path: Some(path.to_string_lossy().to_string()),
        version: if runnable { run.summary.clone() } else { None },
        installed: true,
        runnable,
        logged_in: None,
        available: runnable,
        status: if runnable { "ready" } else { "not_runnable" }.to_string(),
        detail: if runnable {
            run.summary
                .or_else(|| Some(format!("{label} CLI 已检测到")))
        } else {
            Some(
                run.summary
                    .unwrap_or_else(|| format!("{label} CLI 无法执行")),
            )
        },
        reason: if runnable {
            None
        } else {
            run.reason.or_else(|| Some("run_failed".to_string()))
        },
        diagnosis: if runnable {
            Some(format!("{label} CLI 可由 Win 端启动。"))
        } else {
            Some(format!("检测到 {label} 命令路径，但 Win 端无法启动它。"))
        },
        fix_hint: if runnable {
            None
        } else {
            Some("请修复该 CLI 安装或 PATH 后重新检测。".to_string())
        },
        fix_action: if runnable { "none" } else { "repair_path" }.to_string(),
        backend: "cli",
    }
}

fn probe_codex_cli(best_path: Option<PathBuf>) -> LocalCliToolStatus {
    let label = local_cli_display_label("codex");
    let Some(path) = best_path else {
        return LocalCliToolStatus {
            name: "codex".to_string(),
            label,
            path: None,
            version: None,
            installed: false,
            runnable: false,
            logged_in: Some(false),
            available: false,
            status: "not_installed".to_string(),
            detail: Some(
                "未检测到可运行的 Codex CLI；只安装 Codex 桌面端不一定会提供可调用的 codex 命令"
                    .to_string(),
            ),
            reason: Some("not_found".to_string()),
            diagnosis: Some(
                "没有找到可作为命令行工具启动的 Codex CLI。Codex 桌面端和 Codex CLI 是两层能力，桌面端不一定暴露可调用的 codex 命令。"
                    .to_string(),
            ),
            fix_hint: Some(
                "点击安装/修复，让 Win 端运行 OpenAI 官方 Windows 安装器；安装后重新检测。".to_string(),
            ),
            fix_action: "install".to_string(),
            backend: "cli",
        };
    };

    let run = quick_command_status(&path, &["--version"], CODEX_RUN_CHECK_TIMEOUT);
    if !run.success {
        return LocalCliToolStatus {
            name: "codex".to_string(),
            label,
            path: Some(path.to_string_lossy().to_string()),
            version: None,
            installed: true,
            runnable: false,
            logged_in: Some(false),
            available: false,
            status: "not_runnable".to_string(),
            detail: Some(run.summary.clone().unwrap_or_else(|| {
                "检测到 codex 命令，但无法非交互执行；请安装 Codex CLI 或修复 PATH".to_string()
            })),
            reason: run
                .reason
                .clone()
                .or_else(|| Some("run_failed".to_string())),
            diagnosis: Some(codex_not_runnable_diagnosis(&path, run.reason.as_deref())),
            fix_hint: Some(codex_not_runnable_fix_hint(&path)),
            fix_action: "repair_path".to_string(),
            backend: "cli",
        };
    }

    let auth = codex_auth_configured();
    LocalCliToolStatus {
        name: "codex".to_string(),
        label,
        path: Some(path.to_string_lossy().to_string()),
        version: run.summary.clone(),
        installed: true,
        runnable: true,
        logged_in: Some(auth),
        available: auth,
        status: if auth { "ready" } else { "not_logged_in" }.to_string(),
        detail: if auth {
            Some("Codex CLI 可运行，且已检测到 API key 或本机 Codex 登录文件".to_string())
        } else {
            Some("Codex CLI 可运行，但未检测到 OPENAI_API_KEY 或本机 Codex 登录文件".to_string())
        },
        reason: if auth {
            None
        } else {
            Some("not_logged_in".to_string())
        },
        diagnosis: if auth {
            Some("Win 端已找到可运行的 Codex CLI，并检测到本机鉴权。".to_string())
        } else {
            Some("Codex CLI 本身能启动，但当前用户还没有可用的 Codex/OpenAI 鉴权。".to_string())
        },
        fix_hint: if auth {
            None
        } else {
            Some("请在此页保存 OpenAI API Key，或先在本机完成 Codex CLI 登录。".to_string())
        },
        fix_action: if auth { "none" } else { "login" }.to_string(),
        backend: "cli",
    }
}

fn best_cli_path(name: &str) -> Option<PathBuf> {
    let candidates_paths: Vec<PathBuf> = elon_pc_dev_runtime::command_candidates(name);
    if candidates_paths.is_empty() {
        return None;
    }

    let not_vscode = |p: &&PathBuf| {
        let lower = p.to_string_lossy().to_ascii_lowercase();
        !lower.contains("globalstorage") && !lower.contains("copilotcli\\copilot")
    };

    #[cfg(windows)]
    let best = candidates_paths
        .iter()
        .find(|p| p.to_string_lossy().to_ascii_lowercase().ends_with(".cmd") && not_vscode(p))
        .or_else(|| {
            candidates_paths
                .iter()
                .find(|p| p.to_string_lossy().to_ascii_lowercase().ends_with(".cmd"))
        })
        .or_else(|| candidates_paths.iter().find(not_vscode))
        .or_else(|| candidates_paths.first());

    #[cfg(not(windows))]
    let best = candidates_paths
        .iter()
        .find(not_vscode)
        .or_else(|| candidates_paths.first());

    best.cloned()
}

struct QuickCommandStatus {
    success: bool,
    timed_out: bool,
    summary: Option<String>,
    reason: Option<String>,
}

fn quick_command_status(program: &Path, args: &[&str], timeout: Duration) -> QuickCommandStatus {
    let mut command = elon_pc_dev_runtime::command_from_path(program);
    command
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let reason = if error.kind() == std::io::ErrorKind::PermissionDenied {
                "permission_denied"
            } else {
                "spawn_failed"
            };
            return QuickCommandStatus {
                success: false,
                timed_out: false,
                summary: Some(format!("无法启动 {}：{error}", program.display())),
                reason: Some(reason.to_string()),
            };
        }
    };
    let started_at = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return match child.wait_with_output() {
                    Ok(output) => QuickCommandStatus {
                        success: output.status.success(),
                        timed_out: false,
                        summary: first_cli_output_line(&output.stdout, &output.stderr).or_else(
                            || {
                                (!output.status.success()).then(|| {
                                    format!(
                                        "{} 退出码 {:?}",
                                        program.display(),
                                        output.status.code()
                                    )
                                })
                            },
                        ),
                        reason: (!output.status.success()).then(|| "exit_failed".to_string()),
                    },
                    Err(error) => QuickCommandStatus {
                        success: false,
                        timed_out: false,
                        summary: Some(format!("读取 {} 输出失败：{error}", program.display())),
                        reason: Some("output_failed".to_string()),
                    },
                };
            }
            Ok(None) if started_at.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return QuickCommandStatus {
                    success: false,
                    timed_out: true,
                    summary: Some(format!("{} 检测超时", program.display())),
                    reason: Some("timeout".to_string()),
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return QuickCommandStatus {
                    success: false,
                    timed_out: false,
                    summary: Some(format!("检测 {} 失败：{error}", program.display())),
                    reason: Some("run_failed".to_string()),
                };
            }
        }
    }
}

fn codex_not_runnable_diagnosis(path: &Path, reason: Option<&str>) -> String {
    if is_codex_desktop_resource_path(path) {
        return "检测到的是 Codex 桌面端安装包里的受保护资源路径。Windows 可以启动桌面 App，但通常不允许一龙 Win 端直接把这个资源文件当命令行 CLI 调用。"
            .to_string();
    }
    match reason {
        Some("permission_denied") => {
            "Windows 拒绝启动这个 codex 路径；常见原因是 PATH 指到了受保护应用包、权限异常或安装残缺。".to_string()
        }
        Some("timeout") => {
            "codex --version 在短时间内没有返回；可能是命令卡在初始化、杀毒拦截或安装损坏。".to_string()
        }
        Some("exit_failed") => {
            "codex 命令能启动，但版本检测返回失败；通常是 CLI 安装损坏或依赖环境异常。".to_string()
        }
        _ => "检测到 codex 命令路径，但 Win 端无法用非交互方式启动它。".to_string(),
    }
}

fn codex_not_runnable_fix_hint(path: &Path) -> String {
    if is_codex_desktop_resource_path(path) {
        return "请点击安装/修复 Codex，让 Win 端运行 OpenAI 官方 Windows 安装器；如果已安装，请确保本地 Codex CLI 的 bin 目录排在 WindowsApps 桌面资源路径之前。"
            .to_string();
    }
    "请点击安装/修复 Codex，或重新安装 Codex CLI 后再检测。".to_string()
}

fn is_codex_desktop_resource_path(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("\\windowsapps\\")
        && lower.contains("\\openai.codex_")
        && lower.contains("\\app\\resources\\")
}

fn first_cli_output_line(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    [stdout, stderr].into_iter().find_map(|bytes| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|line| line.chars().take(240).collect())
    })
}

fn codex_auth_configured() -> bool {
    let api_runtime = node_agent_api_runtime_config::status_from_env();
    if api_runtime.key_configured || env_key_present("CODEX_API_KEY") {
        return true;
    }
    codex_home_candidates()
        .into_iter()
        .any(|home| codex_auth_file_present(&home))
}

fn env_key_present(name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn codex_home_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(home) = std::env::var("CODEX_HOME") {
        push_unique_path(&mut candidates, PathBuf::from(home));
    }
    for key in ["USERPROFILE", "HOME"] {
        if let Ok(home) = std::env::var(key) {
            push_unique_path(&mut candidates, PathBuf::from(home).join(".codex"));
        }
    }
    candidates
}

fn codex_auth_file_present(home: &Path) -> bool {
    if ["auth.json", "credentials.json"]
        .into_iter()
        .map(|name| home.join(name))
        .any(|path| non_empty_file(&path))
    {
        return true;
    }

    let config = home.join("config.toml");
    if !non_empty_file(&config) {
        return false;
    }
    std::fs::read_to_string(&config)
        .map(|body| {
            let lower = body.to_ascii_lowercase();
            lower.contains("api_key") || lower.contains("openai_api_key")
        })
        .unwrap_or(false)
}

fn non_empty_file(path: &Path) -> bool {
    path.exists()
        && std::fs::metadata(path)
            .map(|meta| meta.len() > 2)
            .unwrap_or(false)
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.as_os_str().is_empty() {
        return;
    }
    let key = path.to_string_lossy().to_ascii_lowercase();
    if !paths
        .iter()
        .any(|item| item.to_string_lossy().to_ascii_lowercase() == key)
    {
        paths.push(path);
    }
}

fn now_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

/// 将附件 URL 下载到本地临时文件，并转换成对应 CLI 参数。
async fn resolve_attachment_args(
    args: Vec<String>,
    cli_name: &str,
    user_token: Option<&str>,
) -> Vec<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();
    let mut result = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--attachment" {
            if let Some(url) = args.get(i + 1) {
                if url.starts_with("http://") || url.starts_with("https://") {
                    let ext = url
                        .rsplit('.')
                        .next()
                        .filter(|e| e.len() <= 5 && e.chars().all(|c| c.is_alphanumeric()))
                        .unwrap_or("jpg");
                    let tmp_path = std::env::temp_dir().join(format!(
                        "elon_attach_{}.{}",
                        uuid::Uuid::new_v4(),
                        ext
                    ));
                    let mut req = client.get(url.as_str());
                    if let Some(tok) = user_token {
                        req = req.bearer_auth(tok);
                    }
                    match req.send().await {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(bytes) = resp.bytes().await {
                                if tokio::fs::write(&tmp_path, &bytes).await.is_ok() {
                                    let local = tmp_path.to_string_lossy().to_string();
                                    if cli_name == "codex" {
                                        // Codex 用 -i 传图片
                                        result.push("-i".to_string());
                                        result.push(local);
                                    } else {
                                        // Copilot 用 --attachment
                                        result.push("--attachment".to_string());
                                        result.push(local);
                                    }
                                    i += 2;
                                    continue;
                                }
                            }
                        }
                        Ok(resp) => {
                            warn!("📎 attachment download failed: status={}", resp.status());
                        }
                        Err(e) => {
                            warn!("📎 attachment download error: {}", e);
                        }
                    }
                    i += 2;
                    continue;
                }
            }
        }
        result.push(args[i].clone());
        i += 1;
    }
    result
}

fn cli_prompt_full_access(runtime_permission: Option<&str>) -> bool {
    matches!(
        runtime_permission.map(str::trim),
        Some("full_access" | "danger_full_access")
    )
}

fn cli_prompt_read_only(runtime_permission: Option<&str>) -> bool {
    !matches!(
        runtime_permission.map(str::trim),
        Some("project_write" | "full_access" | "danger_full_access")
    )
}

fn cli_prompt_timeout_secs(cli_name: &str, runtime_permission: Option<&str>) -> u64 {
    match cli_name.trim().to_ascii_lowercase().as_str() {
        "codex" if cli_prompt_full_access(runtime_permission) => 1200,
        "codex" => 300,
        _ => 180,
    }
}

struct CliPromptRun {
    req_id: String,
    bin: String,
    cli_name: String,
    extra_args: Vec<String>,
    runtime_permission: Option<String>,
    cwd: Option<String>,
    conversation_workspace: Option<pc_workspace_provisioner::ConversationWorkspaceResult>,
    prompt: String,
    server_runtime_config: Option<crate::node_agent_server_runtime::ServerRuntimeConfig>,
    approval_state: node_agent_tool_approval::ToolApprovalState,
    task_journal: node_agent_task_journal::TaskJournal,
    runtime: Arc<NodeRuntime>,
    cancel_rx: watch::Receiver<bool>,
    out_tx: tokio::sync::mpsc::UnboundedSender<Message>,
    codex_vault_switch_attempted: bool,
}

async fn run_cli_prompt(run: CliPromptRun) {
    use tokio::io::AsyncBufReadExt;

    let CliPromptRun {
        req_id,
        bin,
        cli_name,
        extra_args,
        runtime_permission,
        cwd,
        conversation_workspace,
        prompt,
        server_runtime_config,
        approval_state,
        task_journal,
        runtime,
        mut cancel_rx,
        out_tx,
        codex_vault_switch_attempted,
    } = run;
    let bin_owned = bin;
    let cli_name_owned = cli_name;
    let bin = bin_owned.as_str();
    let cli_name = cli_name_owned.as_str();
    if let Err(error) =
        node_agent_cli_security::validate_cli_extra_args(cli_name, extra_args.as_slice())
    {
        let message = error.to_string();
        record_cli_done_outcome(&task_journal, &req_id, false, Some(&message));
        let _ = out_tx.send(ws_text(&AgentToServer::CliDone {
            req_id,
            exit_ok: false,
            error: Some(message),
            session_id: None,
            prompt_tokens: None,
            cached_input_tokens: None,
            completion_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            model: None,
            workspace_status: None,
        }));
        return;
    }
    if cli_name == "api-runtime" {
        let result = crate::node_agent_server_runtime::run_api_runtime_prompt(
            crate::node_agent_server_runtime::RuntimePromptOptions {
                req_id: &req_id,
                cwd: cwd.as_deref(),
                runtime_permission: runtime_permission.as_deref(),
                prompt: &prompt,
                approval_state: Some(approval_state.clone()),
                cancel_rx,
                out_tx: out_tx.clone(),
                task_journal: Some(task_journal.clone()),
            },
        )
        .await;
        let (exit_ok, error, workspace_status) =
            finalize_cli_prompt_workspace(result.exit_ok, result.error, conversation_workspace);
        record_cli_done_outcome(&task_journal, &req_id, exit_ok, error.as_deref());
        let _ = out_tx.send(ws_text(&AgentToServer::CliDone {
            req_id,
            exit_ok,
            error,
            session_id: None,
            prompt_tokens: result.prompt_tokens,
            cached_input_tokens: None,
            completion_tokens: result.completion_tokens,
            reasoning_tokens: None,
            total_tokens: result.total_tokens,
            model: result.model,
            workspace_status,
        }));
        return;
    }
    if cli_name == "server-runtime" {
        let result = match server_runtime_config {
            Some(config) => {
                crate::node_agent_server_runtime::run_server_runtime_prompt(
                    config,
                    crate::node_agent_server_runtime::RuntimePromptOptions {
                        req_id: &req_id,
                        cwd: cwd.as_deref(),
                        runtime_permission: runtime_permission.as_deref(),
                        prompt: &prompt,
                        approval_state: Some(approval_state.clone()),
                        cancel_rx,
                        out_tx: out_tx.clone(),
                        task_journal: Some(task_journal.clone()),
                    },
                )
                .await
            }
            None => crate::node_agent_server_runtime::ServerRuntimeRunResult {
                exit_ok: false,
                error: Some("server-runtime 缺少节点登录上下文".to_string()),
                model: Some("server-runtime".to_string()),
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
            },
        };
        let (exit_ok, error, workspace_status) =
            finalize_cli_prompt_workspace(result.exit_ok, result.error, conversation_workspace);
        record_cli_done_outcome(&task_journal, &req_id, exit_ok, error.as_deref());
        let _ = out_tx.send(ws_text(&AgentToServer::CliDone {
            req_id,
            exit_ok,
            error,
            session_id: None,
            prompt_tokens: result.prompt_tokens,
            cached_input_tokens: None,
            completion_tokens: result.completion_tokens,
            reasoning_tokens: None,
            total_tokens: result.total_tokens,
            model: result.model,
            workspace_status,
        }));
        return;
    }
    let batch_wrapper = node_agent_cli_security::windows_batch_wrapper(bin);
    let actual_bin = batch_wrapper
        .as_ref()
        .map(|(program, _)| *program)
        .unwrap_or(bin);
    let full_access = cli_prompt_full_access(runtime_permission.as_deref());
    let codex_sessions_file = std::env::temp_dir().join("elon_codex_sessions.json");
    let codex_scope_key = if cli_name == "codex" {
        node_agent_cli_security::codex_session_scope_key(
            &extra_args,
            runtime_permission.as_deref(),
            cwd.as_deref(),
        )
    } else {
        None
    };
    let codex_plan = if cli_name == "codex" {
        node_agent_codex_session::load_session_plan(
            &task_journal,
            &codex_sessions_file,
            codex_scope_key.clone(),
        )
    } else {
        node_agent_codex_session::CodexSessionPlan {
            scope_key: None,
            session_id: None,
        }
    };
    let mut cmd = tokio::process::Command::new(actual_bin);
    let mut sidecar_args = Vec::new();
    let mut sidecar_env = Vec::new();
    let codex_last_message_path = if cli_name == "codex" {
        let safe_req_id: String = req_id
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        let path = std::env::temp_dir().join(format!("elon_codex_last_message_{safe_req_id}.txt"));
        let _ = std::fs::remove_file(&path);
        Some(path)
    } else {
        None
    };
    if let Some((_, args)) = batch_wrapper.as_ref() {
        cmd.args(args);
        sidecar_args.extend(args.iter().map(|arg| arg.to_string()));
    }
    if cli_name == "codex" {
        for a in &extra_args {
            if let Some(model) = a.strip_prefix("--codex-model=") {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "-m");
                push_tracked_arg(&mut cmd, &mut sidecar_args, model);
            } else if let Some(effort) = a.strip_prefix("--codex-effort=") {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "-c");
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    format!("model_reasoning_effort=\"{}\"", effort),
                );
            }
        }
        push_tracked_arg(&mut cmd, &mut sidecar_args, "exec");
        if let Some(ref real_sid) = codex_plan.session_id {
            push_tracked_arg(&mut cmd, &mut sidecar_args, "resume");
            push_tracked_arg(&mut cmd, &mut sidecar_args, "--json");
            if let Some(path) = codex_last_message_path.as_ref() {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--output-last-message");
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    path.to_string_lossy().to_string(),
                );
            }
            if full_access {
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    "--dangerously-bypass-approvals-and-sandbox",
                );
            } else {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--skip-git-repo-check");
            }
            push_tracked_arg(&mut cmd, &mut sidecar_args, real_sid);
        } else {
            push_tracked_arg(&mut cmd, &mut sidecar_args, "--json");
            if let Some(path) = codex_last_message_path.as_ref() {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--output-last-message");
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    path.to_string_lossy().to_string(),
                );
            }
            if full_access {
                push_tracked_arg(
                    &mut cmd,
                    &mut sidecar_args,
                    "--dangerously-bypass-approvals-and-sandbox",
                );
            } else if cli_prompt_read_only(runtime_permission.as_deref()) {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--sandbox");
                push_tracked_arg(&mut cmd, &mut sidecar_args, "read-only");
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--skip-git-repo-check");
            } else {
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--sandbox");
                push_tracked_arg(&mut cmd, &mut sidecar_args, "workspace-write");
                push_tracked_arg(&mut cmd, &mut sidecar_args, "--skip-git-repo-check");
            }
        }
        for a in &extra_args {
            if !a.starts_with("--session-id=")
                && !a.starts_with("--codex-model=")
                && !a.starts_with("--codex-effort=")
                && (full_access || a != "--dangerously-bypass-approvals-and-sandbox")
            {
                push_tracked_arg(&mut cmd, &mut sidecar_args, a);
            }
        }
    } else if cli_name == "copilot" {
        if full_access {
            push_tracked_arg(&mut cmd, &mut sidecar_args, "--allow-all");
        }
        for a in &extra_args {
            push_tracked_arg(&mut cmd, &mut sidecar_args, a);
        }
    } else {
        for a in &extra_args {
            push_tracked_arg(&mut cmd, &mut sidecar_args, a);
        }
    }
    if cli_name == "codex" {
        push_tracked_arg(&mut cmd, &mut sidecar_args, &prompt);
    } else if cli_name == "copilot" || cli_name == "claude" || cli_name == "gemini" {
        push_tracked_arg(&mut cmd, &mut sidecar_args, "-p");
        push_tracked_arg(&mut cmd, &mut sidecar_args, &prompt);
    } else {
        push_tracked_arg(&mut cmd, &mut sidecar_args, &prompt);
    }
    if let Some(dir) = &cwd {
        cmd.current_dir(dir);
    }
    apply_env(
        &mut cmd,
        &mut sidecar_env,
        cli_name,
        actual_bin,
        cwd.as_deref(),
    );
    if cli_name == "codex" {
        if let Some((name, home)) = node_agent_codex_child_env::codex_child_home_env_assignment() {
            cmd.env(name, &home);
            sidecar_env.push((name.to_string(), home));
        }
    }
    let stdin_piped_empty = cli_name == "copilot" || cli_name == "claude" || cli_name == "gemini";
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(if stdin_piped_empty {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        });
    hide_tokio_command_window(&mut cmd);
    let codex_key = codex_plan.scope_key.clone();

    if node_agent_cli_sidecar_runner::sidecar_enabled_for_cli(cli_name) {
        let sidecar_registry = runtime.cli_sidecars.clone();
        let session_id = node_agent_cli_sidecar_runner::session_id_for_task(&req_id);
        let output_path = sidecar_registry.output_path(&req_id, &session_id);
        let launch_config = node_agent_cli_sidecar_runner::CliSidecarLaunchConfig {
            session_id,
            task_id: req_id.clone(),
            cli_name: cli_name.to_string(),
            route: node_agent_active_task::route_for_cli(cli_name).to_string(),
            program: actual_bin.to_string(),
            args: sidecar_args.clone(),
            cwd: cwd.clone(),
            runtime_permission: runtime_permission.clone(),
            env: sidecar_env.clone(),
            output_path,
            registry_dir: sidecar_registry.dir(),
            task_journal_dir: None,
            codex_session_scope_key: codex_key.clone(),
            legacy_codex_sessions_file: Some(codex_sessions_file.clone()),
            timeout_secs: cli_prompt_timeout_secs(cli_name, runtime_permission.as_deref()),
            stdin_piped_empty,
            initial_cols: node_agent_cli_pty::default_cols(),
            initial_rows: node_agent_cli_pty::default_rows(),
        };
        match node_agent_cli_sidecar_runner::spawn_sidecar(launch_config).await {
            Ok(launch) => {
                if let Some(pid) = launch.sidecar_pid {
                    runtime.set_cli_prompt_os_pid(&req_id, Some(pid)).await;
                    if let Err(error) = task_journal.record_process_started(&req_id, pid) {
                        warn!("PC 任务 journal 写入 sidecar pid 失败: {error}");
                    }
                }
                let result = node_agent_cli_sidecar_runner::follow_sidecar_output(
                    &sidecar_registry,
                    &req_id,
                    &launch.output_path,
                    &mut cancel_rx,
                    |event| match event {
                        node_agent_cli_sidecar_runner::CliSidecarOutputEvent::Stdout(text) => {
                            if cli_name == "codex" {
                                let (session_id, visible_text) =
                                    node_agent_codex_session::strip_session_id_lines(&text);
                                if let (Some(ref key), Some(real_id)) =
                                    (codex_key.as_ref(), session_id.as_deref())
                                {
                                    node_agent_codex_session::persist_session_compat(
                                        &task_journal,
                                        Some(&codex_sessions_file),
                                        &req_id,
                                        key,
                                        real_id,
                                    );
                                }
                                if visible_text.is_empty() {
                                    return;
                                }
                                send_cli_chunk_message(&out_tx, &req_id, &visible_text);
                            } else {
                                send_cli_chunk_message(&out_tx, &req_id, &text);
                            }
                        }
                        node_agent_cli_sidecar_runner::CliSidecarOutputEvent::Stderr(text) => {
                            if cli_name == "codex" {
                                if !text.trim().is_empty() {
                                    info!("[codex stderr] {}", text.trim_end());
                                }
                            } else {
                                send_cli_chunk_message(&out_tx, &req_id, &text);
                            }
                        }
                        node_agent_cli_sidecar_runner::CliSidecarOutputEvent::ChildStarted(pid) => {
                            if let Err(error) = task_journal.record_process_started(&req_id, pid) {
                                warn!("PC 任务 journal 写入 sidecar child pid 失败: {error}");
                            }
                        }
                    },
                )
                .await;
                let mut result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        let message = format!("sidecar 输出跟随失败: {error}");
                        record_cli_done_outcome(&task_journal, &req_id, false, Some(&message));
                        let _ = out_tx.send(ws_text(&AgentToServer::CliDone {
                            req_id,
                            exit_ok: false,
                            error: Some(message),
                            session_id: latest_codex_session_id(
                                cli_name,
                                &codex_plan,
                                &task_journal,
                            ),
                            prompt_tokens: None,
                            cached_input_tokens: None,
                            completion_tokens: None,
                            reasoning_tokens: None,
                            total_tokens: None,
                            model: None,
                            workspace_status: None,
                        }));
                        return;
                    }
                };
                if result.canceled {
                    let message = "用户已停止 PC CLI 任务".to_string();
                    let (exit_ok, error, workspace_status) =
                        finalize_cli_prompt_workspace(false, Some(message), conversation_workspace);
                    let model = cli_model_from_args(cli_name, &extra_args);
                    record_cli_done_outcome(&task_journal, &req_id, exit_ok, error.as_deref());
                    let _ = out_tx.send(ws_text(&cli_done_message(
                        req_id,
                        exit_ok,
                        error,
                        None,
                        model,
                        workspace_status,
                        latest_codex_session_id(cli_name, &codex_plan, &task_journal),
                    )));
                    return;
                }
                if !result.exit_ok && cli_name == "codex" && !codex_vault_switch_attempted {
                    if let Ok(Some(message)) =
                        node_agent_codex_vault::try_auto_switch_after_codex_failure(
                            &runtime,
                            &result.stdout_text,
                            &result.stderr_text,
                        )
                        .await
                    {
                        send_cli_chunk(
                            &out_tx,
                            &task_journal,
                            &req_id,
                            "stdout",
                            &format!("codex\n{message}\n"),
                        );
                        Box::pin(run_cli_prompt(CliPromptRun {
                            req_id,
                            bin: bin_owned,
                            cli_name: cli_name_owned,
                            extra_args,
                            runtime_permission,
                            cwd,
                            conversation_workspace,
                            prompt,
                            server_runtime_config,
                            approval_state,
                            task_journal,
                            runtime,
                            cancel_rx,
                            out_tx,
                            codex_vault_switch_attempted: true,
                        }))
                        .await;
                        return;
                    }
                }
                if !result.exit_ok
                    && cli_name == "codex"
                    && codex_plan.is_resume()
                    && node_agent_codex_session::stale_resume_failure(
                        &result.stdout_text,
                        &result.stderr_text,
                    )
                {
                    if let Some(scope_key) = codex_plan.scope_key.as_deref() {
                        node_agent_codex_session::clear_stale_session(
                            &task_journal,
                            &codex_sessions_file,
                            &req_id,
                            scope_key,
                        )
                        .await;
                    }
                    send_cli_chunk(
                        &out_tx,
                        &task_journal,
                        &req_id,
                        "stdout",
                        "codex\n已发现本机 Codex session 失效，正在清理旧 session 并自动重新开始本轮任务。\n",
                    );
                    Box::pin(run_cli_prompt(CliPromptRun {
                        req_id,
                        bin: bin_owned,
                        cli_name: cli_name_owned,
                        extra_args,
                        runtime_permission,
                        cwd,
                        conversation_workspace,
                        prompt,
                        server_runtime_config,
                        approval_state,
                        task_journal,
                        runtime,
                        cancel_rx,
                        out_tx,
                        codex_vault_switch_attempted,
                    }))
                    .await;
                    return;
                }
                if cli_name == "codex" && !contains_codex_reply_marker(&result.stdout_text) {
                    if let Some(text) = codex_last_message_chunk(codex_last_message_path.as_ref()) {
                        send_cli_chunk(&out_tx, &task_journal, &req_id, "stdout", &text);
                        result.stdout_text.push_str(&text);
                    }
                }
                if result.exit_ok
                    && cli_name == "codex"
                    && !contains_codex_reply_marker(&result.stdout_text)
                {
                    let diagnostic = if result.stdout_text.trim().is_empty() {
                        "Codex CLI 执行完成，但没有返回可解析输出。请查看 PC 节点日志确认是否已完成文件修改。"
                    } else {
                        "Codex CLI 执行完成，但输出里没有可解析的 codex 回复段。请查看 PC 节点日志确认是否已完成文件修改。"
                    };
                    let text = format!("codex\n{diagnostic}\n");
                    let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
                        req_id: req_id.clone(),
                        text: text.clone(),
                    }));
                    let _ = task_journal.record_cli_chunk(&req_id, "stdout", &text);
                }
                let error = if result.exit_ok {
                    None
                } else {
                    Some(cli_done_error(
                        cli_name,
                        &result.stdout_text,
                        &result.stderr_text,
                    ))
                };
                let (exit_ok, error, workspace_status) =
                    finalize_cli_prompt_workspace(result.exit_ok, error, conversation_workspace);
                record_cli_done_outcome(&task_journal, &req_id, exit_ok, error.as_deref());
                let combined_usage_text = format!("{}\n{}", result.stdout_text, result.stderr_text);
                let usage = cli_usage::parse_cli_usage(&combined_usage_text);
                let model = usage
                    .as_ref()
                    .and_then(|u| u.model.clone())
                    .or_else(|| cli_model_from_args(cli_name, &extra_args));
                let _ = out_tx.send(ws_text(&cli_done_message(
                    req_id,
                    exit_ok,
                    error,
                    usage,
                    model,
                    workspace_status,
                    latest_codex_session_id(cli_name, &codex_plan, &task_journal),
                )));
                return;
            }
            Err(error) => {
                warn!("启动 CLI sidecar 失败，回落到直接子进程: {error:#}");
            }
        }
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let message = format!("无法启动 {} : {}", bin, e);
            record_cli_done_outcome(&task_journal, &req_id, false, Some(&message));
            let _ = out_tx.send(ws_text(&AgentToServer::CliDone {
                req_id,
                exit_ok: false,
                error: Some(message),
                session_id: latest_codex_session_id(cli_name, &codex_plan, &task_journal),
                prompt_tokens: None,
                cached_input_tokens: None,
                completion_tokens: None,
                reasoning_tokens: None,
                total_tokens: None,
                model: None,
                workspace_status: None,
            }));
            return;
        }
    };
    if let Some(pid) = child.id() {
        runtime.set_cli_prompt_os_pid(&req_id, Some(pid)).await;
        if let Err(error) = task_journal.record_process_started(&req_id, pid) {
            warn!("PC 任务 journal 写入进程 pid 失败: {error}");
        }
    }

    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    let (stderr_tx, mut stderr_rx) = tokio::sync::mpsc::unbounded_channel::<Option<String>>();
    {
        let stderr_tx = stderr_tx.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) | Err(_) => {
                        let _ = stderr_tx.send(None);
                        break;
                    }
                    Ok(_) => {
                        while matches!(buf.last(), Some(&b'\n') | Some(&b'\r')) {
                            buf.pop();
                        }
                        let _ = stderr_tx.send(Some(String::from_utf8_lossy(&buf).into_owned()));
                    }
                }
            }
        });
    }
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let mut stdout_done = false;
    let mut stderr_done = false;

    while !stdout_done || !stderr_done {
        tokio::select! {
            line = stdout_lines.next_line(), if !stdout_done => match line {
                Ok(Some(l)) => {
                    stdout_text.push_str(&l);
                    stdout_text.push('\n');
                    if cli_name == "codex" {
                        if let Some(real_id) =
                            node_agent_codex_session::extract_session_id_from_text(&l)
                        {
                            if let Some(ref key) = codex_key {
                                node_agent_codex_session::persist_session_compat(
                                    &task_journal,
                                    Some(&codex_sessions_file),
                                    &req_id,
                                    key,
                                    &real_id,
                                );
                            }
                            continue;
                        }
                    }
                    send_cli_chunk(&out_tx, &task_journal, &req_id, "stdout", &(l + "\n"));
                }
                Ok(None) => { stdout_done = true; }
                Err(e) => {
                    let message = format!("stdout 读取错误: {e}");
                    warn!("{message}");
                    stdout_text.push_str(&message);
                    stdout_text.push('\n');
                    stdout_done = true;
                }
            },
            opt = stderr_rx.recv(), if !stderr_done => match opt {
                Some(Some(l)) => {
                    stderr_text.push_str(&l);
                    stderr_text.push('\n');
                    if cli_name == "codex" {
                        if let Some(real_id) =
                            node_agent_codex_session::extract_session_id_from_text(&l)
                        {
                            if let Some(ref key) = codex_key {
                                node_agent_codex_session::persist_session_compat(
                                    &task_journal,
                                    Some(&codex_sessions_file),
                                    &req_id,
                                    key,
                                    &real_id,
                                );
                            }
                            continue;
                        }
                        if !l.trim().is_empty() {
                            info!("[codex stderr] {}", l);
                            let _ = task_journal.record_cli_chunk(&req_id, "stderr", &(l + "\n"));
                        }
                    } else {
                        send_cli_chunk(&out_tx, &task_journal, &req_id, "stderr", &(l + "\n"));
                    }
                }
                Some(None) | None => { stderr_done = true; }
            },
            changed = cancel_rx.changed() => {
                if changed.is_ok() && *cancel_rx.borrow() {
                    warn!("[{}] CLI 收到取消请求，强杀进程", cli_name);
                    let _ = child.kill().await;
                    let message = "用户已停止 PC CLI 任务".to_string();
                    let (exit_ok, error, workspace_status) =
                        finalize_cli_prompt_workspace(false, Some(message), conversation_workspace);
                    let model = cli_model_from_args(cli_name, &extra_args);
                    record_cli_done_outcome(&task_journal, &req_id, exit_ok, error.as_deref());
                    let _ = out_tx.send(ws_text(&cli_done_message(
                        req_id,
                        exit_ok,
                        error,
                        None,
                        model,
                        workspace_status,
                        latest_codex_session_id(cli_name, &codex_plan, &task_journal),
                    )));
                    return;
                }
            },
            _ = tokio::time::sleep(std::time::Duration::from_secs(
                cli_prompt_timeout_secs(cli_name, runtime_permission.as_deref())
            )) => {
                warn!("[{}] CLI 执行超时，强杀进程", cli_name);
                let _ = child.kill().await;
                let timeout_secs = cli_prompt_timeout_secs(cli_name, runtime_permission.as_deref());
                let message = format!("{} 执行超时（超过{}秒），已强制终止",
                    cli_name, timeout_secs);
                record_cli_done_outcome(&task_journal, &req_id, false, Some(&message));
                let _ = out_tx.send(ws_text(&AgentToServer::CliDone {
                    req_id,
                    exit_ok: false,
                    error: Some(message),
                    session_id: latest_codex_session_id(cli_name, &codex_plan, &task_journal),
                    prompt_tokens: None,
                    cached_input_tokens: None,
                    completion_tokens: None,
                    reasoning_tokens: None,
                    total_tokens: None,
                    model: None,
                    workspace_status: None,
                }));
                return;
            },
        }
    }

    let exit_ok = child.wait().await.map(|s| s.success()).unwrap_or(false);
    if cli_name == "codex" && !contains_codex_reply_marker(&stdout_text) {
        if let Some(text) = codex_last_message_chunk(codex_last_message_path.as_ref()) {
            send_cli_chunk(&out_tx, &task_journal, &req_id, "stdout", &text);
            stdout_text.push_str(&text);
        }
    }
    if !exit_ok && cli_name == "codex" && !codex_vault_switch_attempted {
        if let Ok(Some(message)) = node_agent_codex_vault::try_auto_switch_after_codex_failure(
            &runtime,
            &stdout_text,
            &stderr_text,
        )
        .await
        {
            send_cli_chunk(
                &out_tx,
                &task_journal,
                &req_id,
                "stdout",
                &format!("codex\n{message}\n"),
            );
            Box::pin(run_cli_prompt(CliPromptRun {
                req_id,
                bin: bin_owned,
                cli_name: cli_name_owned,
                extra_args,
                runtime_permission,
                cwd,
                conversation_workspace,
                prompt,
                server_runtime_config,
                approval_state,
                task_journal,
                runtime,
                cancel_rx,
                out_tx,
                codex_vault_switch_attempted: true,
            }))
            .await;
            return;
        }
    }
    if !exit_ok
        && cli_name == "codex"
        && codex_plan.is_resume()
        && node_agent_codex_session::stale_resume_failure(&stdout_text, &stderr_text)
    {
        if let Some(scope_key) = codex_plan.scope_key.as_deref() {
            node_agent_codex_session::clear_stale_session(
                &task_journal,
                &codex_sessions_file,
                &req_id,
                scope_key,
            )
            .await;
        }
        send_cli_chunk(
            &out_tx,
            &task_journal,
            &req_id,
            "stdout",
            "codex\n已发现本机 Codex session 失效，正在清理旧 session 并自动重新开始本轮任务。\n",
        );
        Box::pin(run_cli_prompt(CliPromptRun {
            req_id,
            bin: bin_owned,
            cli_name: cli_name_owned,
            extra_args,
            runtime_permission,
            cwd,
            conversation_workspace,
            prompt,
            server_runtime_config,
            approval_state,
            task_journal,
            runtime,
            cancel_rx,
            out_tx,
            codex_vault_switch_attempted,
        }))
        .await;
        return;
    }
    if exit_ok && cli_name == "codex" && !contains_codex_reply_marker(&stdout_text) {
        let diagnostic = if stdout_text.trim().is_empty() {
            "Codex CLI 执行完成，但没有返回可解析输出。请查看 PC 节点日志确认是否已完成文件修改。"
        } else {
            "Codex CLI 执行完成，但输出里没有可解析的 codex 回复段。请查看 PC 节点日志确认是否已完成文件修改。"
        };
        let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
            req_id: req_id.clone(),
            text: format!("codex\n{diagnostic}\n"),
        }));
        let _ = task_journal.record_cli_chunk(&req_id, "stdout", &format!("codex\n{diagnostic}\n"));
    }
    let error = if exit_ok {
        None
    } else {
        Some(cli_done_error(cli_name, &stdout_text, &stderr_text))
    };
    let (exit_ok, error, workspace_status) =
        finalize_cli_prompt_workspace(exit_ok, error, conversation_workspace);
    record_cli_done_outcome(&task_journal, &req_id, exit_ok, error.as_deref());
    let combined_usage_text = format!("{}\n{}", stdout_text, stderr_text);
    let usage = cli_usage::parse_cli_usage(&combined_usage_text);
    let model = usage
        .as_ref()
        .and_then(|u| u.model.clone())
        .or_else(|| cli_model_from_args(cli_name, &extra_args));
    let _ = out_tx.send(ws_text(&cli_done_message(
        req_id,
        exit_ok,
        error,
        usage,
        model,
        workspace_status,
        latest_codex_session_id(cli_name, &codex_plan, &task_journal),
    )));
}

fn send_cli_chunk(
    out_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    task_journal: &node_agent_task_journal::TaskJournal,
    req_id: &str,
    stream: &str,
    text: &str,
) {
    let _ = task_journal.record_cli_chunk(req_id, stream, text);
    let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
        req_id: req_id.to_string(),
        text: text.to_string(),
    }));
}

fn send_cli_chunk_message(
    out_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
    req_id: &str,
    text: &str,
) {
    let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
        req_id: req_id.to_string(),
        text: text.to_string(),
    }));
}

fn codex_last_message_chunk(path: Option<&PathBuf>) -> Option<String> {
    let path = path?;
    let text = std::fs::read_to_string(path).ok()?;
    let _ = std::fs::remove_file(path);
    let reply = text.trim();
    if reply.is_empty() {
        None
    } else {
        Some(format!("codex\n{reply}\n"))
    }
}

fn contains_codex_reply_marker(output: &str) -> bool {
    strip_cli_control_sequences(output)
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("codex"))
}

fn strip_cli_control_sequences(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\u{1b}' {
            i += 1;
            if i < chars.len() && chars[i] == '[' {
                i += 1;
                while i < chars.len() {
                    let next = chars[i];
                    i += 1;
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
                continue;
            }
            if i < chars.len() && chars[i] == ']' {
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\u{7}' {
                        i += 1;
                        break;
                    }
                    if chars[i] == '\u{1b}' && i + 1 < chars.len() && chars[i + 1] == '\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            continue;
        }
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            i += 1;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    out
}

fn push_tracked_arg(
    cmd: &mut tokio::process::Command,
    sidecar_args: &mut Vec<String>,
    arg: impl AsRef<str>,
) {
    let arg = arg.as_ref().to_string();
    cmd.arg(&arg);
    sidecar_args.push(arg);
}

fn record_cli_done_outcome(
    task_journal: &node_agent_task_journal::TaskJournal,
    req_id: &str,
    exit_ok: bool,
    error: Option<&str>,
) {
    let status = if exit_ok {
        "done"
    } else if cli_done_error_is_canceled(error) {
        "canceled"
    } else {
        "failed"
    };
    if let Err(journal_error) = task_journal.record_finished_with_outcome(req_id, status, error) {
        warn!("PC 任务 journal 写入终态失败: {journal_error}");
    }
}

fn cli_done_error_is_canceled(error: Option<&str>) -> bool {
    let Some(error) = error.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let lower = error.to_ascii_lowercase();
    lower.contains("cancel")
        || lower.contains("cancelled")
        || lower.contains("canceled")
        || lower.contains("stopped")
        || error.contains("取消")
        || error.contains("停止")
        || error.contains("终止")
}

struct PreparedCliPromptCwd {
    cwd: Option<String>,
    conversation_workspace: Option<pc_workspace_provisioner::ConversationWorkspaceResult>,
}

fn prepare_cli_prompt_cwd(
    cwd: Option<String>,
    project_context: Option<homecli_proto::CliProjectContext>,
) -> anyhow::Result<PreparedCliPromptCwd> {
    let (base_cwd, context) = node_agent_cli_security::prepare_cli_base_cwd(cwd, project_context)?;
    if cli_prompt_read_only(context.runtime_permission.as_deref()) {
        return Ok(PreparedCliPromptCwd {
            cwd: Some(base_cwd.to_string_lossy().to_string()),
            conversation_workspace: None,
        });
    }
    let workspace = pc_workspace_provisioner::prepare_conversation_workspace(
        base_cwd.to_string_lossy().as_ref(),
        &context.project_id,
        &context.conversation_id,
    )?;
    if workspace.isolated {
        info!(
            "🧩 项目会话使用隔离 worktree: project={} conversation={} path={}",
            context.project_id, context.conversation_id, workspace.workspace_path
        );
    }
    Ok(PreparedCliPromptCwd {
        cwd: Some(workspace.workspace_path.clone()),
        conversation_workspace: Some(workspace),
    })
}

fn finalize_cli_prompt_workspace(
    exit_ok: bool,
    error: Option<String>,
    workspace: Option<pc_workspace_provisioner::ConversationWorkspaceResult>,
) -> (bool, Option<String>, Option<CliWorkspaceStatus>) {
    let Some(workspace) = workspace else {
        return (exit_ok, error, None);
    };
    if !exit_ok {
        return (
            exit_ok,
            error.clone(),
            Some(cli_workspace_status(
                &workspace,
                "skipped",
                error.as_deref(),
            )),
        );
    }
    match pc_workspace_provisioner::merge_conversation_workspace(&workspace) {
        Ok(message)
            if message.starts_with("conversation worktree still")
                || message.starts_with("conversation worktree missing git metadata")
                || message.starts_with("base workspace") =>
        {
            warn!("会话 worktree 暂未合并: {message}");
            (
                false,
                Some(message.clone()),
                Some(cli_workspace_status(&workspace, "blocked", Some(&message))),
            )
        }
        Ok(message) => {
            info!("会话 worktree 合并结果: {message}");
            let merge_status = if workspace.isolated {
                "merged"
            } else {
                "shared"
            };
            (
                true,
                None,
                Some(cli_workspace_status(
                    &workspace,
                    merge_status,
                    Some(&message),
                )),
            )
        }
        Err(e) => {
            warn!("会话 worktree 合并失败: {e:#}");
            let message = format!("会话 worktree 合并失败: {e}");
            (
                false,
                Some(message.clone()),
                Some(cli_workspace_status(&workspace, "failed", Some(&message))),
            )
        }
    }
}

fn cli_workspace_status(
    workspace: &pc_workspace_provisioner::ConversationWorkspaceResult,
    merge_status: &str,
    merge_message: Option<&str>,
) -> CliWorkspaceStatus {
    CliWorkspaceStatus {
        base_workspace_path: workspace.base_workspace_path.clone(),
        active_workspace_path: workspace.workspace_path.clone(),
        isolated: workspace.isolated,
        branch: workspace.branch.clone(),
        prepare_status: "prepared".into(),
        merge_status: Some(merge_status.into()),
        merge_message: merge_message.map(ToOwned::to_owned),
    }
}

fn cli_model_from_args(cli_name: &str, args: &[String]) -> Option<String> {
    if cli_name.eq_ignore_ascii_case("codex") {
        return args.iter().find_map(|arg| {
            arg.strip_prefix("--codex-model=")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });
    }

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix("--model=") {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
        if arg == "--model" {
            if let Some(value) = iter.next().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                return Some(value.to_string());
            }
        }
    }
    None
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
    hide_tokio_command_window(&mut cmd);

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
    let _ = out_tx.send(ws_text(&AgentToServer::TaskStarted {
        task_id: task_id.clone(),
        pid,
    }));

    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    // stderr 字节级读取，避免 Windows GBK 编码触发 UTF-8 错误
    let (stderr_tx2, mut stderr_rx2) = tokio::sync::mpsc::unbounded_channel::<Option<String>>();
    {
        let tx = stderr_tx2.clone();
        let task_id2 = task_id.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) | Err(_) => {
                        let _ = tx.send(None);
                        break;
                    }
                    Ok(_) => {
                        while matches!(buf.last(), Some(&b'\n') | Some(&b'\r')) {
                            buf.pop();
                        }
                        let _ = tx.send(Some(String::from_utf8_lossy(&buf).into_owned()));
                    }
                }
            }
            drop(task_id2); // 保持 task_id2 活跃直到 stderr 读完
        });
    }
    let mut stdout_done = false;
    let mut stderr_done = false;

    while !stdout_done || !stderr_done {
        tokio::select! {
            line = stdout_lines.next_line(), if !stdout_done => match line {
                Ok(Some(l)) => { let _ = out_tx.send(ws_text(&AgentToServer::TaskStdout { task_id: task_id.clone(), data: l + "\n" })); }
                Ok(None) => { stdout_done = true; }
                Err(e) => { warn!("stdout err: {e}"); stdout_done = true; }
            },
            opt = stderr_rx2.recv(), if !stderr_done => match opt {
                Some(Some(l)) => { let _ = out_tx.send(ws_text(&AgentToServer::TaskStderr { task_id: task_id.clone(), data: l + "\n" })); }
                Some(None) | None => { stderr_done = true; }
            },
        }
    }

    let code = child.wait().await.ok().and_then(|s| s.code());
    let _ = out_tx.send(ws_text(&AgentToServer::TaskExit { task_id, code }));
}

fn hide_tokio_command_window(_command: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        _command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
}

// ── 本机 TTS 合成（代理到本地 model-tts-worker）────────────────────────────

use base64::Engine as _;

async fn run_tts_synthesis(
    req_id: String,
    worker_base_url: String,
    text: String,
    voice_id: Option<String>,
    emotion_id: Option<String>,
    intensity: Option<String>,
    provider: Option<String>,
) -> AgentToServer {
    let url = format!("{}/synthesize", worker_base_url.trim_end_matches('/'));
    let mut body = serde_json::json!({
        "text": text,
        "cacheVersion": "pc_relay_v1",
    });
    if let Some(v) = &voice_id {
        body["voiceId"] = serde_json::json!(v);
    }
    if let Some(e) = &emotion_id {
        body["emotionId"] = serde_json::json!(e);
    }
    if let Some(i) = &intensity {
        body["intensity"] = serde_json::json!(i);
    }
    if let Some(p) = &provider {
        body["provider"] = serde_json::json!(p);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .unwrap_or_default();

    let resp = match client.post(&url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            return AgentToServer::TtsSynthesizeError {
                req_id,
                message: format!("本机 TTS Worker 请求失败: {e}"),
            }
        }
    };
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let msg = resp.text().await.unwrap_or_default();
        return AgentToServer::TtsSynthesizeError {
            req_id,
            message: format!("TTS Worker 返回 {status}: {msg}"),
        };
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/wav")
        .to_string();
    let worker_voice = resp
        .headers()
        .get("x-elon-tts-worker-voice")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    if content_type.starts_with("application/json") {
        match resp.json::<serde_json::Value>().await {
            Ok(j) => {
                let b64 = j
                    .get("audioBase64")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mime = j
                    .get("mime")
                    .and_then(|v| v.as_str())
                    .unwrap_or("audio/wav")
                    .to_string();
                AgentToServer::TtsSynthesizeResponse {
                    req_id,
                    audio_b64: b64,
                    mime,
                    worker_voice,
                }
            }
            Err(e) => AgentToServer::TtsSynthesizeError {
                req_id,
                message: format!("JSON 解析失败: {e}"),
            },
        }
    } else {
        match resp.bytes().await {
            Ok(bytes) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                AgentToServer::TtsSynthesizeResponse {
                    req_id,
                    audio_b64: b64,
                    mime: content_type,
                    worker_voice,
                }
            }
            Err(e) => AgentToServer::TtsSynthesizeError {
                req_id,
                message: format!("读取音频失败: {e}"),
            },
        }
    }
}

// ── 主连接循环 ────────────────────────────────────────────────────────────────

async fn run_session(
    cfg: &NodeConfig,
    creds: &Credentials,
    runtime: &Arc<NodeRuntime>,
) -> Result<()> {
    runtime
        .set_connected(false, "正在扫描本机能力，完成后连接云端")
        .await;

    // 扫描本地模型
    let models = discover_models(cfg).await;
    runtime.set_models(models.clone()).await;
    if models.is_empty() {
        warn!("⚠️  未发现本地 LLM，节点将以无模型状态上线（可后续发送 RegisterCapabilities 更新）");
    } else {
        info!(
            "🧠 发现 {} 个本地模型: {}",
            models.len(),
            models
                .iter()
                .map(|m| m.model_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // 检测本机可用的 CLI（返回 (cli名, 完整路径)）
    let cli_probe = runtime.refresh_cli_probe_now().await;
    let cli_pairs = cli_probe.available_pairs();
    let available_clis: Vec<String> = cli_probe.available_names();
    if !available_clis.is_empty() {
        info!(
            "🛠  检测到本地 CLI: {}",
            cli_pairs
                .iter()
                .map(|(n, p)| format!("{} ({})", n, p))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    // 将完整路径存到 runtime，供 run_cli_prompt 使用
    runtime.set_cli_paths(cli_pairs.clone()).await;
    let server_runtime_status = node_agent_route_c_status::server_runtime_status_from_cloud(
        &cfg.cloud_http_url,
        creds.user_token.as_deref(),
    )
    .await;
    let mut dev_runtime = elon_pc_dev_runtime::collect_dev_runtime_profile_with_server_runtime(
        &available_clis,
        server_runtime_status.ready,
    );
    dev_runtime.server_runtime_status = Some(server_runtime_status.status);
    if dev_runtime.workspace_provision_ready {
        info!(
            "📁 PC 开发运行时已就绪: {}",
            dev_runtime
                .workspace_root_path
                .as_deref()
                .unwrap_or("workspace root unknown")
        );
    } else {
        warn!("⚠️  PC 开发运行时未就绪: {}", dev_runtime.issues.join("；"));
    }
    let hardware = runtime.refresh_hardware_profile().await;
    let storage_settings = runtime.storage_settings.read().await.clone();
    let storage = pc_storage_repo::storage_profile(&storage_settings);

    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut request = cfg.cloud_url.as_str().into_client_request()?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", creds.agent_secret).parse()?,
    );

    let (ws_stream, _) = connect_async(request).await?;
    info!("✅ 已连接到云端: {}", cfg.cloud_url);

    let (ws_write, mut ws_read) = ws_stream.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();
    let (control_tx, mut control_rx) = mpsc::unbounded_channel::<Message>();
    let writer = tokio::spawn(async move {
        let mut sink = ws_write;
        while let Some(msg) = node_agent_ws_control_queue::recv(&mut control_rx, &mut out_rx).await
        {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    // The cloud requires the first WebSocket frame to be Register within 10s.
    // Discover capabilities first so the registered session can immediately
    // answer protocol pings and accept dispatched work.
    let lifecycle =
        node_agent_lifecycle::runtime_report(runtime, true, true, "正在注册云端会话").await;
    out_tx.send(ws_text(&AgentToServer::Register {
        agent_id: creds.agent_id.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        proto_version: PROTO_VERSION,
        allowed_clis: available_clis.clone(),
        allowed_cwds: vec![],
        owner_user_id: Some(creds.owner_user_id.clone()),
        device_name: Some(machine_label()),
        install_id: Some(runtime.install_id.clone()),
        hardware: Some(hardware.clone()),
        storage: Some(storage.clone()),
        dev_runtime: Some(dev_runtime.clone()),
        lifecycle: Some(lifecycle.clone()),
    }))?;
    // 发送 RegisterCapabilities（含 TTS Worker URL）
    let tts_url = runtime.tts_worker_url.read().await.clone();
    out_tx.send(ws_text(&AgentToServer::RegisterCapabilities {
        models: models.clone(),
        allowed_clis: available_clis,
        tts_worker_url: tts_url,
        hardware: Some(hardware),
        storage: Some(storage),
        dev_runtime: Some(dev_runtime),
        lifecycle: Some(lifecycle),
    }))?;
    runtime.set_connected(true, "已连接，贡献算力中").await;

    let ping_tx = control_tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            if ping_tx.send(Message::Ping(vec![])).is_err() {
                break;
            }
        }
    });

    let (cfg_r, out_tx_r, control_tx_r) = (cfg.clone(), out_tx.clone(), control_tx.clone());
    let read_result: Result<()> = async {
        loop {
            let frame = tokio::select! {
                _ = runtime.wake.notified() => {
                    info!("凭证已变更，断开当前会话以应用新状态");
                    break;
                }
                frame = tokio::time::timeout(CLOUD_WS_READ_TIMEOUT, ws_read.next()) => match frame {
                    Ok(Some(f)) => f.map_err(|e| anyhow!("ws read: {e}"))?,
                    Ok(None) => break,
                    Err(_) => {
                        return Err(anyhow!(
                            "云端 WebSocket {} 秒内无任何消息，主动重连",
                            CLOUD_WS_READ_TIMEOUT.as_secs()
                        ));
                    }
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
                                run_llm_inference(
                                    &cfg_c, req_id, &model, messages, max_tokens, tx_c,
                                )
                                .await;
                            });
                        }
                        ServerToAgent::Ping { nonce } => {
                            let _ = control_tx_r.send(ws_text(&AgentToServer::Pong { nonce }));
                        }
                        ServerToAgent::ProvisionProjectWorkspace {
                            req_id,
                            project_id,
                            user_id,
                            name,
                            template,
                            repo_url,
                            branch,
                        } => {
                            info!(
                                "📁 ProvisionProjectWorkspace: {} project={}",
                                req_id, project_id
                            );
                            let tx_c = out_tx_r.clone();
                            tokio::spawn(async move {
                                let project_id_for_error = project_id.clone();
                                let response =
                                    match pc_workspace_provisioner::provision_project_workspace(
                                        pc_workspace_provisioner::ProjectWorkspaceRequest {
                                            project_id,
                                            user_id,
                                            name,
                                            template,
                                            repo_url,
                                            branch,
                                        },
                                    ) {
                                        Ok(result) => AgentToServer::ProjectWorkspaceProvisioned {
                                            req_id,
                                            project_id: project_id_for_error,
                                            workspace_path: result.workspace_path,
                                            git_head: result.git_head,
                                            git_remote_origin: result.git_remote_origin,
                                            git_branch: result.git_branch,
                                            created: result.created,
                                        },
                                        Err(e) => AgentToServer::ProjectWorkspaceProvisionError {
                                            req_id,
                                            project_id: project_id_for_error,
                                            message: e.to_string(),
                                        },
                                    };
                                let _ = tx_c.send(ws_text(&response));
                            });
                        }
                        ServerToAgent::PrepareProjectStorageRepo {
                            req_id,
                            project_id,
                            user_id,
                            name,
                            branch,
                            access_token,
                            prepare_worktree,
                        } => {
                            info!(
                                "🗄️  PrepareProjectStorageRepo: {} project={}",
                                req_id, project_id
                            );
                            let tx_c = out_tx_r.clone();
                            let storage_settings = runtime.storage_settings.read().await.clone();
                            tokio::spawn(async move {
                                let project_id_for_error = project_id.clone();
                                let response = match pc_storage_repo::prepare_project_storage_repo(
                                    &storage_settings,
                                    pc_storage_repo::StorageRepoRequest {
                                        project_id,
                                        user_id,
                                        name,
                                        branch,
                                        access_token,
                                        prepare_worktree,
                                    },
                                ) {
                                    Ok(result) => AgentToServer::ProjectStorageRepoReady {
                                        req_id,
                                        project_id: project_id_for_error,
                                        storage_repo_path: result.storage_repo_path,
                                        storage_repo_url: result.storage_repo_url,
                                        storage_worktree_path: result.storage_worktree_path,
                                        branch: result.branch,
                                        created: result.created,
                                    },
                                    Err(e) => AgentToServer::ProjectStorageRepoError {
                                        req_id,
                                        project_id: project_id_for_error,
                                        message: e.to_string(),
                                    },
                                };
                                let _ = tx_c.send(ws_text(&response));
                            });
                        }
                        ServerToAgent::InspectProjectWorkspace {
                            req_id,
                            workspace_path,
                        } => {
                            project_workspace_inspect::spawn_workspace_inspect_response(
                                req_id,
                                workspace_path,
                                out_tx_r.clone(),
                            );
                        }
                        ServerToAgent::AuditProjectGitWorktrees {
                            req_id,
                            workspace_path,
                        } => {
                            project_git_worktree_audit::spawn_git_worktree_audit_response(
                                req_id,
                                workspace_path,
                                out_tx_r.clone(),
                            );
                        }
                        ServerToAgent::ReadProjectDocuments {
                            req_id,
                            workspace_path,
                            seed_defaults,
                        } => {
                            info!("📚 ReadProjectDocuments: {}", req_id);
                            let tx_c = out_tx_r.clone();
                            tokio::spawn(async move {
                                let path = std::path::PathBuf::from(workspace_path);
                                let response =
                                    match project_docs_scan::collect_project_documents_with_options(
                                        &path,
                                        project_docs_scan::ProjectDocumentScanOptions {
                                            seed_missing_defaults: seed_defaults,
                                        },
                                    ) {
                                        Ok(snapshot) => {
                                            AgentToServer::ProjectDocumentsRead { req_id, snapshot }
                                        }
                                        Err(e) => AgentToServer::ProjectDocumentsReadError {
                                            req_id,
                                            message: e.to_string(),
                                        },
                                    };
                                let _ = tx_c.send(ws_text(&response));
                            });
                        }
                        ServerToAgent::CleanupProjectWorkspace {
                            req_id,
                            project_id,
                            workspace_path,
                        } => {
                            info!("🧹 CleanupProjectWorkspace: {}", req_id);
                            let tx_c = out_tx_r.clone();
                            tokio::spawn(async move {
                                let project_id_for_error = project_id.clone();
                                let response =
                                    match pc_workspace_provisioner::cleanup_project_workspace(
                                        &project_id,
                                        &workspace_path,
                                    ) {
                                        Ok(result) => AgentToServer::ProjectWorkspaceCleaned {
                                            req_id,
                                            project_id: project_id_for_error,
                                            removed_paths: result.removed_paths,
                                            skipped_paths: result.skipped_paths,
                                        },
                                        Err(e) => AgentToServer::ProjectWorkspaceCleanupError {
                                            req_id,
                                            project_id: project_id_for_error,
                                            message: e.to_string(),
                                        },
                                    };
                                let _ = tx_c.send(ws_text(&response));
                            });
                        }
                        ServerToAgent::CliPrompt {
                            req_id,
                            cli,
                            extra_args,
                            cwd,
                            project_context,
                            prompt,
                        } => {
                            info!("📝 CliPrompt: {} cli={}", req_id, cli);
                            let tx_c = out_tx_r.clone();
                            let rt_c = runtime.clone();
                            tokio::spawn(async move {
                                let req_id_for_cleanup = req_id.clone();
                                let (cancel_tx, cancel_rx) = watch::channel(false);
                                if rt_c.cli_prompt_active(&req_id_for_cleanup).await {
                                    warn!(
                                        "拒绝重复启动 PC CLI prompt: {} 已经在运行",
                                        req_id_for_cleanup
                                    );
                                    let _ = tx_c.send(ws_text(&duplicate_cli_prompt_done(req_id)));
                                    return;
                                }
                                let requested_runtime_permission = project_context
                                    .as_ref()
                                    .and_then(|ctx| ctx.runtime_permission.clone());
                                let _ = tx_c.send(ws_text(&cli_prompt_accepted(
                                    req_id_for_cleanup.clone(),
                                    Some(cli.clone()),
                                    cwd.clone(),
                                    requested_runtime_permission.clone(),
                                )));
                                let resolved_cli = match rt_c.resolve_cli(&cli).await {
                                    Ok(resolved) => resolved,
                                    Err(e) => {
                                        let _ = tx_c.send(ws_text(&AgentToServer::CliDone {
                                            req_id,
                                            exit_ok: false,
                                            error: Some(e.to_string()),
                                            session_id: None,
                                            prompt_tokens: None,
                                            cached_input_tokens: None,
                                            completion_tokens: None,
                                            reasoning_tokens: None,
                                            total_tokens: None,
                                            model: None,
                                            workspace_status: None,
                                        }));
                                        return;
                                    }
                                };
                                // 处理 --attachment URL：下载图片到本地临时文件
                                // Copilot: --attachment <url> → 下载后 --attachment <local_path>
                                // Codex:   --attachment <url> → 下载后 -i <local_path>
                                let resolved_args = resolve_attachment_args(
                                    extra_args,
                                    resolved_cli.name(),
                                    rt_c.creds
                                        .read()
                                        .await
                                        .as_ref()
                                        .and_then(|c| c.user_token.clone())
                                        .as_deref(),
                                )
                                .await;
                                let runtime_permission = requested_runtime_permission;
                                if let Err(e) =
                                    node_agent_full_access::require_route_a_full_access_grant(
                                        &rt_c.full_access_grants,
                                        resolved_cli.name(),
                                        runtime_permission.as_deref(),
                                        project_context.as_ref(),
                                        cwd.as_deref(),
                                    )
                                    .await
                                {
                                    let _ = tx_c.send(ws_text(&AgentToServer::CliDone {
                                        req_id,
                                        exit_ok: false,
                                        error: Some(e.to_string()),
                                        session_id: None,
                                        prompt_tokens: None,
                                        cached_input_tokens: None,
                                        completion_tokens: None,
                                        reasoning_tokens: None,
                                        total_tokens: None,
                                        model: None,
                                        workspace_status: None,
                                    }));
                                    return;
                                }
                                let prepared_cwd =
                                    match prepare_cli_prompt_cwd(cwd, project_context) {
                                        Ok(cwd) => cwd,
                                        Err(e) => {
                                            let _ = tx_c.send(ws_text(&AgentToServer::CliDone {
                                                req_id,
                                                exit_ok: false,
                                                error: Some(e.to_string()),
                                                session_id: None,
                                                prompt_tokens: None,
                                                cached_input_tokens: None,
                                                completion_tokens: None,
                                                reasoning_tokens: None,
                                                total_tokens: None,
                                                model: None,
                                                workspace_status: None,
                                            }));
                                            return;
                                        }
                                    };
                                let handle = node_agent_active_task::ActiveCliPromptHandle::new(
                                    req_id_for_cleanup.clone(),
                                    resolved_cli.name().to_string(),
                                    node_agent_active_task::route_for_cli(resolved_cli.name()),
                                    prepared_cwd.cwd.clone(),
                                    runtime_permission.clone(),
                                    cancel_tx,
                                );
                                if !rt_c.try_register_cli_prompt(handle).await {
                                    warn!(
                                        "拒绝重复启动 PC CLI prompt: {} 注册竞争失败",
                                        req_id_for_cleanup
                                    );
                                    let _ = tx_c.send(ws_text(&duplicate_cli_prompt_done(req_id)));
                                    return;
                                }
                                if let Err(error) = rt_c.task_journal.record_started(
                                    node_agent_task_journal::TaskJournalStart {
                                        req_id: &req_id_for_cleanup,
                                        cli_name: resolved_cli.name(),
                                        route: Some(node_agent_active_task::route_for_cli(
                                            resolved_cli.name(),
                                        )),
                                        run_handle_id: Some(&req_id_for_cleanup),
                                        cwd: prepared_cwd.cwd.as_deref(),
                                        runtime_permission: runtime_permission.as_deref(),
                                    },
                                ) {
                                    warn!("PC 任务 journal 写入开始事件失败: {error}");
                                }
                                run_cli_prompt(CliPromptRun {
                                    req_id,
                                    bin: resolved_cli.bin().to_string(),
                                    cli_name: resolved_cli.name().to_string(),
                                    extra_args: resolved_args,
                                    runtime_permission,
                                    cwd: prepared_cwd.cwd,
                                    conversation_workspace: prepared_cwd.conversation_workspace,
                                    prompt,
                                    server_runtime_config: Some(
                                        crate::node_agent_server_runtime::ServerRuntimeConfig {
                                            server_url: rt_c.cloud_http_url(),
                                            user_token: rt_c.user_token().await,
                                        },
                                    ),
                                    approval_state: rt_c.tool_approvals.clone(),
                                    task_journal: rt_c.task_journal.clone(),
                                    runtime: rt_c.clone(),
                                    cancel_rx,
                                    out_tx: tx_c,
                                    codex_vault_switch_attempted: false,
                                })
                                .await;
                                if let Err(error) =
                                    rt_c.task_journal.record_finished(&req_id_for_cleanup)
                                {
                                    warn!("PC 任务 journal 写入结束事件失败: {error}");
                                }
                                rt_c.finish_cli_prompt(&req_id_for_cleanup).await;
                            });
                        }
                        ServerToAgent::Cancel { task_id } => {
                            let canceled = runtime.cancel_cli_prompt(&task_id).await;
                            if canceled {
                                info!("🛑 已请求取消 CLI prompt: {}", task_id);
                            } else {
                                warn!("🛑 未找到可取消的 CLI prompt: {}", task_id);
                            }
                        }
                        ServerToAgent::ToolApprovalDecision {
                            req_id,
                            approval_id,
                            dispatch_id,
                            decision,
                        } => {
                            let accepted = runtime
                                .decide_tool_approval(&req_id, &approval_id, &decision)
                                .await;
                            let _ =
                                out_tx_r.send(ws_text(&AgentToServer::ToolApprovalDecisionAck {
                                    req_id: req_id.clone(),
                                    approval_id: approval_id.clone(),
                                    dispatch_id,
                                    accepted,
                                }));
                            if accepted {
                                info!(
                                    "✅ 已接收工具审批决定: req_id={}, approval_id={}, decision={}",
                                    req_id, approval_id, decision
                                );
                            } else {
                                warn!(
                                    "⚠️ 工具审批决定未匹配到待审批调用: req_id={}, approval_id={}",
                                    req_id, approval_id
                                );
                            }
                        }
                        ServerToAgent::Exec {
                            task_id,
                            cli,
                            args,
                            cwd,
                            env,
                        } => {
                            info!("⚙️  Exec: {} {}", cli, args.join(" "));
                            let tx_c = out_tx_r.clone();
                            tokio::spawn(async move {
                                run_exec(task_id, cli, args, cwd, env, tx_c).await;
                            });
                        }
                        ServerToAgent::TtsSynthesizeRequest {
                            req_id,
                            text,
                            voice_id,
                            emotion_id,
                            intensity,
                            provider,
                        } => {
                            info!("🎙️  TTS 合成请求: {}", req_id);
                            let tx_c = out_tx_r.clone();
                            let rt_c = runtime.clone();
                            tokio::spawn(async move {
                                let worker_url = rt_c.tts_worker_url.read().await.clone();
                                let reply = match worker_url {
                                    None => AgentToServer::TtsSynthesizeError {
                                        req_id,
                                        message: "本机 TTS Worker 未配置".to_string(),
                                    },
                                    Some(url) => {
                                        run_tts_synthesis(
                                            req_id, url, text, voice_id, emotion_id, intensity,
                                            provider,
                                        )
                                        .await
                                    }
                                };
                                let _ = tx_c.send(ws_text(&reply));
                            });
                        }
                        ServerToAgent::UpdateClient {
                            version,
                            download_url,
                        } => {
                            let ver = version.as_deref().unwrap_or("latest");
                            info!("⬆️  收到云端更新指令，目标版本: {}", ver);
                            runtime.lifecycle.mark_planned_shutdown("update");
                            let cloud_http = runtime.cloud_http_url();
                            tokio::spawn(async move {
                                match crate::node_agent_client_maintenance::push_update_from_server(
                                    &cloud_http,
                                    download_url.as_deref(),
                                )
                                .await
                                {
                                    Ok(msg) => info!("✅ 自动更新已启动: {}", msg),
                                    Err(e) => warn!("⚠️  自动更新失败（需手动更新）: {}", e),
                                }
                            });
                        }
                        _ => {
                            // 其他消息类型暂不处理
                        }
                    }
                }
                Message::Ping(payload) => {
                    node_agent_ws_control_queue::send_pong(&control_tx_r, payload)
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

fn main() -> Result<()> {
    if let Some(config_path) = cli_sidecar_config_arg() {
        return tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(node_agent_cli_sidecar_runner::run_sidecar_from_config_path(
                config_path,
            ));
    }

    #[cfg(windows)]
    {
        let runtime_mode =
            node_client_launcher::runtime_mode_with_autostart_repair(running_as_legacy_agent_exe());
        if !runtime_mode {
            return node_client_launcher::run();
        }
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run_agent_runtime())
}

fn cli_sidecar_config_arg() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--cli-sidecar" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

#[cfg(windows)]
fn running_as_legacy_agent_exe() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .map(|name| name.eq_ignore_ascii_case("elon-node-agent.exe"))
        .unwrap_or(false)
}

async fn run_agent_runtime() -> Result<()> {
    dotenvy::dotenv().ok();
    // 也加载 _internal/node-agent.env（由启动器或 save-openai-key 写入的持久化配置）
    // 使用 override 模式：持久化文件优先于父进程继承的 env 变量，避免残留的外部 env 污染
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let internal_env = dir.join("_internal").join("node-agent.env");
            if internal_env.exists() {
                dotenvy::from_path_override(internal_env).ok();
            }
        }
    }
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = NodeConfig::from_env()?;
    node_agent_proxy::ensure_localhost_no_proxy();
    node_agent_proxy::ensure_cloud_no_proxy(&cfg.cloud_url, &cfg.cloud_http_url);
    let mut persisted = load_persisted();
    let install_id = ensure_install_id(&mut persisted);
    let storage_settings = initial_storage_settings(&persisted);
    let mut creds = initial_credentials(&persisted);
    save_persisted(&PersistedState::from_parts(
        &install_id,
        creds.as_ref(),
        &storage_settings,
    ));

    // 有登录 token 但还没有节点凭证 → 自动注册一次
    if creds.is_none() {
        let token = std::env::var("NODE_USER_TOKEN")
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| persisted.user_token.clone());
        if let Some(tok) = token {
            info!("检测到登录 token，正在自动注册节点…");
            match provision_node(&cfg, &tok, None, &install_id).await {
                Ok(c) => {
                    info!("✅ 节点已自动注册: {}", c.agent_id);
                    save_persisted(&PersistedState::from_parts(
                        &install_id,
                        Some(&c),
                        &storage_settings,
                    ));
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
    if storage_settings.enabled {
        info!(
            "   硬盘服务: {}",
            storage_settings
                .root_path
                .as_deref()
                .unwrap_or("<default storage root>")
        );
    }

    let runtime = Arc::new(NodeRuntime::new(cfg, creds, storage_settings, install_id));
    node_agent_lifecycle::spawn_heartbeat(runtime.lifecycle.clone());
    let admin_port = node_agent_admin_open::admin_port_from_env();
    spawn_admin_server(runtime.clone(), admin_port);
    node_agent_admin_open::maybe_open_admin_page(admin_port);
    runtime.ensure_cli_probe_background(true).await;
    runtime.refresh_models_background();

    let runtime_for_loop = runtime.clone();
    tokio::select! {
        _ = run_loop(runtime_for_loop) => {}
        signal = tokio::signal::ctrl_c() => {
            if let Err(error) = signal {
                warn!("监听 Win 端关闭信号失败: {error}");
            }
            runtime.lifecycle.mark_planned_shutdown("user_interrupt");
            runtime.lifecycle.mark_shutdown_completed("user_interrupt");
        }
    }
    Ok(())
}

#[derive(Default)]
struct NodeStatus {
    connected: bool,
    last_event: String,
    models_cached: Vec<ModelCapability>,
}

pub(crate) struct NodeRuntime {
    cfg: NodeConfig,
    install_id: String,
    creds: RwLock<Option<Credentials>>,
    status: RwLock<NodeStatus>,
    hardware_cached: RwLock<NodeHardwareProfile>,
    cli_paths: RwLock<Vec<(String, String)>>,
    cli_probe_cached: RwLock<LocalCliProbeSnapshot>,
    cli_probe_refreshing: AtomicBool,
    model_scan_refreshing: AtomicBool,
    tts_worker_url: RwLock<Option<String>>,
    storage_settings: RwLock<pc_storage_repo::StorageSettings>,
    active_cli_prompts: node_agent_active_task_registry::ActiveCliPromptRegistry,
    cli_sidecars: node_agent_cli_sidecar::CliSidecarRegistry,
    task_journal: node_agent_task_journal::TaskJournal,
    lifecycle: node_agent_lifecycle::NodeLifecycleTracker,
    tool_approvals: node_agent_tool_approval::ToolApprovalState,
    full_access_grants: node_agent_full_access::FullAccessGrantState,
    wake: Notify,
    local_admin_token: String,
}

impl NodeRuntime {
    fn new(
        cfg: NodeConfig,
        creds: Option<Credentials>,
        storage_settings: pc_storage_repo::StorageSettings,
        install_id: String,
    ) -> Self {
        let tts_url = std::env::var("NODE_TTS_WORKER_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        Self {
            cfg,
            install_id,
            creds: RwLock::new(creds),
            status: RwLock::new(NodeStatus::default()),
            hardware_cached: RwLock::new(crate::node_hardware_probe::collect_hardware_profile()),
            cli_paths: RwLock::new(Vec::new()),
            cli_probe_cached: RwLock::new(LocalCliProbeSnapshot::default()),
            cli_probe_refreshing: AtomicBool::new(false),
            model_scan_refreshing: AtomicBool::new(false),
            tts_worker_url: RwLock::new(tts_url),
            storage_settings: RwLock::new(storage_settings),
            active_cli_prompts: node_agent_active_task_registry::ActiveCliPromptRegistry::new(),
            cli_sidecars: node_agent_cli_sidecar::CliSidecarRegistry::default(),
            task_journal: node_agent_task_journal::TaskJournal::default(),
            lifecycle: node_agent_lifecycle::NodeLifecycleTracker::start(env!("CARGO_PKG_VERSION")),
            tool_approvals: node_agent_tool_approval::ToolApprovalState::default(),
            full_access_grants: node_agent_full_access::FullAccessGrantState::load_default(),
            wake: Notify::new(),
            local_admin_token: node_agent_local_admin::generate_local_admin_token(),
        }
    }

    async fn creds(&self) -> Option<Credentials> {
        self.creds.read().await.clone()
    }

    pub(crate) fn cloud_http_url(&self) -> String {
        self.cfg.cloud_http_url.clone()
    }

    pub(crate) fn local_admin_token(&self) -> &str {
        &self.local_admin_token
    }

    pub(crate) async fn user_token(&self) -> Option<String> {
        self.creds
            .read()
            .await
            .as_ref()
            .and_then(|creds| creds.user_token.clone())
    }

    async fn set_cli_paths(&self, paths: Vec<(String, String)>) {
        *self.cli_paths.write().await = paths;
    }

    async fn cached_cli_probe(&self) -> LocalCliProbeSnapshot {
        self.cli_probe_cached.read().await.clone()
    }

    fn refresh_models_background(self: &Arc<Self>) {
        if self.model_scan_refreshing.swap(true, Ordering::AcqRel) {
            return;
        }
        let runtime = self.clone();
        tokio::spawn(async move {
            let models = discover_models(&runtime.cfg).await;
            runtime.set_models(models).await;
            runtime
                .model_scan_refreshing
                .store(false, Ordering::Release);
        });
    }

    async fn ensure_cli_probe_background(self: &Arc<Self>, force: bool) {
        let stale = self.cached_cli_probe().await.is_stale();
        if !force && !stale {
            return;
        }
        if self.cli_probe_refreshing.swap(true, Ordering::AcqRel) {
            return;
        }
        let runtime = self.clone();
        tokio::spawn(async move {
            let snapshot = tokio::task::spawn_blocking(probe_local_clis)
                .await
                .unwrap_or_else(|_| LocalCliProbeSnapshot::default());
            runtime.set_cli_probe_snapshot(snapshot).await;
            runtime.cli_probe_refreshing.store(false, Ordering::Release);
        });
    }

    async fn refresh_cli_probe_now(self: &Arc<Self>) -> LocalCliProbeSnapshot {
        if self.cli_probe_refreshing.swap(true, Ordering::AcqRel) {
            for _ in 0..24 {
                tokio::time::sleep(Duration::from_millis(50)).await;
                if !self.cli_probe_refreshing.load(Ordering::Acquire) {
                    return self.cached_cli_probe().await;
                }
            }
            return self.cached_cli_probe().await;
        }
        let snapshot = tokio::task::spawn_blocking(probe_local_clis)
            .await
            .unwrap_or_else(|_| LocalCliProbeSnapshot::default());
        self.set_cli_probe_snapshot(snapshot.clone()).await;
        self.cli_probe_refreshing.store(false, Ordering::Release);
        snapshot
    }

    async fn set_cli_probe_snapshot(&self, snapshot: LocalCliProbeSnapshot) {
        let pairs = snapshot.available_pairs();
        self.set_cli_paths(pairs).await;
        *self.cli_probe_cached.write().await = snapshot;
    }

    async fn cli_prompt_active(&self, req_id: &str) -> bool {
        self.active_cli_prompts.contains(req_id).await
    }

    async fn try_register_cli_prompt(
        &self,
        handle: node_agent_active_task::ActiveCliPromptHandle,
    ) -> bool {
        self.active_cli_prompts.try_insert(handle).await
    }

    pub(crate) async fn cancel_cli_prompt(&self, req_id: &str) -> bool {
        let canceled = self
            .active_cli_prompts
            .cancel_tx(req_id)
            .await
            .map(|cancel_tx| cancel_tx.send(true).is_ok())
            .unwrap_or(false);
        if canceled {
            if let Err(error) = self.task_journal.record_cancel_requested(req_id) {
                warn!("PC 任务 journal 写入取消事件失败: {error}");
            }
            return true;
        }
        match self.cli_sidecars.record_cancel_command(req_id) {
            Ok(true) => {
                if let Err(error) = self.task_journal.record_cancel_requested(req_id) {
                    warn!("PC sidecar 任务 journal 写入取消事件失败: {error}");
                }
                true
            }
            Ok(false) => false,
            Err(error) => {
                warn!("PC sidecar 取消命令写入失败: {error}");
                false
            }
        }
    }

    pub(crate) async fn active_cli_prompt_view(
        &self,
        req_id: &str,
    ) -> Option<node_agent_active_task::ActiveCliPromptView> {
        let pending_approvals = self.tool_approvals.pending_for_req(req_id).await;
        self.active_cli_prompts
            .view(req_id, pending_approvals)
            .await
    }

    pub(crate) async fn active_cli_prompt_views_for_workspace(
        &self,
        workspace: &Path,
    ) -> Vec<node_agent_active_task::ActiveCliPromptView> {
        let workspace = node_agent_workspace_match::canonical_or_original(workspace);
        self.active_cli_prompts
            .views_without_approvals()
            .await
            .into_iter()
            .filter(|view| {
                view.cwd.as_deref().is_some_and(|cwd| {
                    node_agent_workspace_match::cwd_matches_workspace(cwd, &workspace)
                })
            })
            .collect()
    }

    pub(crate) fn task_journal_records_for_workspace(
        &self,
        workspace: &Path,
        limit: usize,
    ) -> anyhow::Result<Vec<node_agent_task_journal::TaskJournalRecord>> {
        self.task_journal
            .latest_records_for_workspace(workspace, limit)
    }

    pub(crate) fn task_journal_snapshot(
        &self,
        task_id: &str,
        since: usize,
        limit: usize,
    ) -> anyhow::Result<node_agent_task_journal::TaskJournalSnapshot> {
        self.task_journal.snapshot(task_id, since, limit)
    }

    async fn set_cli_prompt_os_pid(&self, req_id: &str, pid: Option<u32>) {
        self.active_cli_prompts.set_os_pid(req_id, pid).await;
    }

    async fn decide_tool_approval(&self, req_id: &str, approval_id: &str, decision: &str) -> bool {
        if self
            .tool_approvals
            .decide(req_id, approval_id, decision)
            .await
        {
            return true;
        }
        match self
            .cli_sidecars
            .record_tool_approval_decision(req_id, approval_id, decision)
        {
            Ok(accepted) => accepted,
            Err(error) => {
                warn!("PC sidecar 工具审批决定写入失败: {error}");
                false
            }
        }
    }

    async fn finish_cli_prompt(&self, req_id: &str) {
        let cleared_approvals = self.tool_approvals.clear_req(req_id).await;
        if cleared_approvals > 0 {
            info!("已清理 PC 任务 {req_id} 的 {cleared_approvals} 个遗留工具审批");
        }
        self.active_cli_prompts.remove(req_id).await;
    }

    async fn hardware_profile(&self) -> NodeHardwareProfile {
        self.hardware_cached.read().await.clone()
    }

    async fn refresh_hardware_profile(&self) -> NodeHardwareProfile {
        let hardware = crate::node_hardware_probe::collect_hardware_profile();
        *self.hardware_cached.write().await = hardware.clone();
        hardware
    }

    async fn resolve_cli(
        self: &Arc<Self>,
        name: &str,
    ) -> anyhow::Result<crate::node_agent_cli_security::ResolvedCli> {
        let cached_paths = self.cli_paths.read().await.clone();
        match crate::node_agent_cli_security::resolve_cli_request(name, cached_paths.as_slice()) {
            Ok(resolved) => Ok(resolved),
            Err(cached_error) => {
                let refreshed = self.refresh_cli_probe_now().await;
                let refreshed_paths = refreshed.available_pairs();
                match crate::node_agent_cli_security::resolve_cli_request(
                    name,
                    refreshed_paths.as_slice(),
                ) {
                    Ok(resolved) => {
                        info!(
                            "PC CLI 缓存刷新后找到 {} CLI: {}",
                            resolved.name(),
                            resolved.bin()
                        );
                        Ok(resolved)
                    }
                    Err(_) => Err(cli_unavailable_after_refresh_error(
                        name,
                        cached_error,
                        &refreshed,
                    )),
                }
            }
        }
    }

    async fn set_creds(&self, c: Option<Credentials>) {
        let storage = self.storage_settings.read().await.clone();
        save_persisted(&PersistedState::from_parts(
            &self.install_id,
            c.as_ref(),
            &storage,
        ));
        *self.creds.write().await = c;
        self.wake.notify_waiters();
    }

    async fn set_storage_settings(&self, settings: pc_storage_repo::StorageSettings) {
        let creds = self.creds.read().await.clone();
        save_persisted(&PersistedState::from_parts(
            &self.install_id,
            creds.as_ref(),
            &settings,
        ));
        *self.storage_settings.write().await = settings;
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

fn cli_unavailable_after_refresh_error(
    name: &str,
    cached_error: anyhow::Error,
    refreshed: &LocalCliProbeSnapshot,
) -> anyhow::Error {
    let clean = name.trim().to_ascii_lowercase();
    let detail = refreshed
        .tools
        .iter()
        .find(|tool| tool.name.eq_ignore_ascii_case(&clean))
        .map(cli_probe_tool_detail)
        .unwrap_or_else(|| format!("刷新后仍未找到 {clean} CLI"));
    anyhow!("此 PC 节点刷新本机 CLI 后仍不能使用 {clean}：{detail}。上一轮缓存错误：{cached_error}")
}

fn cli_probe_tool_detail(tool: &LocalCliToolStatus) -> String {
    let mut parts = vec![format!("状态={}", tool.status)];
    if let Some(path) = tool
        .path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("路径={path}"));
    }
    if let Some(detail) = tool
        .detail
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(detail.to_string());
    }
    if let Some(reason) = tool
        .reason
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("原因={reason}"));
    }
    if let Some(hint) = tool
        .fix_hint
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("建议={hint}"));
    }
    parts.join("；")
}

#[cfg(test)]
mod node_agent_cli_probe_status_tests {
    use super::{cli_probe_tool_detail, LocalCliToolStatus};

    #[test]
    fn cli_probe_tool_detail_keeps_actionable_context() {
        let detail = cli_probe_tool_detail(&LocalCliToolStatus {
            name: "codex".to_string(),
            label: "Codex",
            path: Some(r"C:\Users\me\AppData\Local\OpenAI\Codex\bin\abc\codex.exe".to_string()),
            version: None,
            installed: true,
            runnable: false,
            logged_in: Some(false),
            available: false,
            status: "not_runnable".to_string(),
            detail: Some("检测到 codex 命令，但无法非交互执行".to_string()),
            reason: Some("spawn_failed".to_string()),
            diagnosis: None,
            fix_hint: Some("请修复该 CLI 安装或 PATH 后重新检测。".to_string()),
            fix_action: "repair_path".to_string(),
            backend: "cli",
        });

        assert!(detail.contains("状态=not_runnable"));
        assert!(detail.contains("路径=C:\\Users\\me"));
        assert!(detail.contains("原因=spawn_failed"));
        assert!(detail.contains("重新检测"));
    }
}

fn spawn_admin_server(runtime: Arc<NodeRuntime>, port: u16) {
    let addr: std::net::SocketAddr = ([127, 0, 0, 1], port).into();
    tokio::spawn(async move {
        let cors = node_agent_local_admin::cors_layer(&runtime.cfg.cloud_http_url);
        let local_admin_guard = axum::middleware::from_fn_with_state(
            runtime.clone(),
            node_agent_local_admin::require_local_admin,
        );
        let protected_routes = axum::Router::new()
            .route("/api/env-check", axum::routing::get(admin_env_check))
            .route(
                "/api/install-env",
                axum::routing::post(node_agent_install_env::admin_install_env),
            )
            .route(
                "/api/codex-cli/refresh",
                axum::routing::post(admin_codex_cli_refresh),
            )
            .route(
                "/api/doctor/snapshot",
                axum::routing::get(windows_doctor::snapshot_handler),
            )
            .route(
                "/api/doctor/analyze",
                axum::routing::post(windows_doctor::analyze_handler),
            )
            .route(
                "/api/doctor/sessions",
                axum::routing::get(windows_doctor::sessions_list_handler)
                    .post(windows_doctor::session_create_handler),
            )
            .route(
                "/api/doctor/sessions/:session_id",
                axum::routing::get(windows_doctor::session_get_handler)
                    .delete(windows_doctor::session_delete_handler),
            )
            .route(
                "/api/doctor/memory",
                axum::routing::get(windows_doctor::memory_list_handler)
                    .post(windows_doctor::memory_save_handler),
            )
            .route(
                "/api/doctor/repair",
                axum::routing::post(windows_doctor::repair_handler),
            )
            .merge(node_agent_download_router::routes())
            .route(
                "/api/save-openai-key",
                axum::routing::post(admin_save_openai_key),
            )
            .route("/api/login", axum::routing::post(admin_login))
            .route("/api/logout", axum::routing::post(admin_logout))
            .route(
                "/api/register-project",
                axum::routing::post(admin_register_project),
            )
            .merge(node_agent_cli_sidecar_admin::routes())
            .merge(node_agent_codex_vault::routes())
            .merge(node_agent_task_journal_api::routes())
            .route(
                "/api/project-folder/pick",
                axum::routing::post(node_agent_project_picker::pick_local_project_folder),
            )
            .route(
                "/api/project-folder/default",
                axum::routing::post(node_agent_project_picker::prepare_default_project_folder),
            )
            .route(
                "/api/project-folder/inspect",
                axum::routing::post(node_agent_project_picker::inspect_local_project_folder),
            )
            .route(
                "/api/project-agent-runs",
                axum::routing::post(node_agent_project_agent_runs::list_handler),
            )
            .route(
                "/api/full-access/grants",
                axum::routing::get(node_agent_full_access::list_handler)
                    .post(node_agent_full_access::grant_handler),
            )
            .route(
                "/api/client-maintenance",
                axum::routing::get(node_agent_client_maintenance::status_handler),
            )
            .route(
                "/api/client-maintenance/autostart",
                axum::routing::get(node_agent_client_maintenance::autostart_status_handler)
                    .post(node_agent_client_maintenance::autostart_set_handler),
            )
            .route(
                "/api/client-maintenance/open",
                axum::routing::post(node_agent_client_maintenance::open_target_handler),
            )
            .route(
                "/api/client-maintenance/diagnostics/export",
                axum::routing::post(node_agent_client_diagnostics::export_handler),
            )
            .route(
                "/api/client-maintenance/update",
                axum::routing::post(node_agent_client_maintenance::update_handler),
            )
            .route(
                "/api/client-maintenance/repair",
                axum::routing::post(node_agent_client_maintenance::repair_handler),
            )
            .route(
                "/api/client-maintenance/uninstall",
                axum::routing::post(node_agent_client_maintenance::uninstall_handler),
            )
            .route(
                "/api/storage-config",
                axum::routing::get(admin_storage_config_get).post(admin_storage_config_set),
            )
            .route("/api/tts-status", axum::routing::get(admin_tts_status))
            .route(
                "/api/tts-relay-config",
                axum::routing::get(admin_tts_relay_get).post(admin_tts_relay_set),
            )
            .route_layer(local_admin_guard);
        let app = axum::Router::new()
            .merge(node_agent_local_pc_frontend::routes())
            .route(
                "/api/status",
                axum::routing::get(node_agent_admin_status::admin_status),
            )
            .route(
                "/storage/git/:token/*path",
                axum::routing::any(admin_storage_git_http),
            )
            .merge(protected_routes)
            .with_state(runtime)
            .layer(cors)
            .layer(node_agent_local_admin::private_network_header_layer());
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                info!("🖥️  本地 PC 工作台: http://127.0.0.1:{}/pc", port);
                if let Err(e) = axum::serve(listener, app).await {
                    warn!("admin server 退出: {e}");
                }
            }
            Err(e) => warn!("admin server 无法监听 {addr}: {e}"),
        }
    });
}

async fn admin_tts_relay_get(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let url = rt.tts_worker_url.read().await.clone();
    axum::Json(serde_json::json!({ "ttsWorkerUrl": url }))
}

#[derive(serde::Deserialize)]
struct TtsRelaySetReq {
    tts_worker_url: Option<String>,
}

async fn admin_tts_relay_set(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<TtsRelaySetReq>,
) -> axum::Json<serde_json::Value> {
    let url = req
        .tts_worker_url
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty());
    *rt.tts_worker_url.write().await = url.clone();
    rt.wake.notify_one();
    axum::Json(serde_json::json!({ "ok": true, "ttsWorkerUrl": url }))
}

async fn admin_storage_config_get(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let settings = rt.storage_settings.read().await.clone();
    let profile = pc_storage_repo::storage_profile(&settings);
    axum::Json(serde_json::json!({
        "enabled": settings.enabled,
        "root_path": settings.root_path,
        "git_base_url": settings.git_base_url,
        "profile": profile,
    }))
}

#[derive(serde::Deserialize)]
struct StorageConfigSetReq {
    enabled: Option<bool>,
    root_path: Option<String>,
    git_base_url: Option<String>,
}

async fn admin_storage_config_set(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<StorageConfigSetReq>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let enabled = req.enabled.unwrap_or(false);
    let root_path = clean_optional_admin_field(req.root_path.as_deref()).or_else(|| {
        enabled.then(|| {
            pc_storage_repo::default_storage_root()
                .to_string_lossy()
                .to_string()
        })
    });
    if enabled {
        if let Some(root) = root_path.as_deref() {
            if let Err(e) = std::fs::create_dir_all(root) {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({
                        "ok": false,
                        "error": format!("创建硬盘服务目录失败: {e}"),
                    })),
                );
            }
        }
    }
    let settings = pc_storage_repo::StorageSettings {
        enabled,
        root_path,
        git_base_url: clean_optional_admin_field(req.git_base_url.as_deref()),
    };
    rt.set_storage_settings(settings.clone()).await;
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "ok": true,
            "enabled": settings.enabled,
            "root_path": settings.root_path,
            "git_base_url": settings.git_base_url,
            "profile": pc_storage_repo::storage_profile(&settings),
        })),
    )
}

async fn admin_tts_status() -> axum::Json<serde_json::Value> {
    let port = std::env::var("ELON_TTS_WORKER_PORT")
        .or_else(|_| std::env::var("TTS_WORKER_PORT"))
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(5011);
    let enabled = std::env::var("TTS_WORKER_ENABLED")
        .map(|v| {
            matches!(
                v.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                return axum::Json(serde_json::json!({
                    "running": true,
                    "enabled_in_env": enabled,
                    "port": port,
                    "health": body,
                }));
            }
            axum::Json(
                serde_json::json!({ "running": true, "enabled_in_env": enabled, "port": port }),
            )
        }
        _ => axum::Json(serde_json::json!({
            "running": false,
            "enabled_in_env": enabled,
            "port": port,
        })),
    }
}

async fn admin_codex_cli_refresh(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let cli_probe = rt.refresh_cli_probe_now().await;
    axum::Json(serde_json::json!({
        "ok": true,
        "cli_probe": {
            "refreshing": rt.cli_probe_refreshing.load(Ordering::Acquire),
            "refreshed_at_ms": cli_probe.refreshed_at_ms,
            "stale": cli_probe.is_stale(),
        },
        "codex_cli": cli_probe.codex_status(),
        "allowed_clis": cli_probe.available_names(),
        "cli_tools": cli_probe.tools,
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
    let token = if let Some(t) = req
        .token
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        t
    } else {
        let account = req.account.unwrap_or_default();
        let account = account.trim();
        let password = req.password.unwrap_or_default();
        if account.is_empty() || password.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(
                    serde_json::json!({ "ok": false, "error": "请填写账号和密码，或直接粘贴 token" }),
                ),
            );
        }
        match cloud_login(&rt.cfg, account, &password).await {
            Ok(t) => t,
            Err(e) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(
                        serde_json::json!({ "ok": false, "error": format!("登录失败: {e}") }),
                    ),
                );
            }
        }
    };

    // 2) 用 token 注册/换取节点凭证；若已有凭证则尝试续约（保留 agent_id）
    let existing = rt.creds.read().await.clone();
    match provision_node(&rt.cfg, &token, existing.as_ref(), &rt.install_id).await {
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
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({ "ok": true })),
    )
}

#[derive(Deserialize)]
struct AdminRegisterReq {
    project_id: Option<String>,
    name: String,
    workspace_path: String,
    description: Option<String>,
    repo_url: Option<String>,
    branch: Option<String>,
    dev_profile: Option<serde_json::Value>,
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
            axum::Json(
                serde_json::json!({ "ok": false, "error": "name 和 workspace_path 不能为空" }),
            ),
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
    let inspect = project_workspace_inspect::inspect_project_workspace(path).ok();
    let repo_url = clean_optional_admin_field(req.repo_url.as_deref())
        .or_else(|| {
            inspect
                .as_ref()
                .and_then(|status| status.git_remote_origin.clone())
        })
        .or_else(|| git_value_at(pb, &["remote", "get-url", "origin"]));
    let branch = clean_optional_admin_field(req.branch.as_deref())
        .or_else(|| {
            inspect
                .as_ref()
                .and_then(|status| status.git_branch.clone())
        })
        .or_else(|| {
            git_value_at(pb, &["rev-parse", "--abbrev-ref", "HEAD"]).filter(|value| value != "HEAD")
        });

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
    let url = format!(
        "{}/api/projects/external",
        rt.cfg.cloud_http_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "project_id": req.project_id.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "name": name,
        "workspace_path": path,
        "description": req.description,
        "node_id": creds.agent_id,
        "repo_url": repo_url,
        "branch": branch,
        "landing": project_landing::load_workspace_landing(pb),
        "dev_profile": req.dev_profile,
    });
    let client = node_agent_cloud_net::direct_cloud_client_or_default(Duration::from_secs(15));
    match client
        .post(&url)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
            if status.is_success() {
                (
                    StatusCode::OK,
                    axum::Json(serde_json::json!({
                        "ok": true,
                        "cloud": json,
                    })),
                )
            } else {
                (
                    StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                    axum::Json(serde_json::json!({
                        "ok": false,
                        "error": format!("云端返回 {}: {}", status, json),
                    })),
                )
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

async fn admin_storage_git_http(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    req: axum::http::Request<axum::body::Body>,
) -> axum::response::Response {
    let settings = rt.storage_settings.read().await.clone();
    pc_storage_git_http::handle_git_http(settings, req).await
}

fn clean_optional_admin_field(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn git_value_at(path: &std::path::Path, args: &[&str]) -> Option<String> {
    let output = elon_pc_dev_runtime::command_output("git", args, Some(path)).ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

// ── AI 编码工具 & Android 环境检查 / 安装 ────────────────────────────────────

/// 检查单个命令行工具是否可用（PATH + 常见安装目录双路扫描）。
fn tool_available(bin: &str) -> bool {
    if elon_pc_dev_runtime::command_path(bin).is_some() {
        return true;
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
    let home =
        std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    let init = std::path::PathBuf::from(&home)
        .join(".gradle")
        .join("init.gradle");
    if !init.exists() {
        return false;
    }
    std::fs::read_to_string(&init)
        .map(|s| s.contains("maven.aliyun.com"))
        .unwrap_or(false)
}

/// GET /api/env-check — 返回各工具安装状态。
async fn admin_env_check(
    axum::extract::State(_rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(|| {
        let api_runtime = node_agent_api_runtime_config::status_from_env();
        let api_runtime_contract = node_agent_api_runtime_config::tool_contract();
        serde_json::json!({
            "git":          tool_available("git"),
            "java":         tool_available("java"),
            "node":         tool_available("node"),
            "npm":          tool_available("npm"),
            "codex":        tool_available("codex"),
            "copilot":      tool_available("copilot"),
            "claude":       tool_available("claude"),
            "gemini":       tool_available("gemini"),
            "android_sdk":  android_sdk_ready(),
            "gradle_mirror": gradle_mirror_ok(),
            "ollama":       tool_available("ollama"),
            "openai_key":   api_runtime.key_configured,
            "api_runtime_key": api_runtime.key_configured,
            "api_runtime_model": api_runtime.model,
            "api_runtime_model_configured": api_runtime.model_configured,
            "api_runtime_base": api_runtime.api_base,
            "api_runtime_ready": api_runtime.ready,
            "api_runtime_contract": api_runtime_contract,
        })
    })
    .await
    .unwrap_or_else(|_| serde_json::json!({}));
    axum::Json(result)
}

#[derive(Deserialize)]
struct SaveOpenAiKeyReq {
    api_key: String,
    model: Option<String>,
    api_base: Option<String>,
    base_url: Option<String>,
}

/// POST /api/save-openai-key — 保存本机 API key / Codex 共用的 OpenAI-compatible 配置。
async fn admin_save_openai_key(
    axum::extract::State(_rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<SaveOpenAiKeyReq>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use axum::http::StatusCode;

    let base = req.api_base.as_deref().or(req.base_url.as_deref());
    let save = match node_agent_api_runtime_config::validate_save(
        &req.api_key,
        req.model.as_deref(),
        base,
    ) {
        Ok(save) => save,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": error.to_string()
                })),
            );
        }
    };

    // 当前进程立即生效：本机 API key 运行方式和 Codex 子进程都会继承。
    node_agent_api_runtime_config::apply_to_process(&save);

    // 持久化到启动器实际读取的 _internal/node-agent.env，避免重启后本机 API key 配置丢失。
    if let Some(env_file) = node_agent_env_file_path() {
        if let Err(error) = node_agent_api_runtime_config::persist_to_env_file(&env_file, &save) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": error.to_string()
                })),
            );
        }
    }

    let status = node_agent_api_runtime_config::status_from_env();
    let contract = node_agent_api_runtime_config::tool_contract();
    let msg = if status.ready {
        "我的 API key 已就绪，Codex 也会继承该 API key"
    } else {
        "API key 已保存；还需要配置模型后才会就绪"
    };
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "ok": true,
            "msg": msg,
            "api_runtime_ready": status.ready,
            "api_runtime_model": status.model,
            "api_runtime_base": status.api_base,
            "api_runtime_contract": contract,
        })),
    )
}
