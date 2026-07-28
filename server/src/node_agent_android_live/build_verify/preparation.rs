use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use tokio::sync::{Mutex, RwLock};

use super::gradle::{validate_debug_application_id_suffix, validate_package_name};
use super::{
    bootstrap_debug_runtime_with_reporter, PrepareDebugRuntimeRequest, PrepareDebugRuntimeResult,
};
use crate::node_agent_android_live::broker::LiveUiBroker;

const MAX_EVIDENCE: usize = 64;
const MAX_EVIDENCE_DETAIL_CHARS: usize = 1_200;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PreparationKey {
    slot_id: String,
    package_name: String,
    device_id: String,
    lkg_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationEvidence {
    pub(crate) phase: String,
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) recorded_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareDebugRuntimeProgress {
    pub(crate) operation_id: String,
    pub(crate) status: String,
    pub(crate) phase: String,
    pub(crate) retry_after_ms: Option<u64>,
    pub(crate) source_revision: Option<String>,
    pub(crate) integration_revision: Option<String>,
    pub(crate) generation: u64,
    pub(crate) commits: Vec<String>,
    pub(crate) evidence: Vec<PreparationEvidence>,
    pub(crate) result: Option<PrepareDebugRuntimeResult>,
    pub(crate) error: Option<String>,
}

struct PreparationState {
    owner_session_id: String,
    device_id: String,
    operation_id: String,
    status: String,
    phase: String,
    source_revision: Option<String>,
    integration_revision: Option<String>,
    generation: u64,
    commits: Vec<String>,
    evidence: Vec<PreparationEvidence>,
    result: Option<PrepareDebugRuntimeResult>,
    error: Option<String>,
}

impl PreparationState {
    fn progress(&self) -> PrepareDebugRuntimeProgress {
        PrepareDebugRuntimeProgress {
            operation_id: self.operation_id.clone(),
            status: self.status.clone(),
            phase: self.phase.clone(),
            retry_after_ms: (self.status == "IN_PROGRESS").then_some(2_000),
            source_revision: self.source_revision.clone(),
            integration_revision: self.integration_revision.clone(),
            generation: self.generation,
            commits: self.commits.clone(),
            evidence: self.evidence.clone(),
            result: self.result.clone(),
            error: self.error.clone(),
        }
    }
}

#[derive(Clone)]
pub(super) struct PreparationReporter {
    state: Arc<RwLock<PreparationState>>,
}

impl PreparationReporter {
    pub(super) async fn phase(&self, phase: &str, detail: impl AsRef<str>) {
        let mut state = self.state.write().await;
        state.phase = phase.to_string();
        push_evidence(&mut state, phase, "IN_PROGRESS", detail.as_ref());
    }

    pub(super) async fn evidence(&self, phase: &str, status: &str, detail: impl AsRef<str>) {
        let mut state = self.state.write().await;
        push_evidence(&mut state, phase, status, detail.as_ref());
    }

    async fn complete(&self, result: PrepareDebugRuntimeResult) {
        let mut state = self.state.write().await;
        state.status = "COMPLETED".to_string();
        state.phase = "LIVE".to_string();
        push_evidence(
            &mut state,
            "RUNTIME_HANDSHAKE",
            "PASSED",
            format!(
                "connected={} runtimeBuildId={} nodeCount={}",
                result.build.runtime_connected,
                result.build.runtime_build_id.as_deref().unwrap_or("none"),
                result.build.node_count
            )
            .as_str(),
        );
        state.source_revision = result
            .integration
            .source_revision
            .clone()
            .or_else(|| state.source_revision.clone());
        state.integration_revision = result.integration.integration_revision.clone();
        state.commits = result
            .integration
            .contributions
            .iter()
            .map(|contribution| contribution.commit_sha.clone())
            .collect();
        state.result = Some(result);
        state.error = None;
    }

    async fn fail(&self, error: &anyhow::Error) {
        let mut state = self.state.write().await;
        state.status = "FAILED".to_string();
        let phase = state.phase.clone();
        let detail = format!("phase={phase}; error={error:#}");
        push_evidence(&mut state, &phase, "FAILED", &detail);
        state.error = Some(detail);
    }
}

#[derive(Default)]
pub(crate) struct PreparationRegistry {
    operations: Mutex<HashMap<PreparationKey, Arc<RwLock<PreparationState>>>>,
}

impl PreparationRegistry {
    pub(crate) async fn busy_device_ids_except(
        &self,
        owner_session_id: &str,
    ) -> std::collections::HashSet<String> {
        let states = self
            .operations
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut busy = std::collections::HashSet::new();
        for state in states {
            let state = state.read().await;
            if state.owner_session_id != owner_session_id && state.status == "IN_PROGRESS" {
                busy.insert(state.device_id.clone());
            }
        }
        busy
    }

    pub(crate) async fn poll_or_start(
        &self,
        broker: Arc<LiveUiBroker>,
        mut request: PrepareDebugRuntimeRequest,
        host_port: u16,
        restart: bool,
        owner_session_id: &str,
    ) -> Result<PrepareDebugRuntimeProgress> {
        let mut operations = self.operations.lock().await;
        let mut plan = prepare_integration_plan(&broker, &request)?;
        let key = PreparationKey {
            slot_id: plan.slot_id.clone(),
            package_name: plan.package_name.clone(),
            device_id: request.device_id.trim().to_string(),
            lkg_enabled: plan.lkg_enabled,
        };
        if let Some(existing) = operations.get(&key).cloned() {
            let progress = existing.read().await.progress();
            let source_revision = Some(plan.source_revision.clone());
            let source_unchanged = progress.source_revision == source_revision;
            if progress.generation == plan.generation
                && (progress.status == "IN_PROGRESS" || (!restart && source_unchanged))
            {
                return observable_progress(&broker, &plan.slot_id, progress);
            }
            if should_allocate_restart_generation(
                &progress,
                plan.generation,
                restart,
                source_unchanged,
            ) {
                plan = broker.debug_integration.restart_failed_generation(&plan)?;
            }
        }

        request.integration_plan = Some(plan.clone());
        let operation_id = format!("runtime_prepare_{}", uuid::Uuid::new_v4().simple());
        let state = Arc::new(RwLock::new(PreparationState {
            owner_session_id: owner_session_id.to_string(),
            device_id: request.device_id.trim().to_string(),
            operation_id,
            status: "IN_PROGRESS".to_string(),
            phase: "QUEUED".to_string(),
            source_revision: Some(plan.source_revision.clone()),
            integration_revision: plan.integration_revision.clone(),
            generation: plan.generation,
            commits: plan.contributions.clone(),
            evidence: vec![PreparationEvidence {
                phase: "QUEUED".to_string(),
                status: "IN_PROGRESS".to_string(),
                detail: format!(
                    "device={} package={}；后台准备已启动，可重复调用本工具读取阶段进度",
                    request.device_id.trim(),
                    key.package_name
                ),
                recorded_at: Utc::now().to_rfc3339(),
            }],
            result: None,
            error: None,
        }));
        operations.insert(key, state.clone());
        drop(operations);

        let reporter = PreparationReporter {
            state: state.clone(),
        };
        tokio::spawn(async move {
            match bootstrap_debug_runtime_with_reporter(
                &broker,
                request,
                host_port,
                Some(&reporter),
            )
            .await
            {
                Ok(result) => reporter.complete(result).await,
                Err(error) => reporter.fail(&error).await,
            }
        });
        let progress = state.read().await.progress();
        Ok(progress)
    }
}

fn should_allocate_restart_generation(
    progress: &PrepareDebugRuntimeProgress,
    plan_generation: u64,
    restart: bool,
    source_unchanged: bool,
) -> bool {
    progress.status == "FAILED"
        && progress.generation == plan_generation
        && (restart || !source_unchanged)
}

fn observable_progress(
    broker: &LiveUiBroker,
    slot_id: &str,
    mut progress: PrepareDebugRuntimeProgress,
) -> Result<PrepareDebugRuntimeProgress> {
    if let Some(status) = broker.debug_integration.status(slot_id)? {
        if status.desired_generation == progress.generation {
            progress.integration_revision = status.integration_revision;
            progress.commits = status
                .contributions
                .into_iter()
                .map(|contribution| contribution.commit_sha)
                .collect();
        }
    }
    Ok(progress)
}

fn prepare_integration_plan(
    broker: &LiveUiBroker,
    request: &PrepareDebugRuntimeRequest,
) -> Result<crate::node_agent_android_live::debug_integration::DebugIntegrationPlan> {
    let project_root = PathBuf::from(request.project_root.trim())
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {}", request.project_root.trim()))?
        .to_string_lossy()
        .to_string();
    let requested_base_package_name = validate_package_name(request.base_package_name.trim())?;
    let base_package_name =
        crate::node_agent_android_live::debug_base_package_name(requested_base_package_name);
    let install_id = broker
        .node_install_id()
        .context("PC 节点缺少稳定安装标识，拒绝创建调试集成候选")?;
    let suffix = crate::node_agent_android_live::resolve_debug_application_id_suffix(
        request.debug_application_id_suffix.trim(),
        install_id,
        request.device_id.trim(),
        request.isolated_emulator_package,
    )?;
    validate_debug_application_id_suffix(&suffix)?;
    let package_name = format!("{base_package_name}{suffix}");
    let project_id = request
        .lease
        .as_ref()
        .map(|lease| lease.project_id.as_str())
        .unwrap_or(project_root.as_str());
    let device_identity = request
        .lease
        .as_ref()
        .map(|lease| lease.hardware_serial.as_str())
        .unwrap_or(request.device_id.trim());
    broker.debug_integration.register_candidate(
        &project_root,
        project_id,
        device_identity,
        &package_name,
        request.candidate.as_ref(),
        "compat-mcp-prepare",
        Some(request.lkg_enabled),
    )
}

fn push_evidence(state: &mut PreparationState, phase: &str, status: &str, detail: &str) {
    if state.evidence.len() >= MAX_EVIDENCE {
        state.evidence.remove(0);
    }
    state.evidence.push(PreparationEvidence {
        phase: phase.to_string(),
        status: status.to_string(),
        detail: truncate_chars(detail, MAX_EVIDENCE_DETAIL_CHARS),
        recorded_at: Utc::now().to_rfc3339(),
    });
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("…");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reporter_keeps_failure_phase_and_bounded_evidence() {
        let state = Arc::new(RwLock::new(PreparationState {
            owner_session_id: "session-a".into(),
            device_id: "device-a".into(),
            operation_id: "op-1".into(),
            status: "IN_PROGRESS".into(),
            phase: "BUILD".into(),
            source_revision: Some("rev-1".into()),
            integration_revision: Some("integrated-1".into()),
            generation: 1,
            commits: vec!["commit-1".into()],
            evidence: Vec::new(),
            result: None,
            error: None,
        }));
        let reporter = PreparationReporter {
            state: state.clone(),
        };
        for index in 0..80 {
            reporter
                .evidence("BUILD", "IN_PROGRESS", format!("event-{index}"))
                .await;
        }
        reporter.fail(&anyhow::anyhow!("gradle failed")).await;
        let progress = state.read().await.progress();
        assert_eq!(progress.status, "FAILED");
        assert_eq!(progress.phase, "BUILD");
        assert!(progress.evidence.len() <= MAX_EVIDENCE);
        let observable = serde_json::to_value(&progress).unwrap();
        assert_eq!(observable["operationId"], "op-1");
        assert_eq!(observable["sourceRevision"], "rev-1");
        assert_eq!(observable["integrationRevision"], "integrated-1");
        assert_eq!(observable["generation"], 1);
        assert_eq!(observable["commits"], serde_json::json!(["commit-1"]));
        assert!(should_allocate_restart_generation(&progress, 1, true, true));
        assert!(!should_allocate_restart_generation(
            &progress, 1, false, true
        ));
        assert!(progress.error.unwrap().contains("phase=BUILD"));
    }
}
