mod admission;
mod cleanup;
mod environment;
mod lease;
mod paths;
mod telemetry;

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
    path::Path,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex, OnceLock,
    },
};

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
        if self.succeeded {
            let _ = cleanup::remove_managed_path(&self.paths.root, &self.paths.task_temp);
        }
        self.lease.take();
        let final_snapshot = telemetry::capture(
            &self.paths,
            self.telemetry.reclaimed_bytes,
            self.policy.min_free_bytes,
        );
        telemetry::persist(&self.paths, &final_snapshot);
    }
}

pub(crate) fn prepare_run(
    data_paths: &NodeDataPaths,
    request: BuildRunRequest<'_>,
) -> Result<PreparedBuildRun> {
    let _prepare_guard = prepare_lock()
        .lock()
        .map_err(|_| anyhow!("构建缓存准备锁已损坏"))?;
    let task_id = request.task_id.trim();
    let project_id = request.project_id.trim();
    if task_id.is_empty() || project_id.is_empty() {
        return Err(anyhow!("构建任务必须包含 task_id 和 project_id"));
    }
    let paths = resolve_run_paths(data_paths, task_id, project_id, request.cwd)?;
    prepare_run_directories(&paths)?;
    let policy = BuildCachePolicy::default();
    let cleanup = admit(&paths, &policy)?;
    telemetry::record_cleanup(&paths, &cleanup);
    prepare_run_directories(&paths)?;
    let lease = BuildRunLease::acquire(&paths, task_id)?;
    let environment = BuildEnvironment::for_run(&paths, project_id, task_id);
    let telemetry = telemetry::capture(&paths, cleanup.reclaimed_bytes, policy.min_free_bytes);
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
    telemetry::capture_root_status(data_paths, &BuildCachePolicy::default())
}

pub(crate) fn cleanup_rebuildable(
    data_paths: &NodeDataPaths,
    apply: bool,
) -> Result<crate::node_agent_data_root::CleanupResult> {
    let _prepare_guard = prepare_lock()
        .lock()
        .map_err(|_| anyhow!("构建缓存准备锁已损坏"))?;
    let current = status(data_paths);
    if apply && current.active_leases > 0 {
        return Err(anyhow!(
            "当前仍有 {} 个构建任务使用缓存，拒绝清理",
            current.active_leases
        ));
    }
    crate::node_agent_data_root::cleanup(data_paths, apply)
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

fn prepare_lock() -> &'static Mutex<()> {
    static PREPARE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    PREPARE_LOCK.get_or_init(|| Mutex::new(()))
}
