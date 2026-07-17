use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{CapabilityGapDocument, CapabilityGapStatus};
use crate::node_agent_android_live::broker::LiveUiSession;
use crate::node_agent_android_live::fit_run::{workspace_fingerprint, FitRunDocument};
use crate::node_agent_android_live::protocol::LiveSessionView;

const LOSS_EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum GapExecutionMode {
    BusinessThread,
    EvolutionThread,
    LegacySameThread,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum GapDeliveryImpact {
    DeliveryBlocking,
    DeliveryNonBlocking,
    EvolutionOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BusinessDeliveryEvidence {
    source_revision: String,
    source_writeback_verified: bool,
    patch_free_build_verified: bool,
    visual_loss: f64,
    max_visual_loss: f64,
    source_parity_loss: f64,
    max_source_parity_loss: f64,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CapabilityGapHandoffPolicy {
    execution_mode: GapExecutionMode,
    delivery_impact: GapDeliveryImpact,
    #[serde(default)]
    origin_gap_id: Option<String>,
    #[serde(default)]
    origin_thread_id: Option<String>,
    #[serde(default)]
    business_delivery: Option<BusinessDeliveryEvidence>,
}

impl Default for CapabilityGapHandoffPolicy {
    fn default() -> Self {
        Self {
            execution_mode: GapExecutionMode::LegacySameThread,
            delivery_impact: GapDeliveryImpact::DeliveryBlocking,
            origin_gap_id: None,
            origin_thread_id: None,
            business_delivery: None,
        }
    }
}

impl CapabilityGapHandoffPolicy {
    pub(super) fn from_report(arguments: &Value) -> Result<Self> {
        let execution_mode = match arguments
            .get("executionMode")
            .and_then(Value::as_str)
            .unwrap_or("BUSINESS_THREAD")
        {
            "BUSINESS_THREAD" => GapExecutionMode::BusinessThread,
            "EVOLUTION_THREAD" => GapExecutionMode::EvolutionThread,
            value => bail!("不支持的 executionMode: {value}"),
        };
        let delivery_impact = match arguments
            .get("deliveryImpact")
            .and_then(Value::as_str)
            .unwrap_or("DELIVERY_BLOCKING")
        {
            "DELIVERY_BLOCKING" => GapDeliveryImpact::DeliveryBlocking,
            "DELIVERY_NON_BLOCKING" => GapDeliveryImpact::DeliveryNonBlocking,
            "EVOLUTION_ONLY" => GapDeliveryImpact::EvolutionOnly,
            value => bail!("不支持的 deliveryImpact: {value}"),
        };
        let business_delivery = arguments
            .get("businessDelivery")
            .filter(|value| !value.is_null())
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .context("businessDelivery 格式非法")?;
        let policy = Self {
            execution_mode,
            delivery_impact,
            origin_gap_id: optional_bounded_text(arguments, "originGapId", 128)?,
            origin_thread_id: optional_bounded_text(arguments, "originThreadId", 128)?,
            business_delivery,
        };
        policy.validate_shape()?;
        Ok(policy)
    }

    pub(super) async fn validate_report(
        &self,
        session: &LiveUiSession,
        fit_run_id: Option<&str>,
    ) -> Result<()> {
        self.validate_shape()?;
        if self.delivery_impact != GapDeliveryImpact::DeliveryNonBlocking {
            return Ok(());
        }
        if fit_run_id.is_none() {
            bail!("DELIVERY_NON_BLOCKING 必须绑定 fitRunId");
        }
        let evidence = self
            .business_delivery
            .as_ref()
            .ok_or_else(|| anyhow!("缺少 businessDelivery"))?;
        evidence.validate_thresholds()?;
        let project_root = session
            .project_root
            .as_deref()
            .ok_or_else(|| anyhow!("UI capability gap 未绑定项目目录"))?;
        let source_revision = workspace_fingerprint(project_root)?
            .ok_or_else(|| anyhow!("DELIVERY_NON_BLOCKING 需要 Git 源码 Revision"))?;
        if evidence.source_revision != source_revision {
            bail!("businessDelivery.sourceRevision 不是当前工作区 Revision");
        }
        validate_live_proof(&session.view().await, evidence)
    }

    fn validate_shape(&self) -> Result<()> {
        match (self.execution_mode, self.delivery_impact) {
            (GapExecutionMode::BusinessThread, GapDeliveryImpact::DeliveryBlocking) => {}
            (GapExecutionMode::BusinessThread, GapDeliveryImpact::DeliveryNonBlocking) => {
                self.business_delivery
                    .as_ref()
                    .ok_or_else(|| anyhow!("DELIVERY_NON_BLOCKING 必须提供 businessDelivery"))?
                    .validate_thresholds()?;
            }
            (GapExecutionMode::EvolutionThread, GapDeliveryImpact::EvolutionOnly) => {
                if self.origin_gap_id.is_none() {
                    bail!("EVOLUTION_THREAD 必须提供 originGapId");
                }
                if self.business_delivery.is_some() {
                    bail!("EVOLUTION_THREAD 不得递归声明 businessDelivery");
                }
            }
            (GapExecutionMode::LegacySameThread, _) => {}
            _ => bail!("executionMode 与 deliveryImpact 不匹配"),
        }
        Ok(())
    }

    pub(super) fn is_business_thread(&self) -> bool {
        self.execution_mode == GapExecutionMode::BusinessThread
    }

    pub(super) fn is_evolution_thread(&self) -> bool {
        self.execution_mode == GapExecutionMode::EvolutionThread
    }

    pub(super) fn thread_handoff(&self, gap: &CapabilityGapDocument) -> Value {
        if !self.is_business_thread() {
            return Value::Null;
        }
        let report_arguments = json!({
            "taskId": gap.task_id,
            "executionMode": "EVOLUTION_THREAD",
            "deliveryImpact": "EVOLUTION_ONLY",
            "originGapId": gap.gap_id,
            "originThreadId": self.origin_thread_id,
            "missingCapabilities": gap.missing_capabilities,
            "evidence": gap.evidence,
            "proposedChanges": gap.proposed_changes,
            "resumeTarget": gap.resume_target,
        });
        json!({
            "target": "CODEX_DESKTOP_USER_THREAD",
            "environment": "WORKTREE",
            "waitForOrigin": false,
            "priority": "BACKGROUND_EVOLUTION",
            "suggestedTitle": format!("UI 平台进化 · {}", gap.gap_id),
            "originGapId": gap.gap_id,
            "originThreadId": self.origin_thread_id,
            "reportArguments": report_arguments,
            "resourcePolicy": {
                "foregroundUiTasksHavePriority": true,
                "serialize": ["REAL_ANDROID_RENDERER", "NODE_AGENT_PUBLISH", "NODE_AGENT_RESTART"],
                "whileForegroundUiTaskActive": "WAIT_WITHOUT_HOLDING_SHARED_RESOURCES",
                "deviceAuthorizationFailure": "STOP_AND_REQUEST_HUMAN"
            },
            "prompt": format!(
                "这是从 UI 业务任务分流出的后台平台进化任务。先按项目 WF-START 在独立 Worktree 预检，再调用 ui_report_capability_gap，并原样使用 reportArguments。只修复平台能力缺口：{}。前台 UI 任务优先；存在前台 UI 工作时，不占用真机 Renderer、不发布或重启节点，等待后再继续。完成升级、测试、发布与 RECHECK_PASSED 后，把 commit/version/验证结果通知原任务 {}；不要重做业务 UI。",
                gap.missing_capabilities.join(", "),
                self.origin_thread_id.as_deref().unwrap_or("（未提供 originThreadId）")
            )
        })
    }
}

impl BusinessDeliveryEvidence {
    fn validate_thresholds(&self) -> Result<()> {
        if self.source_revision.trim().is_empty()
            || self.source_revision.len() > 256
            || !self.source_writeback_verified
            || !self.patch_free_build_verified
            || self.reason.trim().is_empty()
            || self.reason.len() > 2_000
        {
            bail!("businessDelivery 缺少源码写回、无补丁构建或原因证据");
        }
        for (loss, limit) in [
            (self.visual_loss, self.max_visual_loss),
            (self.source_parity_loss, self.max_source_parity_loss),
        ] {
            if !loss.is_finite() || !limit.is_finite() || loss < 0.0 || limit < 0.0 || loss > limit
            {
                bail!("businessDelivery loss 必须是通过阈值的非负有限数");
            }
        }
        Ok(())
    }
}

fn validate_live_proof(view: &LiveSessionView, evidence: &BusinessDeliveryEvidence) -> Result<()> {
    let proof = view
        .source_proof
        .as_ref()
        .ok_or_else(|| anyhow!("DELIVERY_NON_BLOCKING 缺少新鲜的 Runtime 源码证明"))?;
    if !view.connected
        || view.history_count != 0
        || view.redo_count != 0
        || view.runtime_build_id.is_none()
        || proof.runtime_build_id != view.runtime_build_id
        || proof.source_revision != evidence.source_revision
        || (proof.source_parity_loss - evidence.source_parity_loss).abs() > LOSS_EPSILON
        || proof.source_parity_loss > evidence.max_source_parity_loss
    {
        bail!("DELIVERY_NON_BLOCKING 必须绑定当前无 Patch 历史、同 build、同源码 Revision 的 Runtime 证明");
    }
    Ok(())
}

fn optional_bounded_text(value: &Value, field: &str, max: usize) -> Result<Option<String>> {
    let Some(text) = value.get(field).and_then(Value::as_str) else {
        return Ok(None);
    };
    let text = text.trim();
    if text.is_empty() || text.len() > max {
        bail!("{field} 非法");
    }
    Ok(Some(text.to_string()))
}

#[derive(Debug, Clone)]
pub(crate) struct DelegatedCapabilityGap {
    gap_id: String,
    fit_run_id: Option<String>,
    missing_capabilities: Vec<String>,
    delivery_impact: GapDeliveryImpact,
    business_delivery: Option<BusinessDeliveryEvidence>,
    platform_view: Value,
}

impl DelegatedCapabilityGap {
    pub(super) fn from_gap(gap: &CapabilityGapDocument) -> Self {
        Self {
            gap_id: gap.gap_id.clone(),
            fit_run_id: gap.fit_run_id.clone(),
            missing_capabilities: gap.missing_capabilities.clone(),
            delivery_impact: gap.delegation.delivery_impact,
            business_delivery: gap.delegation.business_delivery.clone(),
            platform_view: json!({
                "pending": true,
                "gapId": gap.gap_id,
                "status": gap.status,
                "deliveryImpact": gap.delegation.delivery_impact,
                "threadHandoff": gap.delegation.thread_handoff(gap),
            }),
        }
    }

    pub(crate) fn is_nonblocking(&self) -> bool {
        self.delivery_impact == GapDeliveryImpact::DeliveryNonBlocking
    }

    pub(crate) fn covers_capability_result(&self, capabilities: &Value) -> bool {
        match capabilities["status"].as_str() {
            Some("READY") => true,
            Some("PLATFORM_GAP") => capabilities["missing"].as_array().is_some_and(|values| {
                !values.is_empty()
                    && values.iter().all(|value| {
                        value.as_str().is_some_and(|name| {
                            self.missing_capabilities.iter().any(|item| item == name)
                        })
                    })
            }),
            _ => false,
        }
    }

    pub(crate) fn evidence_matches_source(&self, source_revision: Option<&str>) -> bool {
        self.business_delivery.as_ref().is_some_and(|evidence| {
            source_revision.is_some_and(|revision| revision == evidence.source_revision)
        })
    }

    pub(crate) fn accepts_fit_run(&self, run: &FitRunDocument, task_id: &str) -> bool {
        let Some(evidence) = self.business_delivery.as_ref() else {
            return false;
        };
        let Some(candidate) = run.current.as_ref() else {
            return false;
        };
        self.fit_run_id.as_deref() == Some(run.run_id.as_str())
            && run.task_id.as_deref() == Some(task_id)
            && run.target_reached()
            && run.source_verified()
            && candidate.source_revision.as_deref() == Some(evidence.source_revision.as_str())
            && (candidate.score.overall_loss - evidence.visual_loss).abs() <= LOSS_EPSILON
            && candidate.score.overall_loss <= evidence.max_visual_loss
            && candidate.source_parity_loss.is_some_and(|loss| {
                (loss - evidence.source_parity_loss).abs() <= LOSS_EPSILON
                    && loss <= evidence.max_source_parity_loss
            })
    }

    pub(crate) fn fit_run_id(&self) -> Option<&str> {
        self.fit_run_id.as_deref()
    }

    pub(crate) fn platform_view(&self) -> Value {
        self.platform_view.clone()
    }

    pub(crate) fn gap_id(&self) -> &str {
        &self.gap_id
    }
}

pub(super) fn is_delegated_business_gap(gap: &CapabilityGapDocument, task_id: &str) -> bool {
    gap.task_id == task_id
        && gap.status == CapabilityGapStatus::Deferred
        && gap.delegation.is_business_thread()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_android_live::protocol::LiveSourceProofView;

    fn evidence() -> BusinessDeliveryEvidence {
        BusinessDeliveryEvidence {
            source_revision: "revision-1".into(),
            source_writeback_verified: true,
            patch_free_build_verified: true,
            visual_loss: 0.02,
            max_visual_loss: 0.035,
            source_parity_loss: 0.01,
            max_source_parity_loss: 0.035,
            reason: "业务视觉已通过，平台仅缺少免重复安装能力".into(),
        }
    }

    #[test]
    fn nonblocking_evidence_requires_current_patch_free_runtime_proof() {
        let mut view = LiveSessionView {
            id: "session".into(),
            device_id: "device".into(),
            package_name: "package".into(),
            project_root: None,
            device_port: 0,
            created_at: "now".into(),
            connected: true,
            runtime_build_id: Some("build-1".into()),
            runtime_version: None,
            tree_revision: 1,
            node_count: 1,
            history_count: 0,
            redo_count: 0,
            source_proof: Some(LiveSourceProofView {
                source_revision: "revision-1".into(),
                runtime_build_id: Some("build-1".into()),
                source_parity_loss: 0.01,
                verified_at: "now".into(),
            }),
            last_seen_at: None,
            last_error: None,
        };
        assert!(validate_live_proof(&view, &evidence()).is_ok());
        view.history_count = 1;
        assert!(validate_live_proof(&view, &evidence()).is_err());
    }

    #[test]
    fn report_modes_cannot_recurse_or_mix_delivery_impact() {
        assert!(CapabilityGapHandoffPolicy::from_report(&json!({
            "executionMode":"BUSINESS_THREAD",
            "deliveryImpact":"EVOLUTION_ONLY"
        }))
        .is_err());
        assert!(CapabilityGapHandoffPolicy::from_report(&json!({
            "executionMode":"EVOLUTION_THREAD",
            "deliveryImpact":"EVOLUTION_ONLY",
            "originGapId":"gap_origin"
        }))
        .is_ok());
    }
}
