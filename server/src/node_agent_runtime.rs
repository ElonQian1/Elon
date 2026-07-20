// server/src/node_agent_runtime.rs
//! NodeRuntime — PC 节点的核心运行时状态：凭证、CLI 状态、任务注册表、lifecycle。
//! 从 node_agent_main.rs 抽取，保持原有公共接口不变。

use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{Mutex, Notify, RwLock};
use tracing::{info, warn};

use homecli_proto::{ModelCapability, NodeHardwareProfile};

use crate::node_agent_active_task;
use crate::node_agent_active_task_registry;
use crate::node_agent_cli_probe::{
    cli_unavailable_after_refresh_error, probe_local_clis, LocalCliProbeSnapshot,
};
use crate::node_agent_cli_security;
use crate::node_agent_cli_sidecar;
use crate::node_agent_completion_outbox;
use crate::node_agent_config::{
    load_persisted, save_persisted, Credentials, NodeConfig, PersistedState,
};
use crate::node_agent_data_root::{self, NodeDataRootState};
use crate::node_agent_full_access;
use crate::node_agent_lifecycle;
use crate::node_agent_local_admin;
use crate::node_agent_local_llm::discover_models;
use crate::node_agent_local_task_store;
use crate::node_agent_task_journal;
use crate::node_agent_tool_approval;
use crate::node_agent_workspace_match;
use crate::pc_storage_repo;

#[derive(Default)]
pub(crate) struct NodeStatus {
    pub(crate) connected: bool,
    pub(crate) last_event: String,
    pub(crate) models_cached: Vec<ModelCapability>,
}

pub(crate) struct NodeRuntime {
    pub(crate) cfg: NodeConfig,
    pub(crate) install_id: String,
    pub(crate) creds: RwLock<Option<Credentials>>,
    pub(crate) status: RwLock<NodeStatus>,
    hardware_cached: RwLock<NodeHardwareProfile>,
    cli_paths: RwLock<Vec<(String, String)>>,
    cli_probe_cached: RwLock<LocalCliProbeSnapshot>,
    pub(crate) cli_probe_refreshing: AtomicBool,
    model_scan_refreshing: AtomicBool,
    pub(crate) tts_worker_url: RwLock<Option<String>>,
    pub(crate) storage_settings: RwLock<pc_storage_repo::StorageSettings>,
    pub(crate) node_data_root: RwLock<NodeDataRootState>,
    pub(crate) node_data_root_transition: Arc<Mutex<()>>,
    pub(crate) cache_advisor: crate::node_agent_cache_advisor::CacheArchitectureAdvisor,
    persisted_state_transition: Mutex<()>,
    pub(crate) active_cli_prompts: node_agent_active_task_registry::ActiveCliPromptRegistry,
    pub(crate) cli_sidecars: node_agent_cli_sidecar::CliSidecarRegistry,
    pub(crate) task_journal: node_agent_task_journal::TaskJournal,
    pub(crate) completion_outbox: node_agent_completion_outbox::CliCompletionOutbox,
    pub(crate) local_tasks: node_agent_local_task_store::LocalTaskStore,
    pub(crate) self_evolution: crate::node_agent_self_evolution::SelfEvolutionCoordinator,
    pub(crate) update_recovery: crate::node_agent_update_recovery::UpdateRecoveryStore,
    pub(crate) lifecycle: node_agent_lifecycle::NodeLifecycleTracker,
    pub(crate) tool_approvals: node_agent_tool_approval::ToolApprovalState,
    pub(crate) full_access_grants: node_agent_full_access::FullAccessGrantState,
    pub(crate) live_ui: std::sync::Arc<crate::node_agent_android_live::LiveUiBroker>,
    pub(crate) ui_fit_runs: std::sync::Arc<crate::node_agent_android_live::fit_run::FitRunService>,
    pub(crate) wake: Notify,
    pub(crate) desktop_review_auth: crate::node_agent_desktop_review_auth::DesktopReviewAuth,
    local_admin_token: String,
}

