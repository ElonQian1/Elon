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
use homecli_proto::{AgentToServer, CliWorkspaceStatus, ServerToAgent, PROTO_VERSION};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

/// WS ping 间隔：每 30s 向云端发一次 WS-level Ping，检测 zombie 连接
const WS_PING_INTERVAL: Duration = Duration::from_secs(30);
/// 读超时：90s 内如果未收到任何 WS frame（包括 Pong），视为 zombie，强制断开重连
const WS_READ_TIMEOUT: Duration = Duration::from_secs(90);

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
                info!(
                    "[relay-client] 连接正常断开，{:.1}s 后重连",
                    backoff.as_secs_f32()
                );
                // 正常断开后重置退避，快速重连
                backoff = Duration::from_secs(2);
            }
            Err(e) => {
                warn!(
                    "[relay-client] 连接错误: {e:#}，{:.1}s 后重连",
                    backoff.as_secs_f32()
                );
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
    request
        .headers_mut()
        .insert("Authorization", format!("Bearer {}", agent_secret).parse()?);

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
        owner_user_id: None,
        device_name: local_device_name(),
        hardware: Some(crate::node_hardware_probe::collect_hardware_profile()),
    };
    out_tx.send(Message::Text(serde_json::to_string(&register)?))?;
    info!("[relay-client] Register 发送完毕，等待请求...");

    let http_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(55))
        .build()?;
    let local_base = format!("http://127.0.0.1:{}", local_port);

    // 周期 WS Ping：防止 NAT 保活 / 检测 zombie 连接
    let ping_tx = out_tx.clone();
    let _ping_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(WS_PING_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if ping_tx.send(Message::Ping(b"keepalive".to_vec())).is_err() {
                break; // 写任务已退出，结束 ping 循环
            }
        }
    });

    // 读循环：带超时，防止 zombie TCP 连接永久阻塞
    loop {
        let frame = match tokio::time::timeout(WS_READ_TIMEOUT, ws_read.next()).await {
            Ok(Some(Ok(f))) => f,
            Ok(Some(Err(e))) => return Err(e.into()),
            Ok(None) => break, // WS 正常关闭
            Err(_) => {
                return Err(anyhow!(
                    "read timeout ({:.0}s) - 云端连接可能已失效，强制重连",
                    WS_READ_TIMEOUT.as_secs_f32()
                ));
            }
        };
        let text = match frame {
            Message::Text(t) => t,
            Message::Close(_) => break,
            Message::Ping(d) => {
                let _ = out_tx.send(Message::Pong(d));
                continue;
            }
            Message::Pong(_) => continue, // 收到 Pong，超时计时器自然重置
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
                cwd,
                project_context,
                prompt,
            } => {
                let tx = out_tx.clone();
                tokio::spawn(handle_cli_prompt(
                    req_id,
                    cli,
                    extra_args,
                    cwd,
                    project_context,
                    prompt,
                    tx,
                ));
            }

            ServerToAgent::Ping { nonce } => {
                let pong = AgentToServer::Pong { nonce };
                let _ = out_tx.send(Message::Text(serde_json::to_string(&pong)?));
            }

            ServerToAgent::ProvisionProjectWorkspace {
                req_id,
                project_id,
                user_id,
                name,
                template,
            } => {
                let tx = out_tx.clone();
                tokio::spawn(async move {
                    let project_id_for_error = project_id.clone();
                    let response =
                        match crate::pc_workspace_provisioner::provision_project_workspace(
                            crate::pc_workspace_provisioner::ProjectWorkspaceRequest {
                                project_id,
                                user_id,
                                name,
                                template,
                            },
                        ) {
                            Ok(result) => AgentToServer::ProjectWorkspaceProvisioned {
                                req_id,
                                project_id: project_id_for_error,
                                workspace_path: result.workspace_path,
                                git_head: result.git_head,
                                created: result.created,
                            },
                            Err(e) => AgentToServer::ProjectWorkspaceProvisionError {
                                req_id,
                                project_id: project_id_for_error,
                                message: e.to_string(),
                            },
                        };
                    let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap()));
                });
            }

            ServerToAgent::InspectProjectWorkspace {
                req_id,
                workspace_path,
            } => {
                let tx = out_tx.clone();
                tokio::spawn(async move {
                    let response = match crate::project_workspace_inspect::inspect_project_workspace(
                        &workspace_path,
                    ) {
                        Ok(status) => AgentToServer::ProjectWorkspaceInspected { req_id, status },
                        Err(e) => AgentToServer::ProjectWorkspaceInspectError {
                            req_id,
                            message: e.to_string(),
                        },
                    };
                    let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap()));
                });
            }

            ServerToAgent::ReadProjectDocuments {
                req_id,
                workspace_path,
            } => {
                let tx = out_tx.clone();
                tokio::spawn(async move {
                    let path = std::path::PathBuf::from(workspace_path);
                    let response = match crate::project_docs_scan::collect_project_documents(&path)
                    {
                        Ok(snapshot) => AgentToServer::ProjectDocumentsRead { req_id, snapshot },
                        Err(e) => AgentToServer::ProjectDocumentsReadError {
                            req_id,
                            message: e.to_string(),
                        },
                    };
                    let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap()));
                });
            }

            ServerToAgent::CleanupProjectWorkspace {
                req_id,
                project_id,
                workspace_path,
            } => {
                let tx = out_tx.clone();
                tokio::spawn(async move {
                    let project_id_for_error = project_id.clone();
                    let response = match crate::pc_workspace_provisioner::cleanup_project_workspace(
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
                    let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap()));
                });
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

            // LLM 流式推理请求 —— pc_relay_client 不处理此消息
            ServerToAgent::LlmStreamRequest { req_id, .. } => {
                let err = AgentToServer::LlmStreamError {
                    req_id,
                    message: "此节点未配置 LLM 推理能力".into(),
                };
                let _ = out_tx.send(Message::Text(serde_json::to_string(&err)?));
            }

            // TTS 合成请求 —— pc_relay_client 不处理（由 node_agent_main 处理）
            ServerToAgent::TtsSynthesizeRequest { req_id, .. } => {
                let err = AgentToServer::TtsSynthesizeError {
                    req_id,
                    message: "此节点未配置 TTS 能力".into(),
                };
                let _ = out_tx.send(Message::Text(serde_json::to_string(&err)?));
            }
        }
    }

    drop(out_tx);
    let _ = writer.await;
    Ok(())
}

