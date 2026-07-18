use super::*;

pub(super) async fn run_relay_loop(
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

pub(super) async fn run_relay_session(
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
    let (control_tx, mut control_rx) = mpsc::unbounded_channel::<Message>();
    let running_cli_tasks: RunningCliTasks = Arc::new(Mutex::new(HashMap::new()));

    // 写任务：drain out_rx → ws_write
    let writer = tokio::spawn(async move {
        let mut sink = ws_write;
        loop {
            let msg = tokio::select! {
                biased;
                control = control_rx.recv() => match control {
                    Some(msg) => msg,
                    None => break,
                },
                msg = out_rx.recv() => match msg {
                    Some(msg) => msg,
                    None => break,
                },
            };
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
        proto_version: LEGACY_RELAY_PROTO_VERSION,
        capabilities: Vec::new(),
        allowed_clis: vec!["copilot".into(), "codex".into()],
        allowed_cwds: vec![],
        owner_user_id: None,
        device_name: local_device_name(),
        install_id: None,
        hardware: Some(crate::node_hardware_probe::collect_hardware_profile()),
        storage: None,
        dev_runtime: None,
        lifecycle: None,
    };
    out_tx.send(Message::Text(serde_json::to_string(&register)?))?;
    info!("[relay-client] Register 发送完毕，等待请求...");

    let http_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(55))
        .build()?;
    let local_base = format!("http://127.0.0.1:{}", local_port);

    // 周期 WS Ping：防止 NAT 保活 / 检测 zombie 连接
    let ping_tx = control_tx.clone();
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
                let _ = control_tx.send(Message::Pong(d));
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
            ServerToAgent::CliCompletionAck { .. } => {
                // The legacy relay has no durable completion outbox.
            }
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
            ServerToAgent::AndroidDeviceHostRequest { request } => {
                let response = AgentToServer::HttpError {
                    req_id: request.req_id,
                    message: "旧版 relay 客户端不支持共享 Android 设备主机".to_string(),
                };
                let _ = out_tx.send(Message::Text(serde_json::to_string(&response)?));
            }

            ServerToAgent::CliPrompt {
                req_id,
                cli,
                extra_args,
                cwd,
                project_context,
                codex_credential_binding: _,
                requires_cloud_control: _,
                cloud_control_deadline: _,
                cloud_control_issued_at: _,
                cloud_control_ttl_ms: _,
                prompt,
            } => {
                let tx = out_tx.clone();
                let tasks = running_cli_tasks.clone();
                let req_id_for_task = req_id.clone();
                let req_id_for_cleanup = req_id.clone();
                let handle = tokio::spawn(async move {
                    crate::pc_relay_cli_prompt::handle_cli_prompt(
                        req_id_for_task,
                        cli,
                        extra_args,
                        cwd,
                        project_context,
                        prompt,
                        tx,
                    )
                    .await;
                    tasks.lock().await.remove(&req_id_for_cleanup);
                });
                let abort_handle = handle.abort_handle();
                running_cli_tasks
                    .lock()
                    .await
                    .insert(req_id.clone(), abort_handle);
                if handle.is_finished() {
                    running_cli_tasks.lock().await.remove(&req_id);
                }
            }

            ServerToAgent::InspectCliTaskJournal {
                req_id, task_id, ..
            } => {
                let response = AgentToServer::CliTaskJournalSnapshot {
                    req_id,
                    task_id,
                    ok: false,
                    snapshot: None,
                    error: Some(
                        "本地 relay 客户端不支持任务 journal 恢复查询，请使用 elon-node-agent"
                            .to_string(),
                    ),
                };
                let _ = out_tx.send(Message::Text(serde_json::to_string(&response)?));
            }

            ServerToAgent::Ping { nonce } => {
                let pong = AgentToServer::Pong { nonce };
                let _ = control_tx.send(Message::Text(serde_json::to_string(&pong)?));
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
                    let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap()));
                });
            }

            ServerToAgent::PrepareProjectStorageRepo {
                req_id, project_id, ..
            } => {
                let err = AgentToServer::ProjectStorageRepoError {
                    req_id,
                    project_id,
                    message: "此 relay 客户端未启用项目硬盘服务，请使用 elon-node-agent".into(),
                };
                let _ = out_tx.send(Message::Text(serde_json::to_string(&err)?));
            }

            ServerToAgent::InspectProjectWorkspace {
                req_id,
                workspace_path,
            } => {
                crate::project_workspace_inspect::spawn_workspace_inspect_response(
                    req_id,
                    workspace_path,
                    out_tx.clone(),
                );
            }

            ServerToAgent::AuditProjectGitWorktrees {
                req_id,
                workspace_path,
            } => {
                crate::project_git_worktree_audit::spawn_git_worktree_audit_response(
                    req_id,
                    workspace_path,
                    out_tx.clone(),
                );
            }

            ServerToAgent::ReadProjectDocuments {
                req_id,
                workspace_path,
                seed_defaults,
                catalog_only,
            } => {
                let tx = out_tx.clone();
                tokio::spawn(async move {
                    let path = std::path::PathBuf::from(workspace_path);
                    let response =
                        match crate::project_docs_scan::collect_project_documents_with_options(
                            &path,
                            crate::project_docs_scan::ProjectDocumentScanOptions {
                                seed_missing_defaults: seed_defaults,
                                catalog_only,
                                include_analysis: true,
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
                    let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap()));
                });
            }
            ServerToAgent::ReadProjectDocumentFile {
                req_id,
                workspace_path,
                document_path,
            } => {
                let tx = out_tx.clone();
                tokio::spawn(async move {
                    let response = match crate::project_document_files::read_project_document_file(
                        std::path::Path::new(&workspace_path),
                        &document_path,
                    ) {
                        Ok(document) => AgentToServer::ProjectDocumentFileRead {
                            req_id,
                            path: document.path,
                            content: document.content,
                            revision: document.revision,
                            byte_len: document.byte_len,
                        },
                        Err(error) => AgentToServer::ProjectDocumentFileReadError {
                            req_id,
                            message: error.to_string(),
                        },
                    };
                    let _ = tx.send(Message::Text(serde_json::to_string(&response).unwrap()));
                });
            }
            ServerToAgent::WriteProjectDocumentFile {
                req_id,
                workspace_path,
                document_path,
                content,
                expected_revision,
            } => {
                let tx = out_tx.clone();
                tokio::spawn(async move {
                    let response = match crate::project_document_files::write_project_document_file(
                        std::path::Path::new(&workspace_path),
                        &document_path,
                        &content,
                        expected_revision.as_deref(),
                    ) {
                        Ok(document) => AgentToServer::ProjectDocumentFileWritten {
                            req_id,
                            path: document.path,
                            revision: document.revision,
                            byte_len: document.byte_len,
                        },
                        Err(error) => AgentToServer::ProjectDocumentFileWriteError {
                            req_id,
                            message: error.message,
                            conflict: error.conflict,
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
            ServerToAgent::Cancel { task_id, .. } => {
                let abort = running_cli_tasks.lock().await.remove(&task_id);
                if let Some(abort) = abort {
                    warn!("[relay-client] 收到取消请求，终止 CLI 任务: {task_id}");
                    abort.abort();
                    let done = AgentToServer::CliDone {
                        req_id: task_id,
                        exit_ok: false,
                        error: Some("任务已取消".into()),
                        session_id: None,
                        prompt_tokens: None,
                        cached_input_tokens: None,
                        completion_tokens: None,
                        reasoning_tokens: None,
                        total_tokens: None,
                        model: None,
                        workspace_status: None,
                    };
                    let _ = out_tx.send(Message::Text(serde_json::to_string(&done)?));
                } else {
                    warn!("[relay-client] 收到取消请求，但未找到运行中的 CLI 任务: {task_id}");
                    let err = AgentToServer::TaskError {
                        task_id,
                        message:
                            "未找到运行中的本地 CLI 任务，可能已经结束或不在此 relay 客户端执行"
                                .into(),
                    };
                    let _ = out_tx.send(Message::Text(serde_json::to_string(&err)?));
                }
            }

            ServerToAgent::ToolApprovalDecision { .. } => {
                // Tool approval decisions are handled by the full node agent runtime.
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

            // 更新指令由 node_agent_main 处理；此处静默忽略
            ServerToAgent::UpdateClient { .. } => {}
        }
    }

    drop(out_tx);
    drop(control_tx);
    let _ = writer.await;
    Ok(())
}

pub(super) fn local_device_name() -> Option<String> {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

// ── HTTP 请求转发 ─────────────────────────────────────────────────────────────

pub(super) async fn handle_http_request(
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