impl NodeRuntime {
    pub(crate) fn new(
        cfg: NodeConfig,
        creds: Option<Credentials>,
        storage_settings: pc_storage_repo::StorageSettings,
        node_data_root: NodeDataRootState,
        install_id: String,
    ) -> Self {
        let tts_url = std::env::var("NODE_TTS_WORKER_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let live_ui = Arc::new(crate::node_agent_android_live::LiveUiBroker::new());
        let ui_fit_runs =
            Arc::new(crate::node_agent_android_live::fit_run::FitRunService::live(live_ui.clone()));
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
            node_data_root: RwLock::new(node_data_root),
            node_data_root_transition: Arc::new(Mutex::new(())),
            cache_advisor: crate::node_agent_cache_advisor::CacheArchitectureAdvisor::load_default(
            ),
            persisted_state_transition: Mutex::new(()),
            active_cli_prompts: node_agent_active_task_registry::ActiveCliPromptRegistry::new(),
            cli_sidecars: node_agent_cli_sidecar::CliSidecarRegistry::default(),
            task_journal: node_agent_task_journal::TaskJournal::default(),
            completion_outbox: node_agent_completion_outbox::CliCompletionOutbox::default(),
            local_tasks: node_agent_local_task_store::LocalTaskStore::default(),
            self_evolution: crate::node_agent_self_evolution::SelfEvolutionCoordinator::default(),
            update_recovery: crate::node_agent_update_recovery::UpdateRecoveryStore::default(),
            lifecycle: node_agent_lifecycle::NodeLifecycleTracker::start(env!("CARGO_PKG_VERSION")),
            tool_approvals: node_agent_tool_approval::ToolApprovalState::default(),
            full_access_grants: node_agent_full_access::FullAccessGrantState::load_default(),
            live_ui,
            ui_fit_runs,
            wake: Notify::new(),
            desktop_review_auth: crate::node_agent_desktop_review_auth::DesktopReviewAuth::from_env(
            ),
            local_admin_token: node_agent_local_admin::generate_local_admin_token(),
        }
    }

    pub(crate) async fn creds(&self) -> Option<Credentials> {
        self.creds.read().await.clone()
    }

