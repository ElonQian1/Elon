mod admission;
mod cleanup;
mod environment;
mod janitor;
mod lease;
mod paths;
mod reservation;
mod target_lock;
mod telemetry;
mod usage;

#[cfg(test)]
mod tests;

pub(crate) use admission::BuildCachePolicy;
pub(crate) use environment::BuildEnvironment;
pub(crate) use telemetry::{BuildCacheTelemetry, NodeBuildCacheStatus};

use admission::admit;
use anyhow::{anyhow, Result};
use elon_pc_dev_runtime::NodeDataPaths;
use lease::BuildRunLease;
use paths::{prepare_run_directories, resolve_run_paths, BuildRunPaths};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};
use target_lock::{AdmissionLock, TargetLock};

const RUN_OUTCOME_PENDING: u8 = 0;
const RUN_OUTCOME_SUCCESS: u8 = 1;
const RUN_OUTCOME_FAILED: u8 = 2;

pub(crate) struct BuildRunRequest<'a> {
    pub(crate) task_id: &'a str,
    pub(crate) project_id: &'a str,
    pub(crate) cwd: Option<&'a Path>,
}

pub(crate) struct PreparedBuildRun {
    environment: BuildEnvironment,
    paths: BuildRunPaths,
    policy: BuildCachePolicy,
    lease: Option<BuildRunLease>,
    target_lock: Option<TargetLock>,
    telemetry: BuildCacheTelemetry,
    succeeded: bool,
}

impl PreparedBuildRun {
    pub(crate) fn environment(&self) -> &BuildEnvironment {
        &self.environment
    }

    pub(crate) fn finish(&mut self, succeeded: bool) {
        self.succeeded = succeeded;
    }
}

impl Drop for PreparedBuildRun {
    fn drop(&mut self) {
        // Release active ownership first, then enqueue all recursive work. Drop
        // may run on a Tokio worker and must never scan/delete a multi-GB tree.
        self.lease.take();
        self.target_lock.take();
        invalidate_status(&self.paths.root);
        janitor::enqueue(janitor::JanitorJob {
            paths: self.paths.clone(),
            policy: self.policy.clone(),
            succeeded: self.succeeded,
            initial_reclaimed_bytes: self.telemetry.reclaimed_bytes,
        });
    }
}

pub(crate) fn prepare_run(
    data_paths: &NodeDataPaths,
    request: BuildRunRequest<'_>,
) -> Result<PreparedBuildRun> {
    let task_id = request.task_id.trim();
    let project_id = request.project_id.trim();
    if task_id.is_empty() || project_id.is_empty() {
        return Err(anyhow!("构建任务必须包含 task_id 和 project_id"));
    }
    let paths = resolve_run_paths(data_paths, task_id, project_id, request.cwd)?;
    {
        let _prepare_guard = prepare_lock()
            .lock()
            .map_err(|_| anyhow!("构建缓存准备锁已损坏"))?;
        prepare_run_directories(&paths)?;
    }
    let target_lock = TargetLock::acquire(&paths, task_id)?;
    let _admission_lock = AdmissionLock::acquire(&paths, "prepare_run")?;
    let _prepare_guard = prepare_lock()
        .lock()
        .map_err(|_| anyhow!("构建缓存准备锁已损坏"))?;
    prepare_run_directories(&paths)?;
    let policy = BuildCachePolicy::default();
    usage::touch(&paths)?;
    let active_reserved_bytes = reservation::active_reserved_bytes(&paths)?;
    let cleanup = admit(&paths, &policy, active_reserved_bytes)?;
    telemetry::record_cleanup(&paths, &cleanup);
    prepare_run_directories(&paths)?;
    let lease = BuildRunLease::acquire(
        &paths,
        task_id,
        reservation::reservation_for_new_run(&policy),
    )?;
    invalidate_status(&paths.root);
    let environment = BuildEnvironment::for_run(&paths, project_id, task_id);
    let telemetry = telemetry::capture(&paths, cleanup.reclaimed_bytes, &policy);
    telemetry::persist(&paths, &telemetry);
    tracing::info!(
        project = %paths.project_key,
        toolchain = %paths.toolchain_key,
        cache_bytes = telemetry.cache_bytes,
        reclaimed_bytes = telemetry.reclaimed_bytes,
        disk_free_bytes = ?telemetry.disk_free_bytes,
        "PC 节点已准备受治理的项目构建环境"
    );
    Ok(PreparedBuildRun {
        environment,
        paths,
        policy,
        lease: Some(lease),
        target_lock: Some(target_lock),
        telemetry,
        succeeded: false,
    })
}

pub(crate) struct RegisteredCliBuildRun {
    task_id: String,
    run: Option<PreparedBuildRun>,
    outcome: Arc<AtomicU8>,
}

