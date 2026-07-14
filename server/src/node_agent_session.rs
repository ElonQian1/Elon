//! PC 节点主 WebSocket 会话循环。
//! 从 node_agent_main.rs 拆分，保持行为不变。

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures::{SinkExt, StreamExt};
use homecli_proto::{
    AgentToServer, CliCompletionProducerIdentity, ServerToAgent, CAP_ANDROID_DEVICE_HOST_V1,
    CAP_PROJECT_BUILD_CACHE_V1, PROTO_VERSION,
};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

use super::node_agent_config::machine_label;
use super::node_agent_local_llm::discover_models;
use super::{
    node_agent_active_task, node_agent_full_access, node_agent_lifecycle,
    node_agent_route_c_status, node_agent_task_journal, node_agent_task_journal_inspect,
    node_agent_ws_control_queue, pc_storage_repo, project_git_worktree_audit,
    project_workspace_inspect, resolve_attachment_args, run_cli_prompt, run_exec,
    run_llm_inference, run_tts_synthesis, ws_text, CliPromptRun, Credentials, NodeConfig,
    NodeRuntime, CLOUD_WS_READ_TIMEOUT,
};

const COMPLETION_REPLAY_INTERVAL: Duration = Duration::from_secs(3);
const COMPLETION_REPLAY_SCAN_LIMIT: usize = 100;
const COMPLETION_REPLAY_BATCH_LIMIT: usize = 16;
const COMPLETION_REPLAY_BASE_BACKOFF_MS: u64 = 3_000;
const COMPLETION_REPLAY_MAX_BACKOFF_MS: u64 = 5 * 60 * 1_000;
const SESSION_TASK_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

mod project_workspace;

// ── 主连接循环 ────────────────────────────────────────────────────────────────

