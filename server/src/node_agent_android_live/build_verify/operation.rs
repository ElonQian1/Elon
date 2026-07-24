use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{bail, Result};
use chrono::Utc;
use serde::Serialize;
use tokio::sync::{Mutex, RwLock};

use super::{build_and_verify_with_reporter, BuildVerifyRequest, BuildVerifyResult};
use crate::node_agent_android_live::broker::LiveUiBroker;

const MAX_OPERATIONS: usize = 64;
const MAX_EVIDENCE: usize = 64;
const MAX_EVIDENCE_DETAIL_CHARS: usize = 1_200;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildVerifyEvidence {
    pub(crate) phase: String,
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) recorded_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildVerifyOperationProgress {
    pub(crate) operation_id: String,
    pub(crate) status: String,
    pub(crate) phase: String,
    pub(crate) retry_after_ms: Option<u64>,
    pub(crate) evidence: Vec<BuildVerifyEvidence>,
    pub(crate) result: Option<BuildVerifyResult>,
    pub(crate) error: Option<String>,
}

struct BuildVerifyOperationState {
    session_id: String,
    operation_id: String,
    status: String,
    phase: String,
    evidence: Vec<BuildVerifyEvidence>,
    result: Option<BuildVerifyResult>,
    error: Option<String>,
}

impl BuildVerifyOperationState {
    fn progress(&self) -> BuildVerifyOperationProgress {
        BuildVerifyOperationProgress {
            operation_id: self.operation_id.clone(),
            status: self.status.clone(),
            phase: self.phase.clone(),
            retry_after_ms: (self.status == "IN_PROGRESS").then_some(2_000),
            evidence: self.evidence.clone(),
            result: self.result.clone(),
            error: self.error.clone(),
        }
    }
}

#[derive(Clone)]
pub(super) struct BuildVerifyOperationReporter {
    state: Arc<RwLock<BuildVerifyOperationState>>,
}

impl BuildVerifyOperationReporter {
    pub(super) async fn phase(&self, phase: &str, detail: impl AsRef<str>) {
        let mut state = self.state.write().await;
        state.phase = phase.to_string();
        push_evidence(&mut state, phase, "IN_PROGRESS", detail.as_ref());
    }

    pub(super) async fn evidence(&self, phase: &str, detail: impl AsRef<str>) {
        let mut state = self.state.write().await;
        push_evidence(&mut state, phase, "PASSED", detail.as_ref());
    }