fn local_device_name() -> Option<String> {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

// ── CliPrompt 处理 ────────────────────────────────────────────────────────────

async fn handle_cli_prompt(
    req_id: String,
    cli: String,
    extra_args: Vec<String>,
    cwd: Option<String>,
    project_context: Option<homecli_proto::CliProjectContext>,
    prompt: String,
    out: mpsc::UnboundedSender<Message>,
) {
    info!(
        "[relay-client] CliPrompt: cli={} cwd={} req_id={}",
        cli,
        cwd.as_deref().unwrap_or("<default>"),
        req_id
    );
    let prepared_cwd = match prepare_cli_cwd(cwd, project_context) {
        Ok(cwd) => cwd,
        Err(e) => {
            warn!("[relay-client] 准备 CLI 工作目录失败: {e:#}");
            let done = AgentToServer::CliDone {
                req_id,
                exit_ok: false,
                error: Some(e.to_string()),
                prompt_tokens: None,
                cached_input_tokens: None,
                completion_tokens: None,
                reasoning_tokens: None,
                total_tokens: None,
                model: None,
                workspace_status: None,
            };
            let _ = out.send(Message::Text(serde_json::to_string(&done).unwrap()));
            return;
        }
    };
    let (exit_ok, error) = match run_cli_and_stream(
        &req_id,
        &cli,
        &extra_args,
        prepared_cwd.cwd.as_deref(),
        &prompt,
        &out,
    )
    .await
    {
        Ok(ok) => (ok, None),
        Err(e) => {
            warn!("[relay-client] CLI 执行失败: {e:#}");
            (false, Some(e.to_string()))
        }
    };
    let (exit_ok, error, workspace_status) =
        finalize_cli_workspace(exit_ok, error, prepared_cwd.conversation_workspace);
    let done = AgentToServer::CliDone {
        req_id,
        exit_ok,
        error,
        prompt_tokens: None,
        cached_input_tokens: None,
        completion_tokens: None,
        reasoning_tokens: None,
        total_tokens: None,
        model: None,
        workspace_status,
    };
    let _ = out.send(Message::Text(serde_json::to_string(&done).unwrap()));
}

struct PreparedCliCwd {
    cwd: Option<String>,
    conversation_workspace: Option<crate::pc_workspace_provisioner::ConversationWorkspaceResult>,
}

fn prepare_cli_cwd(
    cwd: Option<String>,
    project_context: Option<homecli_proto::CliProjectContext>,
) -> Result<PreparedCliCwd> {
    let Some(base_cwd) = cwd else {
        return Ok(PreparedCliCwd {
            cwd: None,
            conversation_workspace: None,
        });
    };
    let Some(context) = project_context else {
        return Ok(PreparedCliCwd {
            cwd: Some(base_cwd),
            conversation_workspace: None,
        });
    };
    let workspace = crate::pc_workspace_provisioner::prepare_conversation_workspace(
        &base_cwd,
        &context.project_id,
        &context.conversation_id,
    )?;
    if workspace.isolated {
        info!(
            "[relay-client] project={} conversation={} 使用会话 worktree: {}",
            context.project_id, context.conversation_id, workspace.workspace_path
        );
    }
    Ok(PreparedCliCwd {
        cwd: Some(workspace.workspace_path.clone()),
        conversation_workspace: Some(workspace),
    })
}

fn finalize_cli_workspace(
    exit_ok: bool,
    error: Option<String>,
    workspace: Option<crate::pc_workspace_provisioner::ConversationWorkspaceResult>,
) -> (bool, Option<String>, Option<CliWorkspaceStatus>) {
    let Some(workspace) = workspace else {
        return (exit_ok, error, None);
    };
    if !exit_ok {
        return (
            exit_ok,
            error.clone(),
            Some(workspace_status(&workspace, "skipped", error.as_deref())),
        );
    }
    match crate::pc_workspace_provisioner::merge_conversation_workspace(&workspace) {
        Ok(message)
            if message.starts_with("conversation worktree still")
                || message.starts_with("base workspace") =>
        {
            warn!("[relay-client] 会话 worktree 暂未合并: {message}");
            (
                false,
                Some(message.clone()),
                Some(workspace_status(&workspace, "blocked", Some(&message))),
            )
        }
        Ok(message) => {
            info!("[relay-client] 会话 worktree 合并结果: {message}");
            let merge_status = if workspace.isolated {
                "merged"
            } else {
                "shared"
            };
            (
                true,
                None,
                Some(workspace_status(&workspace, merge_status, Some(&message))),
            )
        }
        Err(e) => {
            warn!("[relay-client] 会话 worktree 合并失败: {e:#}");
            let message = format!("会话 worktree 合并失败: {e}");
            (
                false,
                Some(message.clone()),
                Some(workspace_status(&workspace, "failed", Some(&message))),
            )
        }
    }
}

fn workspace_status(
    workspace: &crate::pc_workspace_provisioner::ConversationWorkspaceResult,
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

fn resolve_cli_program(cli: &str) -> String {
    if cli.eq_ignore_ascii_case("copilot") {
        if let Ok(v) = std::env::var("COPILOT_CLI_BIN") {
            let v = v.trim();
            if !v.is_empty() {
                return v.to_string();
            }
        }

        #[cfg(windows)]
        {
            let mut candidates = Vec::new();
            if let Ok(appdata) = std::env::var("APPDATA") {
                candidates.push(format!(
                    r"{}\Code\User\globalStorage\github.copilot-chat\copilotCli\copilot.bat",
                    appdata
                ));
                candidates.push(format!(
                    r"{}\Code\User\globalStorage\github.copilot-chat\copilotCli\copilot",
                    appdata
                ));
                candidates.push(format!(r"{}\npm\copilot.cmd", appdata));
                candidates.push(format!(r"{}\npm\copilot", appdata));
            }
            for p in candidates {
                if Path::new(&p).exists() {
                    return p;
                }
            }
        }
    }
    cli.to_string()
}

async fn run_cli_and_stream(
    req_id: &str,
    cli: &str,
    extra_args: &[String],
    cwd: Option<&str>,
    prompt: &str,
    out: &mpsc::UnboundedSender<Message>,
) -> Result<bool> {
    use tokio::io::AsyncBufReadExt;
    use tokio::process::Command;

    let program = resolve_cli_program(cli);
    if program != cli {
        info!("[relay-client] cli={} 使用可执行路径: {}", cli, program);
    }
    #[cfg(windows)]
    let is_batch = {
        let p = program.to_ascii_lowercase();
        p.ends_with(".cmd") || p.ends_with(".bat")
    };

    #[cfg(not(windows))]
    let is_batch = false;

    let mut cmd = if is_batch {
        // .cmd/.bat 通过 cmd /C 执行，避免 CreateProcess 直接调用批处理导致参数异常。
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&program);
        c
    } else {
        Command::new(&program)
    };
    for arg in extra_args {
        cmd.arg(arg);
    }
    if let Some(cwd) = cwd.filter(|value| !value.trim().is_empty()) {
        cmd.current_dir(cwd);
    }
    cmd.arg("-p").arg(prompt);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::null()); // 不转发 stderr（包含 stats/warning）
    cmd.kill_on_drop(true);

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow!("启动 {} 失败: {e}", program))?;
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
            let body = B64
                .decode(b64)
                .map_err(|e| anyhow!("body base64 decode: {e}"))?;
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
