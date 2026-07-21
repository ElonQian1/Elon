//! Reconcile a completed evolution gap with its deferred business origin.

use std::path::Path;

use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use serde_json::{json, Value};

use super::{load_gap, save_gap, CapabilityGapDocument, CapabilityGapStatus};

pub(super) fn has_completed_successor(
    gaps: &[CapabilityGapDocument],
    origin: &CapabilityGapDocument,
) -> bool {
    gaps.iter().any(|candidate| {
        candidate.status == CapabilityGapStatus::Completed
            && candidate.delegation.is_evolution_thread()
            && candidate.delegation.origin_gap_id() == Some(origin.gap_id.as_str())
    })
}

pub(super) fn reconcile_origin(
    root: &Path,
    evolution: &CapabilityGapDocument,
) -> Result<Option<Value>> {
    if evolution.status != CapabilityGapStatus::Completed
        || !evolution.delegation.is_evolution_thread()
    {
        return Ok(None);
    }
    let origin_gap_id = evolution
        .delegation
        .origin_gap_id()
        .ok_or_else(|| anyhow!("已完成的 evolution gap 缺少 originGapId"))?;
    let mut origin = load_gap(root, origin_gap_id)?;
    if !origin.delegation.is_business_thread() {
        bail!("originGapId 不是业务线程 gap")
    }
    if origin.missing_capabilities != evolution.missing_capabilities {
        bail!("evolution gap 与 origin gap 的能力集合不一致")
    }
    if !matches!(
        origin.status,
        CapabilityGapStatus::Deferred | CapabilityGapStatus::Resumed
    ) {
        bail!("origin gap 当前状态不可对账: {:?}", origin.status)
    }
    let reconciled_at = origin
        .reconciled_at
        .clone()
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    origin.status = CapabilityGapStatus::Resumed;
    origin.reconciled_by_gap_id = Some(evolution.gap_id.clone());
    origin.reconciled_at = Some(reconciled_at.clone());
    origin.updated_at = reconciled_at.clone();
    origin.last_error = None;
    save_gap(&origin)?;
    Ok(Some(json!({
        "schema": "elon.capability_gap_reconciliation.v1",
        "originGapId": origin.gap_id,
        "successorGapId": evolution.gap_id,
        "status": origin.status,
        "reconciledAt": reconciled_at,
    })))
}

#[cfg(test)]
mod tests {
    use super::has_completed_successor;
    use crate::node_agent_android_live::capability_gap::{
        handoff::CapabilityGapHandoffPolicy, CapabilityGapDocument, CapabilityGapStatus,
        CapabilityUpgradePolicy,
    };
    use serde_json::json;

    fn gap(
        id: &str,
        status: CapabilityGapStatus,
        delegation: serde_json::Value,
    ) -> CapabilityGapDocument {
        CapabilityGapDocument {
            schema_version: 1,
            gap_id: id.into(),
            task_id: "task".into(),
            fit_run_id: None,
            project_root: ".".into(),
            status,
            missing_capabilities: vec!["PLATFORM_TOOL_DEFECT".into()],
            evidence: vec!["evidence".into()],
            proposed_changes: vec!["change".into()],
            resume_target: "resume".into(),
            policy: CapabilityUpgradePolicy {
                trusted_boundary: "LOCAL_GIT_WORKSPACE".into(),
                automatic_source_upgrade: true,
                automatic_publish: true,
                max_upgrade_rounds: 8,
            },
            delegation: CapabilityGapHandoffPolicy::from_report(&delegation).unwrap(),
            upgrade_rounds: 0,
            attempts: vec![],
            failure_signatures: vec![],
            reconciled_by_gap_id: None,
            reconciled_at: None,
            created_at: "now".into(),
            updated_at: "now".into(),
            last_error: None,
        }
    }

    #[test]
    fn historical_deferred_origin_is_hidden_by_completed_successor() {
        let origin = gap(
            "gap_origin",
            CapabilityGapStatus::Deferred,
            json!({"executionMode":"BUSINESS_THREAD","deliveryImpact":"DELIVERY_BLOCKING"}),
        );
        let successor = gap(
            "gap_successor",
            CapabilityGapStatus::Completed,
            json!({"executionMode":"EVOLUTION_THREAD","deliveryImpact":"EVOLUTION_ONLY","originGapId":"gap_origin"}),
        );
        assert!(has_completed_successor(
            &[origin.clone(), successor],
            &origin
        ));
    }
}
