//! PC 节点主 WebSocket 会话循环。
//! 从 node_agent_main.rs 拆分，保持行为不变。

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use futures::{SinkExt, StreamExt};
use homecli_proto::{AgentToServer, ServerToAgent, PROTO_VERSION};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

use super::node_agent_cli_done::{cli_prompt_accepted, duplicate_cli_prompt_done};
use super::node_agent_config::machine_label;
use super::node_agent_local_llm::discover_models;
use super::{
    node_agent_active_task, node_agent_full_access, node_agent_lifecycle,
    node_agent_route_c_status, node_agent_task_journal, node_agent_task_journal_inspect,
    node_agent_ws_control_queue, pc_storage_repo, pc_workspace_provisioner, prepare_cli_prompt_cwd,
    project_docs_scan, project_git_worktree_audit, project_workspace_inspect,
    resolve_attachment_args, run_cli_prompt, run_exec, run_llm_inference, run_tts_synthesis,
    ws_text, CliPromptRun, Credentials, NodeConfig, NodeRuntime, CLOUD_WS_READ_TIMEOUT,
};

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
                                let original_prompt = prompt.clone();
                                let prompt = match crate::node_agent_ui_design_workspace::prepare_ui_design_workspace(
                                    prompt,
                                    prepared_cwd.cwd.as_deref(),
                                    &resolved_args,
                                ) {
                                    Ok(prompt) => prompt,
                                    Err(error) => {
                                        warn!(error = %error, "UI 设计任务本地工件准备失败，继续使用原始任务上下文");
                                        format!("{original_prompt}\n\nUI design workspace preparation failed: {error:#}\n请先诊断附件与项目工作区，再继续任务。")
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