    /// Rebuild local UI terminal bindings from the durable outbox after an
    /// agent crash between the two SQLite commits. The outbox remains the
    /// source of truth and is never deleted here.
    pub(crate) fn reconcile_local_completion_outbox(&self) {
        let completions = match self.completion_outbox.list_pending(1_000) {
            Ok(completions) => completions,
            Err(error) => {
                warn!(%error, "failed to read durable completion outbox during startup repair");
                return;
            }
        };
        for completion in completions {
            if completion.origin != node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN {
                continue;
            }
            match self.local_tasks.reconcile_completion(&completion) {
                Ok(true) => info!(
                    req_id = %completion.req_id,
                    event_id = %completion.event_id,
                    "repaired local task terminal state from durable outbox"
                ),
                Ok(false) => warn!(
                    req_id = %completion.req_id,
                    event_id = %completion.event_id,
                    "durable local completion has no matching local task row"
                ),
                Err(error) => warn!(
                    req_id = %completion.req_id,
                    event_id = %completion.event_id,
                    %error,
                    "failed to repair local task terminal state from durable outbox"
                ),
            }
        }

        // Any remaining `running` row has lost its in-memory child handle. Keep
        // rows that still have a durable outbox envelope recoverable, and make all
        // other interrupted work explicit and resumable instead of displaying it as live forever.
        let mut durable_req_ids = match self.completion_outbox.pending_req_ids() {
            Ok(req_ids) => req_ids,
            Err(error) => {
                warn!(%error, "failed to read durable completion bindings during startup repair");
                return;
            }
        };
        match self.update_recovery.protected_task_ids() {
            Ok(protected) => durable_req_ids.extend(protected),
            Err(error) => {
                warn!(%error, "failed to read protected update recovery task ids");
                return;
            }
        }
        match self
            .local_tasks
            .interrupt_lingering_running(&durable_req_ids)
        {
            Ok(0) => {}
            Ok(resume_required) => warn!(
                resume_required,
                "marked local tasks resume-required after node-agent restart"
            ),
            Err(error) => warn!(%error, "failed to mark lingering local tasks resume-required"),
        }
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

    pub(crate) async fn set_cli_paths(&self, paths: Vec<(String, String)>) {
        *self.cli_paths.write().await = paths;
    }

    pub(crate) async fn cached_cli_probe(&self) -> LocalCliProbeSnapshot {
        self.cli_probe_cached.read().await.clone()
    }

    pub(crate) fn refresh_models_background(self: &Arc<Self>) {
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

    pub(crate) async fn ensure_cli_probe_background(self: &Arc<Self>, force: bool) {
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

    pub(crate) async fn refresh_cli_probe_now(self: &Arc<Self>) -> LocalCliProbeSnapshot {
        if self.cli_probe_refreshing.swap(true, Ordering::AcqRel) {
            for _ in 0..24 {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
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

    pub(crate) async fn set_cli_probe_snapshot(&self, snapshot: LocalCliProbeSnapshot) {
        let pairs = snapshot.available_pairs();
        self.set_cli_paths(pairs).await;
        *self.cli_probe_cached.write().await = snapshot;
    }

    pub(crate) async fn cli_prompt_active(&self, req_id: &str) -> bool {
        self.active_cli_prompts.contains(req_id).await
    }

    pub(crate) async fn try_register_cli_prompt(
        &self,
        handle: node_agent_active_task::ActiveCliPromptHandle,
    ) -> node_agent_active_task_registry::CliPromptRegistration {
        self.active_cli_prompts.try_insert_with_status(handle).await
    }

    pub(crate) async fn cancel_cli_prompt(&self, req_id: &str) -> bool {
        let audit = homecli_proto::CancelRequestAudit::now(
            "node_agent",
            "runtime",
            "runtime_cancel_requested",
        );
        self.cancel_cli_prompt_with_audit(req_id, &audit).await
    }

    pub(crate) async fn cancel_cli_prompt_with_audit(
        &self,
        req_id: &str,
        audit: &homecli_proto::CancelRequestAudit,
    ) -> bool {
        match self
            .cancel_cli_prompt_with_audit_result(req_id, audit)
            .await
        {
            Ok(outcome) => outcome.accepted(),
            Err(error) => {
                warn!("PC 任务 durable cancel saga 失败，拒绝确认取消: {error}");
                false
            }
        }
    }

    pub(crate) async fn cancel_cli_prompt_with_audit_result(
        &self,
        req_id: &str,
        audit: &homecli_proto::CancelRequestAudit,
    ) -> anyhow::Result<crate::node_agent_cancel_saga::CancelDispatchOutcome> {
        crate::node_agent_cancel_saga::request_cancel(
            &self.active_cli_prompts,
            &self.cli_sidecars,
            &self.task_journal,
            &self.local_tasks,
            req_id,
            audit,
        )
        .await
    }

    pub(crate) async fn adopt_cli_prompt_cloud_control(
        &self,
        req_id: &str,
    ) -> Option<tokio::sync::watch::Sender<bool>> {
        self.active_cli_prompts.adopt_cloud_control(req_id).await
    }

    pub(crate) async fn cancel_cloud_controlled_cli_prompts(&self) -> usize {
        let req_ids = self.active_cli_prompts.cloud_controlled_req_ids().await;
        let mut canceled = 0;
        for req_id in req_ids {
            canceled += usize::from(self.cancel_cli_prompt(&req_id).await);
        }
        canceled
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
        expected_cursor_epoch: Option<&str>,
    ) -> anyhow::Result<node_agent_task_journal::TaskJournalSnapshot> {
        self.task_journal
            .snapshot_with_epoch(task_id, since, limit, expected_cursor_epoch)
    }

    pub(crate) async fn set_cli_prompt_os_pid(&self, req_id: &str, pid: Option<u32>) {
        self.active_cli_prompts.set_os_pid(req_id, pid).await;
    }

    /// 获取 sidecar 注册表的克隆（供 CLI 任务运行器使用）
    pub(crate) fn sidecar_registry(&self) -> node_agent_cli_sidecar::CliSidecarRegistry {
        self.cli_sidecars.clone()
    }

    pub(crate) async fn decide_tool_approval(
        &self,
        req_id: &str,
        approval_id: &str,
        decision: &str,
    ) -> bool {
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

    pub(crate) async fn finish_cli_prompt(&self, req_id: &str) {
        let cleared_approvals = self.tool_approvals.clear_req(req_id).await;
        if cleared_approvals > 0 {
            info!("已清理 PC 任务 {req_id} 的 {cleared_approvals} 个遗留工具审批");
        }
        self.active_cli_prompts.remove(req_id).await;
    }

    pub(crate) async fn hardware_profile(&self) -> NodeHardwareProfile {
        self.hardware_cached.read().await.clone()
    }

    pub(crate) async fn refresh_hardware_profile(&self) -> NodeHardwareProfile {
        let hardware = crate::node_hardware_probe::collect_hardware_profile();
        *self.hardware_cached.write().await = hardware.clone();
        hardware
    }

    pub(crate) async fn resolve_cli(
        self: &Arc<Self>,
        name: &str,
    ) -> anyhow::Result<crate::node_agent_cli_security::ResolvedCli> {
        let cached_paths = self.cli_paths.read().await.clone();
        match node_agent_cli_security::resolve_cli_request(name, cached_paths.as_slice()) {
            Ok(resolved) => Ok(resolved),
            Err(cached_error) => {
                let refreshed = self.refresh_cli_probe_now().await;
                let refreshed_paths = refreshed.available_pairs();
                match node_agent_cli_security::resolve_cli_request(name, refreshed_paths.as_slice())
                {
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

    pub(crate) async fn set_creds(&self, c: Option<Credentials>) -> anyhow::Result<()> {
        let _transition = self.persisted_state_transition.lock().await;
        let mut persisted = load_persisted()?;
        persisted.set_install_id(&self.install_id);
        persisted.set_credentials(c.as_ref());
        save_persisted(&persisted)?;
        *self.creds.write().await = c;
        self.wake.notify_waiters();
        Ok(())
    }

    pub(crate) async fn set_storage_settings(
        &self,
        mut settings: pc_storage_repo::StorageSettings,
    ) -> anyhow::Result<()> {
        let _transition = self.persisted_state_transition.lock().await;
        let data_root = self.node_data_root.read().await.clone();
        if settings.root_path.is_none() {
            if let Some(paths) = data_root.paths.as_ref() {
                settings.root_path = Some(paths.storage().to_string_lossy().to_string());
            }
        }
        let mut persisted = load_persisted()?;
        persisted.set_install_id(&self.install_id);
        persisted.set_storage_settings(&settings);
        save_persisted(&persisted)?;
        *self.storage_settings.write().await = settings;
        self.wake.notify_waiters();
        Ok(())
    }

    /// Upgraded nodes may not yet have the recommended data-root field. Prepare
    /// it best-effort on the first project task, preferring a safe sibling of
    /// the already-bound project without adopting or moving the old project.
    pub(crate) async fn ensure_node_data_root_for_workspace(
        &self,
        workspace_hint: Option<&Path>,
    ) -> anyhow::Result<NodeDataRootState> {
        let _transition = self.node_data_root_transition.lock().await;
        let current = self.node_data_root.read().await.clone();
        if current.paths.is_some() {
            return Ok(current);
        }

        let current_for_prepare = current.clone();
        let workspace_hint = workspace_hint.map(Path::to_path_buf);
        let install_id = self.install_id.clone();
        let fallback_parent = node_agent_data_root::automatic_fallback_parent(
            &crate::node_agent_config::state_path(),
        )?;
        let prepared = tokio::task::spawn_blocking(move || {
            node_agent_data_root::prepare_automatic_root(
                &current_for_prepare,
                workspace_hint.as_deref(),
                Some(&fallback_parent),
                &install_id,
            )
        })
        .await
        .map_err(|error| anyhow::anyhow!("自动准备 AI 临时工作区异常结束: {error}"))??;
        let state = self.set_node_data_root(prepared).await?;
        info!(
            root = %state.configured_root().map(|path| path.display().to_string()).unwrap_or_default(),
            "已为升级节点自动准备 AI 临时工作区；原项目保持原位置"
        );
        Ok(state)
    }

    pub(crate) async fn set_node_data_root(
        &self,
        paths: elon_pc_dev_runtime::NodeDataPaths,
    ) -> anyhow::Result<NodeDataRootState> {
        let _persistence = self.persisted_state_transition.lock().await;
        // Acquire every in-memory write guard before the durable commit. Once
        // save_persisted succeeds there are no more await points, so dropping
        // an HTTP handler cannot expose a new node.json/process env alongside
        // stale in-memory roots.
        let mut data_root_guard = self.node_data_root.write().await;
        let mut storage_guard = self.storage_settings.write().await;
        let previous = data_root_guard.clone();
        let mut storage = storage_guard.clone();
        let legacy_workspace_root = previous
            .paths
            .as_ref()
            .map(elon_pc_dev_runtime::NodeDataPaths::workspaces)
            .or(previous.legacy_workspace_root.clone());
        let legacy_storage_root = storage
            .root_path
            .as_deref()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                previous
                    .paths
                    .as_ref()
                    .map(elon_pc_dev_runtime::NodeDataPaths::storage)
            })
            .or(previous.legacy_storage_root.clone());
        // Selecting or auto-backfilling a recommendation must not silently
        // repoint an already-running Git storage service. A new service with no
        // prior path may use the recommended storage directory by default.
        if storage.root_path.is_none() {
            storage.root_path = Some(paths.storage().to_string_lossy().to_string());
        }
        let next = NodeDataRootState::from_prepared_paths(
            paths.clone(),
            node_agent_data_root::NodeDataRootSource::Persisted,
            legacy_workspace_root,
            legacy_storage_root,
        );
        let mut persisted = load_persisted()?;
        persisted.set_install_id(&self.install_id);
        if !persisted.set_validated_node_data_root(&next) {
            anyhow::bail!("拒绝持久化未经验证的节点数据根");
        }
        persisted.set_storage_settings(&storage);
        save_persisted(&persisted)?;
        // node.json is the durable single source. Publish the new process and
        // in-memory state only after its atomic replacement succeeds. There
        // must be no await between these publications.
        node_agent_data_root::apply_to_process(&paths);
        *storage_guard = storage;
        *data_root_guard = next.clone();
        self.wake.notify_waiters();
        Ok(next)
    }

    pub(crate) async fn set_connected(&self, on: bool, evt: &str) {
        let mut s = self.status.write().await;
        s.connected = on;
        s.last_event = evt.to_string();
    }

    pub(crate) async fn is_cloud_connected(&self) -> bool {
        self.status.read().await.connected
    }

    pub(crate) async fn set_models(&self, models: Vec<ModelCapability>) {
        self.status.write().await.models_cached = models;
    }

    /// 启动 lifecycle 心跳（供 run_agent_runtime 调用）
    pub(crate) fn spawn_lifecycle_heartbeat(&self) {
        node_agent_lifecycle::spawn_heartbeat(self.lifecycle.clone());
    }

    /// 标记计划内关闭（供 ctrl-c 信号处理调用）
    pub(crate) fn mark_lifecycle_planned_shutdown(&self, reason: &str) {
        self.lifecycle.mark_planned_shutdown(reason);
    }

    /// 标记关闭完成（供 ctrl-c 信号处理调用）
    pub(crate) fn mark_lifecycle_shutdown_completed(&self, reason: &str) {
        self.lifecycle.mark_shutdown_completed(reason);
    }
}
