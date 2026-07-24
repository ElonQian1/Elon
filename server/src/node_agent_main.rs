// server/src/node_agent_main.rs

#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

use anyhow::Result;
use homecli_proto::{AgentToServer, ModelCapability, NodeHardwareProfile};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::{watch, Notify, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use node_agent_cli_done::{cli_done_message, latest_codex_session_id};
use node_agent_cli_env::apply_env;
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
mod node_agent_android_device_lease;
mod node_agent_android_inspector;
mod node_agent_android_live;
mod node_agent_android_relay;
mod node_agent_api_runtime_config;
mod node_agent_api_runtime_tools;
mod node_agent_atomic_file;
mod node_agent_build_runtime;
mod node_agent_cache_advisor;
mod node_agent_cancel_saga;
mod node_agent_cli_done;
mod node_agent_cli_env;
mod node_agent_cli_output_aggregate;
mod node_agent_cli_probe;
mod node_agent_cli_runtime_policy;
mod node_agent_completion_outbox;
mod node_agent_source_preview;
use node_agent_cli_probe::{
    cli_unavailable_after_refresh_error, probe_local_clis, LocalCliProbeSnapshot,
};
#[cfg(test)]
mod node_agent_cli_prompt_timeout_tests;
mod node_agent_cli_pty;
mod node_agent_cli_security;
mod node_agent_cli_session_bridge;
mod node_agent_cli_session_bridge_capabilities;
mod node_agent_cli_sidecar;
mod node_agent_cli_sidecar_admin;
mod node_agent_cli_sidecar_io;
#[cfg(test)]
mod node_agent_cli_sidecar_persistence_tests;
#[cfg(test)]
mod node_agent_cli_sidecar_progress_tests;
mod node_agent_cli_sidecar_runner;
#[cfg(test)]
mod node_agent_cli_sidecar_runner_tests;
mod node_agent_cli_supervision_lease;
#[cfg(test)]
mod node_agent_cli_terminal_fixture_tests;
mod node_agent_cli_worker;
mod node_agent_client_diagnostic_logs;
mod node_agent_client_diagnostics;
mod node_agent_client_install_status;
mod node_agent_client_maintenance;
mod node_agent_cloud_net;
mod node_agent_cloud_projects;
mod node_agent_codex_approval;
mod node_agent_codex_auth_switch;
mod node_agent_codex_child_env;
mod node_agent_codex_session;
mod node_agent_codex_task_contract_identity;
mod node_agent_codex_vault;
mod node_agent_codex_vault_active;
mod node_agent_codex_vault_emergency;
mod node_agent_config;
use node_agent_config::{
    ensure_debug_package_identity, ensure_install_id, initial_credentials, initial_node_data_root,
    initial_storage_settings, load_persisted, save_persisted,
};
pub use node_agent_config::{machine_label, state_path, Credentials, NodeConfig};
mod node_agent_cli_redaction;
mod node_agent_cli_runner;
mod node_agent_data_root;
mod node_agent_download_router;
mod node_agent_env;
use node_agent_cli_runner::*;
pub use node_agent_cli_runner::{prepare_cli_prompt_cwd, PreparedCliPromptCwd};
mod node_agent_exec;
use node_agent_exec::hide_tokio_command_window;
pub use node_agent_exec::run_exec;
mod node_agent_desktop_review_auth;
mod node_agent_desktop_review_broker;
mod node_agent_file_info;
mod node_agent_file_range;
mod node_agent_full_access;
mod node_agent_install_env;
mod node_agent_instance_lock;
mod node_agent_lifecycle;
mod node_agent_local_admin;
mod node_agent_local_llm;
mod node_agent_local_task_contract_revision;
mod node_agent_local_task_detached_view;
mod node_agent_local_task_durable_reconcile;
mod node_agent_local_task_orphan_migration;
mod node_agent_local_task_orphan_reconcile;
mod node_agent_local_task_recovery_timing;
mod node_agent_local_task_resume;
mod node_agent_local_task_resume_context;
mod node_agent_local_task_resume_lineage;
mod node_agent_local_task_resume_rebuild;
mod node_agent_local_task_resume_routes;
mod node_agent_local_task_store;
mod node_agent_local_task_supervision;
mod node_agent_local_tasks;
mod node_agent_local_terminal_reconcile;
mod node_agent_supervision_finalized_identity;
mod node_agent_supervision_project_identity;
mod node_agent_supervision_protocol;
mod node_agent_supervision_terminal_lease;
mod node_agent_supervision_terminal_lease_safety;
mod node_agent_terminal_finalization;
mod node_agent_terminal_journal;
use node_agent_local_llm::discover_models;
pub use node_agent_local_llm::run_llm_inference;
mod node_agent_local_pc_frontend;
mod node_agent_program_resolver;
mod node_agent_project_agent_recovery;
mod node_agent_project_agent_runs;
mod node_agent_project_data_policy;
mod node_agent_project_docs_mcp;
mod node_agent_project_docs_mcp_graph_tools;
mod node_agent_project_docs_mcp_knowledge_tools;
mod node_agent_project_docs_mcp_tools;
mod node_agent_project_documents;
mod node_agent_project_manifest_identity;
mod node_agent_project_picker;
mod node_agent_project_profile;
mod node_agent_project_profile_node;
mod node_agent_project_profile_python;
mod node_agent_proxy;
mod node_agent_pwa_auth_profile;
mod node_agent_pwa_runtime;
mod node_agent_registration;
mod node_agent_release_identity;
mod node_agent_restart_drain;
mod node_agent_route_c_status;
mod node_agent_runtime_approval;
mod node_agent_runtime_events;
mod node_agent_self_evolution;
mod node_agent_server_runtime;
mod node_agent_session;
mod node_agent_session_cancel;
mod node_agent_shared_android_devices;
mod node_agent_sidecar_recovery;
mod node_agent_sidecar_recovery_replay;
mod node_agent_supervision_worktree_lease;
use node_agent_session::run_session;
#[cfg(test)]
mod node_agent_task_approval_cleanup_tests;
mod node_agent_task_approval_snapshot;
mod node_agent_task_journal;
mod node_agent_task_journal_api;
mod node_agent_task_journal_events;
mod node_agent_task_journal_inspect;
mod node_agent_task_journal_lock;
#[cfg(test)]
mod node_agent_task_journal_recovery_tests;
#[cfg(test)]
mod node_agent_task_lifecycle_pressure_test_support;
#[cfg(test)]
mod node_agent_task_lifecycle_pressure_tests;
mod node_agent_task_performance_timing;
mod node_agent_task_resume;
mod node_agent_task_resume_sidecar;
#[cfg(test)]
mod node_agent_task_resume_sidecar_tests;
mod node_agent_task_runtime_status;
mod node_agent_tts;
pub use node_agent_tts::run_tts_synthesis;
#[cfg(test)]
mod node_agent_project_docs_mcp_tests;
mod node_agent_tool_approval;
mod node_agent_tool_guard;
mod node_agent_ui_design_workspace;
mod node_agent_update_checkpoint;
mod node_agent_update_gate_reconcile;
mod node_agent_update_reconcile;
mod node_agent_update_recovery;
mod node_agent_update_recovery_api;
mod node_agent_update_recovery_status;
mod node_agent_update_recovery_terminal;
mod node_agent_update_resume;
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
mod project_document_analysis_model;
mod project_document_architecture;
mod project_document_authorization;
mod project_document_federation;
mod project_document_federation_service;
mod project_document_file_operation_model;
mod project_document_file_operations;
mod project_document_files;
mod project_document_git_transaction;
#[cfg(test)]
mod project_document_git_transaction_tests;
mod project_document_governance;
mod project_document_governance_api;
mod project_document_governance_facets;
mod project_document_governance_section_operations;
mod project_document_governance_service;
#[cfg(test)]
mod project_document_governance_tests;
mod project_document_index;
mod project_document_issue_workflow;
mod project_document_knowledge_graph;
mod project_document_knowledge_graph_health;
mod project_document_knowledge_graph_model;
mod project_document_knowledge_graph_service;
mod project_document_knowledge_graph_templates;
mod project_document_maintenance;
mod project_document_observability;
mod project_document_observability_api;
mod project_document_policy;
mod project_document_quality;
mod project_document_quality_rules;
mod project_document_response;
mod project_document_vault;
mod project_document_versioning;
mod project_git_worktree_audit;
mod project_landing;
mod project_workspace_inspect;
mod tools_patch;
mod windows_doctor;

mod node_agent_runtime;
mod node_agent_session_completion_ack;
pub(crate) use node_agent_runtime::NodeRuntime;

mod node_agent_cli_mcp;
mod node_agent_cli_prompt_direct;
mod node_agent_cli_prompt_runner;
mod node_agent_cli_prompt_sidecar;
mod node_agent_cli_task_dispatch;
mod node_agent_cli_task_registration;
mod node_agent_cloud_connection;
mod node_agent_cloud_control;
mod node_agent_codex_effort;
mod node_agent_codex_model_compat;
#[cfg(test)]
#[path = "node_agent_startup_order_tests.rs"]
mod node_agent_startup_order_tests;
pub(crate) use node_agent_cli_prompt_runner::{
    cli_prompt_read_only,
    resolve_attachment_args,
    run_cli_prompt,
    // 保持 super:: 兼容性：子模块通过 super:: 访问这些辅助函数
    ws_text,
    CliPromptRun,
};
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
        runtime.begin_connection_attempt().await;
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
        runtime.set_connection_backoff(backoff).await;
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
    let _instance_lock = node_agent_instance_lock::acquire(&node_agent_config::state_path())?;
    info!(
        path = %_instance_lock.path().display(),
        "已独占 PC 节点状态目录"
    );
    let mut persisted = load_persisted()?;
    let install_id = ensure_install_id(&mut persisted);
    ensure_debug_package_identity(&mut persisted, &install_id)?;
    // Persist the installation identity before binding a data-root marker. If
    // a later write fails, the next start must reuse the same identity instead
    // of making a valid marker look foreign.
    save_persisted(&persisted)?;
    let node_data_root = initial_node_data_root(&persisted, &install_id);
    let storage_settings = initial_storage_settings(&persisted, &node_data_root);
    if let Some(paths) = node_data_root.paths.as_ref() {
        node_agent_data_root::apply_to_process(paths);
    } else if let Some(reason) = node_data_root.invalid_reason.as_deref() {
        warn!("推荐数据根校验失败，将继续继承旧项目目录、缓存与硬盘服务配置: {reason}");
    }
    if node_data_root.source == node_agent_data_root::NodeDataRootSource::Environment
        && persisted.set_validated_node_data_root(&node_data_root)
    {
        persisted.set_storage_settings(&storage_settings);
        save_persisted(&persisted)?;
    }
    let mut creds = initial_credentials(&persisted);

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
                    let mut next_persisted = load_persisted()?;
                    next_persisted.set_install_id(&install_id);
                    next_persisted.set_credentials(Some(&c));
                    save_persisted(&next_persisted)?;
                    creds = Some(c);
                }
                Err(e) => warn!("自动注册失败（可在管理页重新登录）: {e:#}"),
            }
        }
    }

    match &creds {
        Some(c) => info!(
            "🚀 elon-node-agent {} 启动 (agent_id: {})",
            node_agent_release_identity::current(),
            c.agent_id
        ),
        None => info!(
            "🚀 elon-node-agent {} 启动（未登录，请打开管理页 http://127.0.0.1:7799/ 登录）",
            node_agent_release_identity::current()
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

    let runtime = Arc::new(NodeRuntime::new(
        cfg,
        creds,
        storage_settings,
        node_data_root,
        install_id,
    ));
    runtime.desktop_review_broker.spawn();
    // Bind the local management endpoint before any startup reconciliation.
    // Large stale-task inventories must not make the watchdog mistake a
    // healthy new runtime for a failed launch.
    let admin_port = node_agent_admin_open::admin_port_from_env();
    spawn_admin_server(runtime.clone(), admin_port).await;
    node_agent_supervision_terminal_lease::spawn_reconciler(runtime.clone());
    if let Err(error) = node_agent_cli_worker::cleanup_terminal_workers(&runtime.cli_sidecars) {
        warn!(%error, "清理已终态版本化 CLI worker 失败，保留旧 worker 继续启动");
    }
    if let Err(error) = node_agent_cancel_saga::reconcile_runtime(&runtime).await {
        warn!(%error, "启动时重放 durable cancel intent 失败，交由周期 reconcile 重试");
    }
    node_agent_sidecar_recovery::reconcile_surviving_sidecars(runtime.clone()).await;
    node_agent_update_reconcile::reconcile_startup(runtime.clone()).await;
    node_agent_restart_drain::recover_checkpoint_after_startup(runtime.clone());
    match runtime
        .update_recovery
        .mark_runtime_online_if_target(&node_agent_release_identity::current())
    {
        Ok(true) => {}
        Ok(false) => warn!("当前仍是 from_release 运行时，保持更新门禁等待目标版本"),
        Err(error) => warn!(%error, "记录节点更新 runtime-online 阶段失败"),
    }
    // Orphan conversion is the last startup ownership step. Surviving sidecars,
    // update reattach/Resume and runtime-online state must be visible first.
    runtime.reconcile_local_completion_outbox().await;
    node_agent_local_task_orphan_reconcile::spawn_reconciler(runtime.clone());
    runtime.spawn_lifecycle_heartbeat();
    node_agent_cancel_saga::spawn_reconciler(runtime.clone());
    node_agent_self_evolution::spawn_scheduler(runtime.clone());
    project_document_maintenance::spawn_maintenance_worker();
    node_agent_shared_android_devices::spawn(runtime.clone());
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
            runtime.mark_lifecycle_planned_shutdown("user_interrupt");
            runtime.mark_lifecycle_shutdown_completed("user_interrupt");
        }
    }
    Ok(())
}
