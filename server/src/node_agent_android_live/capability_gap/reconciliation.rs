//! Reconcile a completed evolution gap with its deferred business origin.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};

use super::{load_gap, save_gap, CapabilityGapDocument, CapabilityGapStatus};

pub(super) fn origin_project_root(arguments: &Value) -> Result<Option<PathBuf>> {
    let Some(value) = arguments
        .get("originProjectRoot")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    if value.chars().count() > 4_000 {
        bail!("originProjectRoot 超过 4000 字符");
    }
    let root = PathBuf::from(value)
        .canonicalize()
        .context("originProjectRoot 不存在")?;
    if !root.join(".git").exists() {
        bail!("originProjectRoot 不是 Git 工作区");
    }
    Ok(Some(root))
}

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
    let recorded_root = Path::new(&origin.project_root)
        .canonicalize()
        .map_err(|error| anyhow!("origin gap 项目根不可访问: {error}"))?;
    let requested_root = root
        .canonicalize()
        .map_err(|error| anyhow!("originProjectRoot 不可访问: {error}"))?;
    if recorded_root != requested_root {
        bail!("originProjectRoot 与 origin gap 的可信项目根不一致")
    }
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
    use super::{has_completed_successor, reconcile_origin};
    use crate::node_agent_android_live::capability_gap::{
        handoff::CapabilityGapHandoffPolicy, save_gap, CapabilityGapDocument, CapabilityGapStatus,
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

    #[test]
    fn completed_successor_reconciles_origin_in_a_different_worktree() {
        let origin_root =
            std::env::temp_dir().join(format!("elon-gap-origin-{}", uuid::Uuid::new_v4().simple()));
        let evolution_root = std::env::temp_dir().join(format!(
            "elon-gap-evolution-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&origin_root).unwrap();
        std::fs::create_dir_all(&evolution_root).unwrap();
        let mut origin = gap(
            "gap_origin",
            CapabilityGapStatus::Deferred,
            json!({"executionMode":"BUSINESS_THREAD","deliveryImpact":"DELIVERY_BLOCKING"}),
        );
        origin.project_root = origin_root.canonicalize().unwrap().display().to_string();
        save_gap(&origin).unwrap();
        let mut successor = gap(
            "gap_successor",
            CapabilityGapStatus::Completed,
            json!({"executionMode":"EVOLUTION_THREAD","deliveryImpact":"EVOLUTION_ONLY","originGapId":"gap_origin"}),
        );
        successor.project_root = evolution_root.canonicalize().unwrap().display().to_string();

        let result = reconcile_origin(&origin_root, &successor).unwrap().unwrap();
        assert_eq!(result["status"], "RESUMED");
        let reconciled = super::super::load_gap(&origin_root, "gap_origin").unwrap();
        assert_eq!(reconciled.status, CapabilityGapStatus::Resumed);
        assert_eq!(
            reconciled.reconciled_by_gap_id.as_deref(),
            Some("gap_successor")
        );
        let _ = std::fs::remove_dir_all(origin_root);
        let _ = std::fs::remove_dir_all(evolution_root);
    }
}