    async fn complete(&self, result: BuildVerifyResult) {
        let mut state = self.state.write().await;
        state.status = "SUCCEEDED".to_string();
        state.phase = "COMPLETED".to_string();
        push_evidence(
            &mut state,
            "COMPLETED",
            "PASSED",
            "构建验收后台操作已到达可轮询终态",
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

pub(super) async fn report_phase(
    reporter: Option<&BuildVerifyOperationReporter>,
    phase: &str,
    detail: impl AsRef<str>,
) {
    if let Some(reporter) = reporter {
        reporter.phase(phase, detail).await;
    }
}

pub(super) async fn report_evidence(
    reporter: Option<&BuildVerifyOperationReporter>,
    phase: &str,
    detail: impl AsRef<str>,
) {
    if let Some(reporter) = reporter {
        reporter.evidence(phase, detail).await;
    }
}

#[derive(Default)]
pub(crate) struct BuildVerifyOperationRegistry {
    operations: Mutex<HashMap<String, Arc<RwLock<BuildVerifyOperationState>>>>,
}

impl BuildVerifyOperationRegistry {
    pub(crate) async fn start(
        &self,
        broker: Arc<LiveUiBroker>,
        session_id: String,
        request: BuildVerifyRequest,
        host_port: u16,
    ) -> Result<BuildVerifyOperationProgress> {
        broker.session(&session_id).await?;
        let operation_id = format!("ui_build_verify_{}", uuid::Uuid::new_v4().simple());
        let state = Arc::new(RwLock::new(BuildVerifyOperationState {
            session_id: session_id.clone(),
            operation_id: operation_id.clone(),
            status: "IN_PROGRESS".to_string(),
            phase: "QUEUED".to_string(),
            evidence: vec![BuildVerifyEvidence {
                phase: "QUEUED".to_string(),
                status: "IN_PROGRESS".to_string(),
                detail: "后台构建验收已启动；MCP 客户端断开不会取消此操作".to_string(),
                recorded_at: Utc::now().to_rfc3339(),
            }],
            result: None,
            error: None,
        }));
        {
            let mut operations = self.operations.lock().await;
            evict_terminal_operations(&mut operations).await;
            if operations.len() >= MAX_OPERATIONS {
                bail!("当前后台构建验收操作过多，请等待既有 operationId 到达终态");
            }
            operations.insert(operation_id, state.clone());
        }
        let initial_progress = state.read().await.progress();

        let reporter = BuildVerifyOperationReporter {
            state: state.clone(),
        };
        tokio::spawn(async move {
            match build_and_verify_with_reporter(
                broker.as_ref(),
                &session_id,
                request,
                host_port,
                &reporter,
            )
            .await
            {
                Ok(result) => reporter.complete(result).await,
                Err(error) => reporter.fail(&error).await,
            }
        });
        Ok(initial_progress)
    }

    pub(crate) async fn poll(
        &self,
        session_id: &str,
        operation_id: &str,
    ) -> Result<BuildVerifyOperationProgress> {
        validate_operation_id(operation_id)?;
        let state = self
            .operations
            .lock()
            .await
            .get(operation_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("未知 ui_build_and_verify operationId"))?;
        let state = state.read().await;
        if state.session_id != session_id {
            bail!("ui_build_and_verify operationId 不属于当前 Live UI 会话");
        }
        Ok(state.progress())
    }
}

async fn evict_terminal_operations(
    operations: &mut HashMap<String, Arc<RwLock<BuildVerifyOperationState>>>,
) {
    if operations.len() < MAX_OPERATIONS {
        return;
    }
    let entries = operations
        .iter()
        .map(|(id, state)| (id.clone(), state.clone()))
        .collect::<Vec<_>>();
    for (id, state) in entries {
        if state.read().await.status != "IN_PROGRESS" {
            operations.remove(&id);
            if operations.len() < MAX_OPERATIONS {
                break;
            }
        }
    }
}

fn validate_operation_id(value: &str) -> Result<()> {
    let suffix = value
        .strip_prefix("ui_build_verify_")
        .ok_or_else(|| anyhow::anyhow!("ui_build_and_verify operationId 格式无效"))?;
    if suffix.len() != 32 || !suffix.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("ui_build_and_verify operationId 格式无效");
    }
    Ok(())
}

fn push_evidence(state: &mut BuildVerifyOperationState, phase: &str, status: &str, detail: &str) {
    if state.evidence.len() >= MAX_EVIDENCE {
        state.evidence.remove(0);
    }
    state.evidence.push(BuildVerifyEvidence {
        phase: phase.to_string(),
        status: status.to_string(),
        detail: truncate_chars(detail, MAX_EVIDENCE_DETAIL_CHARS),
        recorded_at: Utc::now().to_rfc3339(),
    });
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push('…');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> Arc<RwLock<BuildVerifyOperationState>> {
        Arc::new(RwLock::new(BuildVerifyOperationState {
            session_id: "session-1".into(),
            operation_id: "ui_build_verify_0123456789abcdef0123456789abcdef".into(),
            status: "IN_PROGRESS".into(),
            phase: "INSTALL".into(),
            evidence: Vec::new(),
            result: None,
            error: None,
        }))
    }

    #[tokio::test]
    async fn operation_remains_pollable_after_the_start_call_returns() {
        let state = state();
        let initial = state.read().await.progress();
        assert_eq!(initial.status, "IN_PROGRESS");
        assert_eq!(initial.retry_after_ms, Some(2_000));
        let reporter = BuildVerifyOperationReporter {
            state: state.clone(),
        };
        reporter
            .phase("RETURN_TO_PREVIEW", "PreviewHost 已重新激活")
            .await;
        reporter
            .evidence("TREE_REFRESH", "节点树已刷新 revision=9 nodeCount=42")
            .await;
        reporter.phase("SCREENSHOT", "正在捕获最终真机截图").await;
        reporter.fail(&anyhow::anyhow!("capture timed out")).await;

        let progress = state.read().await.progress();
        assert_eq!(initial.status, "IN_PROGRESS");
        assert_eq!(progress.status, "FAILED");
        assert_eq!(progress.phase, "SCREENSHOT");
        assert!(progress.retry_after_ms.is_none());
        assert!(progress
            .evidence
            .iter()
            .any(|entry| entry.phase == "RETURN_TO_PREVIEW"));
        assert!(progress
            .evidence
            .iter()
            .any(|entry| entry.phase == "TREE_REFRESH"));
        assert!(progress.error.unwrap().contains("capture timed out"));
    }

    #[test]
    fn operation_id_validation_is_fail_closed() {
        assert!(validate_operation_id("ui_build_verify_0123456789abcdef0123456789abcdef").is_ok());
        assert!(validate_operation_id("ui_build_verify_../../escape").is_err());
    }
}
