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
use crate::node_agent_android_live::fit_run::workspace_fingerprint;

const MAX_EVIDENCE: usize = 64;
const MAX_EVIDENCE_DETAIL_CHARS: usize = 1_200;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PreparationKey {
    project_root: String,
    package_name: String,
    device_id: String,
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
    pub(crate) evidence: Vec<PreparationEvidence>,
    pub(crate) result: Option<PrepareDebugRuntimeResult>,
    pub(crate) error: Option<String>,
}

struct PreparationState {
    operation_id: String,
    status: String,
    phase: String,
    source_revision: Option<String>,
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
    pub(crate) async fn poll_or_start(
        &self,
        broker: Arc<LiveUiBroker>,
        request: PrepareDebugRuntimeRequest,
        host_port: u16,
        restart: bool,
    ) -> Result<PrepareDebugRuntimeProgress> {
        let key = preparation_key(&request)?;
        let source_revision = workspace_fingerprint(&key.project_root)?;
        if let Some(existing) = self.operations.lock().await.get(&key).cloned() {
            let progress = existing.read().await.progress();
            let source_unchanged = progress.source_revision == source_revision;
            if progress.status == "IN_PROGRESS" || (!restart && source_unchanged) {
                return Ok(progress);
            }
        }

        let operation_id = format!("runtime_prepare_{}", uuid::Uuid::new_v4().simple());
        let state = Arc::new(RwLock::new(PreparationState {
            operation_id,
            status: "IN_PROGRESS".to_string(),
            phase: "QUEUED".to_string(),
            source_revision,
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
        self.operations.lock().await.insert(key, state.clone());

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

fn preparation_key(request: &PrepareDebugRuntimeRequest) -> Result<PreparationKey> {
    let project_root = PathBuf::from(request.project_root.trim())
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {}", request.project_root.trim()))?
        .to_string_lossy()
        .to_string();
    let base_package_name = validate_package_name(request.base_package_name.trim())?;
    let suffix = validate_debug_application_id_suffix(request.debug_application_id_suffix.trim())?;
    Ok(PreparationKey {
        project_root,
        package_name: format!("{base_package_name}{suffix}"),
        device_id: request.device_id.trim().to_string(),
    })
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
            operation_id: "op-1".into(),
            status: "IN_PROGRESS".into(),
            phase: "BUILD".into(),
            source_revision: Some("rev-1".into()),
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
        assert!(progress.error.unwrap().contains("phase=BUILD"));
        assert!(progress.evidence.len() <= MAX_EVIDENCE);
    }
}