impl Drop for RegisteredCliBuildRun {
    fn drop(&mut self) {
        if let Ok(mut registry) = cli_environment_registry().lock() {
            registry.remove(&self.task_id);
        }
        if let Some(run) = self.run.as_mut() {
            run.finish(self.outcome.load(Ordering::Acquire) == RUN_OUTCOME_SUCCESS);
        }
        self.run.take();
    }
}

pub(crate) fn register_cli_run(
    data_paths: &NodeDataPaths,
    request: BuildRunRequest<'_>,
) -> Result<RegisteredCliBuildRun> {
    let task_id = request.task_id.to_string();
    let run = prepare_run(data_paths, request)?;
    let outcome = Arc::new(AtomicU8::new(RUN_OUTCOME_PENDING));
    let mut registry = cli_environment_registry()
        .lock()
        .map_err(|_| anyhow!("CLI 构建环境注册表锁已损坏"))?;
    if registry.contains_key(&task_id) {
        return Err(anyhow!("CLI 构建环境已经注册: {task_id}"));
    }
    registry.insert(
        task_id.clone(),
        CliRunEntry {
            environment: run.environment().clone(),
            outcome: outcome.clone(),
        },
    );
    drop(registry);
    Ok(RegisteredCliBuildRun {
        task_id,
        run: Some(run),
        outcome,
    })
}

pub(crate) fn cli_run_environment(task_id: &str) -> Option<BuildEnvironment> {
    cli_environment_registry()
        .lock()
        .ok()
        .and_then(|registry| registry.get(task_id).map(|entry| entry.environment.clone()))
}

pub(crate) fn mark_cli_run_outcome(task_id: &str, succeeded: bool) {
    if let Ok(registry) = cli_environment_registry().lock() {
        if let Some(entry) = registry.get(task_id) {
            entry.outcome.store(
                if succeeded {
                    RUN_OUTCOME_SUCCESS
                } else {
                    RUN_OUTCOME_FAILED
                },
                Ordering::Release,
            );
        }
    }
}

pub(crate) fn status(data_paths: &NodeDataPaths) -> NodeBuildCacheStatus {
    let key = data_paths.root().to_path_buf();
    if let Some(status) = cached_status(&key) {
        return status;
    }
    let _capture_guard = status_capture_lock().lock().ok();
    if let Some(status) = cached_status(&key) {
        return status;
    }
    let status = telemetry::capture_root_status(data_paths, &BuildCachePolicy::default());
    if let Ok(mut cache) = status_cache().lock() {
        cache.insert(
            key,
            CachedRootStatus {
                captured: Instant::now(),
                status: status.clone(),
            },
        );
    }
    status
}

/// 不读取 30 秒状态缓存，供数据根切换等并发门禁获取实时 lease 数。
pub(crate) fn active_leases(data_paths: &NodeDataPaths) -> usize {
    lease::active_lease_count(&data_paths.cache().join(".leases"))
}

fn cached_status(root: &Path) -> Option<NodeBuildCacheStatus> {
    status_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(root).cloned())
        .filter(|cached| cached.captured.elapsed() < Duration::from_secs(30))
        .map(|cached| cached.status)
}

pub(crate) fn cleanup_rebuildable(
    data_paths: &NodeDataPaths,
    expected_install_id: &str,
    apply: bool,
) -> Result<crate::node_agent_data_root::CleanupResult> {
    let _admission_lock = AdmissionLock::acquire_root(data_paths.root(), "manual_cleanup")?;
    let _prepare_guard = prepare_lock()
        .lock()
        .map_err(|_| anyhow!("构建缓存准备锁已损坏"))?;
    let current_active_leases = active_leases(data_paths);
    if apply && current_active_leases > 0 {
        return Err(anyhow!(
            "当前仍有 {} 个构建任务使用缓存，拒绝清理",
            current_active_leases
        ));
    }
    let result = crate::node_agent_data_root::cleanup(data_paths, expected_install_id, apply)?;
    if apply {
        invalidate_status(data_paths.root());
    }
    Ok(result)
}

#[derive(Clone)]
struct CachedRootStatus {
    captured: Instant,
    status: NodeBuildCacheStatus,
}

fn status_cache() -> &'static Mutex<HashMap<PathBuf, CachedRootStatus>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedRootStatus>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn status_capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(super) fn invalidate_status(root: &Path) {
    if let Ok(mut cache) = status_cache().lock() {
        cache.remove(root);
    }
}

#[derive(Clone)]
struct CliRunEntry {
    environment: BuildEnvironment,
    outcome: Arc<AtomicU8>,
}

fn cli_environment_registry() -> &'static Mutex<HashMap<String, CliRunEntry>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, CliRunEntry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn prepare_lock() -> &'static Mutex<()> {
    static PREPARE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    PREPARE_LOCK.get_or_init(|| Mutex::new(()))
}