pub(super) async fn run_session(
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
    let transition = runtime.node_data_root_transition.clone().lock_owned().await;
    let data_root = runtime.node_data_root.read().await.clone();
    let workspace_root = data_root
        .paths
        .as_ref()
        .map(elon_pc_dev_runtime::NodeDataPaths::workspaces);
    let profile_clis = available_clis.clone();
    let server_runtime_ready = server_runtime_status.ready;
    let mut dev_runtime = tokio::task::spawn_blocking(move || {
        let _transition = transition;
        elon_pc_dev_runtime::collect_dev_runtime_profile_with_workspace_root(
            &profile_clis,
            server_runtime_ready,
            workspace_root.as_deref(),
        )
    })
    .await
    .map_err(|error| anyhow!("PC 开发运行时能力探测异常结束: {error}"))?;
    dev_runtime.server_runtime_status = Some(server_runtime_status.status);
    if let Some(paths) = data_root.paths.as_ref() {
        dev_runtime.workspace_root_path = Some(paths.workspaces().to_string_lossy().to_string());
    } else {
        dev_runtime.workspace_root_path = data_root
            .configured_root()
            .map(|path| path.to_string_lossy().to_string());
        dev_runtime.workspace_root_writable = false;
        dev_runtime.workspace_provision_ready = false;
        let issue = data_root.invalid_reason.as_deref().map_or_else(
            || "尚未配置统一节点数据根，项目写入保持阻断".to_string(),
            |reason| format!("统一节点数据根无效，项目写入保持阻断: {reason}"),
        );
        if !dev_runtime.issues.iter().any(|existing| existing == &issue) {
            dev_runtime.issues.push(issue);
        }
    }
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
    let (session_stop_tx, session_stop_rx) = watch::channel(false);
    let mut writer_stop_rx = session_stop_rx.clone();
    let mut writer = tokio::spawn(async move {
        let mut sink = ws_write;
        loop {
            let msg = tokio::select! {
                _ = writer_stop_rx.changed() => break,
                msg = node_agent_ws_control_queue::recv(&mut control_rx, &mut out_rx) => msg,
            };
            let Some(msg) = msg else { break };
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
        capabilities: vec![
            CAP_PROJECT_BUILD_CACHE_V1.to_string(),
            CAP_ANDROID_DEVICE_HOST_V1.to_string(),
        ],
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
    let session_producer_identity = CliCompletionProducerIdentity {
        owner_user_id: creds.owner_user_id.clone(),
        agent_id: creds.agent_id.clone(),
        install_id: runtime.install_id.clone(),
    };

    // Durable completion replay runs for the entire connected session. It covers
    // both rows left by a previous process/session and new rows produced while this
    // WebSocket is already online.
    let mut replay_task = {
        let replay_runtime = runtime.clone();
        let replay_tx = out_tx.clone();
        let replay_identity = session_producer_identity.clone();
        let mut stop_rx = session_stop_rx.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(COMPLETION_REPLAY_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = stop_rx.changed() => return,
                    _ = ticker.tick() => {}
                }
                let pending = match replay_runtime
                    .completion_outbox
                    .list_pending_for_producer(&replay_identity, COMPLETION_REPLAY_SCAN_LIMIT)
                {
                    Ok(pending) => pending,
                    Err(error) => {
                        warn!(%error, "读取 CLI completion outbox 失败");
                        continue;
                    }
                };
                let now_ms = unix_now_ms();
                for pending in pending
                    .into_iter()
                    .filter(|pending| completion_replay_is_due(pending, now_ms))
                    .take(COMPLETION_REPLAY_BATCH_LIMIT)
                {
                    let completion = pending.completion;
                    let event_id = completion.event_id.clone();
                    if replay_tx
                        .send(ws_text(&AgentToServer::CliCompletionReplay { completion }))
                        .is_err()
                    {
                        return;
                    }
                    if let Err(error) = replay_runtime
                        .completion_outbox
                        .record_attempt(&event_id, None)
                    {
                        warn!(%event_id, %error, "记录 CLI completion 补传尝试失败");
                    }
                }
            }
        })
    };

    let ping_tx = control_tx.clone();
    let mut ping_stop_rx = session_stop_rx.clone();
    let mut ping_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = ping_stop_rx.changed() => break,
                _ = interval.tick() => {}
            }
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
                        ServerToAgent::CliCompletionAck {
                            event_id,
                            req_id,
                            accepted,
                            deduplicated: _,
                            retryable,
                            error,
                        } => {
                            if let Err(ack_error) = apply_cli_completion_ack(
                                &runtime.completion_outbox,
                                &runtime.local_tasks,
                                &session_producer_identity,
                                &event_id,
                                &req_id,
                                accepted,
                                retryable,
                                error.as_deref(),
                            ) {
                                warn!(%event_id, %req_id, error = %ack_error, "应用 CLI completion ACK 失败，保留本机持久记录供重试或诊断");
                            }
                        }
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
                        ServerToAgent::AndroidDeviceHostRequest { request } => crate::node_agent_android_relay::spawn(
                            runtime.clone(),
                            out_tx_r.clone(),
                            request,
                        ),
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
                            project_workspace::spawn_provision(
                                runtime.clone(),
                                out_tx_r.clone(),
                                project_workspace::ProvisionRequest {
                                    req_id,
                                    project_id,
                                    user_id,
                                    name,
                                    template,
                                    repo_url,
                                    branch,
                                },
                            );
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
                            project_workspace::spawn_prepare_storage(
                                runtime.clone(),
                                out_tx_r.clone(),
                                project_workspace::PrepareStorageRequest {
                                    req_id,
                                    project_id,
                                    user_id,
                                    name,
                                    branch,
                                    access_token,
                                    prepare_worktree,
                                },
                            );
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
                            catalog_only,
                        } => {
                            info!("📚 ReadProjectDocuments: {}", req_id);
                            crate::node_agent_project_documents::spawn_catalog_response(req_id, workspace_path, seed_defaults, catalog_only, out_tx_r.clone());
                        }
                        ServerToAgent::ReadProjectDocumentFile {
                            req_id,
                            workspace_path,
                            document_path,
                        } => crate::node_agent_project_documents::spawn_file_read_response(req_id, workspace_path, document_path, out_tx_r.clone()),
                        ServerToAgent::WriteProjectDocumentFile {
                            req_id,
                            workspace_path,
                            document_path,
                            content,
                            expected_revision,
                        } => crate::node_agent_project_documents::spawn_file_write_response(req_id, workspace_path, document_path, content, expected_revision, out_tx_r.clone()),
                        ServerToAgent::CleanupProjectWorkspace {
                            req_id,
                            project_id,
                            workspace_path,
                        } => {
                            info!("🧹 CleanupProjectWorkspace: {}", req_id);
                            project_workspace::spawn_cleanup(
                                runtime.clone(),
                                out_tx_r.clone(),
                                project_workspace::CleanupRequest {
                                    req_id,
                                    project_id,
                                    workspace_path,
                                },
                            );
                        }
                        ServerToAgent::InspectCliTaskJournal {
                            req_id,
                            task_id,
                            since,
                            limit,
                        } => node_agent_task_journal_inspect::spawn(
                            runtime.clone(),
                            out_tx_r.clone(),
                            req_id,
                            task_id,
                            since,
                            limit,
                        ),
                        ServerToAgent::CliPrompt {
                            req_id,
                            cli,
                            extra_args,
                            cwd,
                            project_context,
                            codex_credential_binding,
                            requires_cloud_control,
                            cloud_control_deadline,
                            cloud_control_issued_at,
                            cloud_control_ttl_ms,
                            prompt,
                        } => {
                            let completion_context =
                                crate::node_agent_cli_done::CliCompletionContext::cloud(
                                    session_producer_identity.clone(),
                                    project_context.clone(),
                                );
                            crate::node_agent_cli_task_dispatch::spawn_cli_task(
                                runtime.clone(),
                                out_tx_r.clone(),
                                crate::node_agent_cli_task_dispatch::CliTaskDispatchRequest {
                                    req_id,
                                    cli,
                                    extra_args,
                                    cwd,
                                    project_context,
                                    codex_credential_binding,
                                    requires_cloud_control,
                                    cloud_control_deadline,
                                    cloud_control_issued_at,
                                    cloud_control_ttl_ms,
                                    prompt,
                                    completion_context,
                                    allow_codex_auth_switch: true,
                                    frozen_codex_home: None,
                                },
                            );
                        }
                        ServerToAgent::Cancel { task_id } => {
                            match crate::node_agent_session_cancel::apply(runtime, &task_id).await? {
                                true => info!("🛑 已请求取消 CLI prompt: {}", task_id),
                                false => warn!("🛑 已保存启动前取消墓碑: {}", task_id),
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
                            project_context,
                        } => {
                            info!("⚙️  Exec: {} {}", cli, args.join(" "));
                            let tx_c = out_tx_r.clone();
                            let rt_c = runtime.clone();
                            tokio::spawn(async move {
                                run_exec(
                                    task_id,
                                    cli,
                                    args,
                                    cwd,
                                    env,
                                    project_context,
                                    rt_c,
                                    tx_c,
                                )
                                .await;
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

    runtime
        .set_connected(false, "云端连接已断开，正在等待重连")
        .await;
    // Cancellation follows the immutable per-task credential boundary. Local
    // owner tasks keep running; only tasks that adopted managed/shared homes
    // depend on this cloud control session.
    let canceled = runtime.cancel_cloud_controlled_cli_prompts().await;
    if canceled > 0 {
        warn!(
            canceled,
            "cloud-controlled Codex tasks canceled after cloud disconnect"
        );
    }

    let _ = session_stop_tx.send(true);
    drop(out_tx);
    drop(control_tx);
    shutdown_session_task(
        "completion replay",
        &mut replay_task,
        SESSION_TASK_SHUTDOWN_TIMEOUT,
    )
    .await;
    shutdown_session_task(
        "protocol ping",
        &mut ping_task,
        SESSION_TASK_SHUTDOWN_TIMEOUT,
    )
    .await;
    shutdown_session_task(
        "websocket writer",
        &mut writer,
        SESSION_TASK_SHUTDOWN_TIMEOUT,
    )
    .await;
    read_result
}

fn completion_replay_is_due(
    pending: &crate::node_agent_completion_outbox::PendingCliCompletion,
    now_ms: u64,
) -> bool {
    pending
        .last_attempt_at_ms
        .map_or(true, |last_attempt_at_ms| {
            now_ms.saturating_sub(last_attempt_at_ms)
                >= completion_replay_backoff_ms(pending.attempt_count)
        })
}

fn completion_replay_backoff_ms(attempt_count: u32) -> u64 {
    let shift = attempt_count.saturating_sub(1).min(7);
    COMPLETION_REPLAY_BASE_BACKOFF_MS
        .saturating_mul(1_u64 << shift)
        .min(COMPLETION_REPLAY_MAX_BACKOFF_MS)
}

fn unix_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

async fn shutdown_session_task(
    label: &str,
    task: &mut tokio::task::JoinHandle<()>,
    timeout_duration: Duration,
) {
    match tokio::time::timeout(timeout_duration, &mut *task).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) if !error.is_cancelled() => {
            warn!(%error, task = label, "PC node session task exited unexpectedly");
        }
        Ok(Err(_)) => {}
        Err(_) => {
            warn!(
                task = label,
                "PC node session task shutdown timed out; aborting"
            );
            task.abort();
            let _ = (&mut *task).await;
        }
    }
}

fn apply_cli_completion_ack(
    outbox: &crate::node_agent_completion_outbox::CliCompletionOutbox,
    local_tasks: &crate::node_agent_local_task_store::LocalTaskStore,
    authenticated_producer: &CliCompletionProducerIdentity,
    event_id: &str,
    req_id: &str,
    accepted: bool,
    retryable: bool,
    error: Option<&str>,
) -> Result<()> {
    let completion = outbox
        .completion_for_binding(event_id, req_id)
        .context("读取 CLI completion ACK 绑定")?
        .ok_or_else(|| anyhow!("未知或不匹配的 CLI completion ACK binding"))?;
    if completion.producer_identity.as_ref() != Some(authenticated_producer) {
        return Err(anyhow!("CLI completion ACK 不属于当前登录/节点/安装身份"));
    }
    let is_local_offline =
        completion.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN;

    // Local display state moves first. If this fails, the outbox remains pending
    // and the server's idempotent ACK can drive the same transition after retry.
    if is_local_offline {
        let reconciled = local_tasks
            .reconcile_completion(&completion)
            .context("从 durable completion 修复本机任务终态")?;
        if !reconciled {
            return Err(anyhow!(
                "durable completion 没有匹配的本机任务，保留 outbox 等待修复"
            ));
        }
        let display_updated = if accepted {
            local_tasks
                .mark_synced(event_id)
                .context("更新本机任务同步状态")?
        } else {
            local_tasks
                .mark_sync_error(event_id, retryable)
                .context("更新本机任务补传错误状态")?
        };
        if !display_updated {
            return Err(anyhow!(
                "本机任务尚未绑定 completion event，保留 outbox 等待重试"
            ));
        }
    }

    if accepted {
        if !outbox
            .acknowledge(event_id, req_id)
            .context("持久化 CLI completion ACK")?
        {
            return Err(anyhow!("CLI completion ACK binding 在迁移前消失"));
        }
        if !outbox
            .delete_acked(event_id)
            .context("清理已确认 CLI completion")?
        {
            return Err(anyhow!("已确认 CLI completion 未能安全清理"));
        }
    } else {
        let message = error.unwrap_or("服务器拒绝 CLI completion 补传");
        if !outbox
            .reject(event_id, req_id, retryable, message)
            .context("持久化 CLI completion 拒绝状态")?
        {
            return Err(anyhow!("CLI completion rejection binding 在迁移前消失"));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "node_agent_session_completion_ack_tests.rs"]
mod completion_ack_tests;
