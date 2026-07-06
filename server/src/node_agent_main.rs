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
mod node_agent_admin_server;
use node_agent_admin_server::spawn_admin_server;
mod node_agent_admin_status;
mod node_agent_api_runtime_config;
mod node_agent_api_runtime_tools;
mod node_agent_cli_done;
mod node_agent_cli_env;
mod node_agent_cli_probe;
use node_agent_cli_probe::{cli_unavailable_after_refresh_error, LocalCliProbeSnapshot, probe_local_clis};
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
mod node_agent_codex_vault; mod node_agent_codex_vault_active; mod node_agent_codex_child_env; mod node_agent_codex_vault_emergency; mod node_agent_codex_auth_switch;
mod node_agent_config;
pub use node_agent_config::{Credentials, machine_label, NodeConfig, state_path};
use node_agent_config::{
    cloud_login, ensure_install_id, initial_credentials, initial_storage_settings,
    load_persisted, save_persisted, PersistedState,
};
mod node_agent_download_router;
mod node_agent_env;
mod node_agent_cli_runner;
use node_agent_cli_runner::*;
pub use node_agent_cli_runner::{prepare_cli_prompt_cwd, PreparedCliPromptCwd};
mod node_agent_exec;
use node_agent_exec::hide_tokio_command_window;
pub use node_agent_exec::run_exec;
mod node_agent_file_info;
mod node_agent_file_range;
mod node_agent_full_access;
mod node_agent_install_env;
mod node_agent_lifecycle;
mod node_agent_local_admin;
mod node_agent_local_llm;
use node_agent_local_llm::discover_models;
pub use node_agent_local_llm::run_llm_inference;
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
mod node_agent_session;
use node_agent_session::run_session;
#[cfg(test)]
mod node_agent_task_approval_cleanup_tests;
mod node_agent_task_approval_snapshot;
mod node_agent_task_journal;
mod node_agent_task_journal_api; mod node_agent_task_journal_events; mod node_agent_task_journal_inspect;
mod node_agent_task_journal_lock;
#[cfg(test)]
mod node_agent_task_journal_recovery_tests;
#[cfg(test)]
mod node_agent_task_lifecycle_pressure_tests;
mod node_agent_task_resume;
mod node_agent_task_resume_sidecar;
#[cfg(test)]
mod node_agent_task_resume_sidecar_tests;
mod node_agent_tts;
pub use node_agent_tts::run_tts_synthesis;
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
                    if let Some(message) = node_agent_codex_auth_switch::try_after_failure(
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
        if let Some(message) =
            node_agent_codex_auth_switch::try_after_failure(&runtime, &stdout_text, &stderr_text)
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
