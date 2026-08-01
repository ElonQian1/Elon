use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};

use crate::store::Store;

use super::{
    lineage_model::SettlementCorrectionLineage,
    model::{
        SettlementCorrectionDetail, SettlementReceipt, CORRECTION_POSTED,
        RECEIPT_KIND_CORRECTION_REPLACEMENT, RECEIPT_KIND_CORRECTION_REVERSAL,
        RECEIPT_KIND_STANDARD,
    },
};

const MAX_LINEAGE_DEPTH: usize = 32;

pub(super) fn resolve(
    store: &Store,
    project_id: &str,
    receipt_id: &str,
) -> Result<SettlementCorrectionLineage> {
    let requested = receipt(store, project_id, receipt_id)?;
    let root = find_root(store, project_id, &requested)?;
    let (effective, posted_corrections, non_posted_corrections) =
        follow_forward(store, project_id, &root)?;
    let requested_position = position(&requested, &root, &effective);
    let effective_has_blocking_dispute =
        store.task_settlement_has_blocking_dispute(project_id, &effective.id)?;
    Ok(SettlementCorrectionLineage {
        schema: "task_economy.settlement_correction_lineage.v1",
        project_id: project_id.trim().to_string(),
        requested_receipt: requested,
        requested_position,
        root_receipt: root,
        effective_receipt: effective,
        depth: posted_corrections.len(),
        posted_corrections,
        non_posted_corrections,
        effective_has_blocking_dispute,
        shadow_only: true,
    })
}

fn find_root(
    store: &Store,
    project_id: &str,
    start: &SettlementReceipt,
) -> Result<SettlementReceipt> {
    let mut current = start.clone();
    let mut visited = HashSet::new();
    let mut depth = 0;
    loop {
        if !visited.insert(current.id.clone()) {
            bail!("影子结算纠正链存在循环凭证关联");
        }
        let Some(correction_id) = current.correction_id.as_deref() else {
            return Ok(current);
        };
        if depth >= MAX_LINEAGE_DEPTH {
            bail!("影子结算纠正链超过最大深度 {MAX_LINEAGE_DEPTH}");
        }
        let correction = correction(store, project_id, correction_id)?;
        let belongs = correction
            .reversal_receipt
            .as_ref()
            .map(|item| item.id.as_str())
            == Some(current.id.as_str())
            || correction
                .replacement_receipt
                .as_ref()
                .map(|item| item.id.as_str())
                == Some(current.id.as_str());
        if !belongs {
            bail!("纠正凭证与纠正记录的双腿关联不一致");
        }
        current = correction.original_receipt;
        depth += 1;
    }
}

fn follow_forward(
    store: &Store,
    project_id: &str,
    root: &SettlementReceipt,
) -> Result<(
    SettlementReceipt,
    Vec<SettlementCorrectionDetail>,
    Vec<SettlementCorrectionDetail>,
)> {
    let mut current = root.clone();
    let mut posted = Vec::new();
    let mut non_posted = Vec::new();
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current.id.clone()) {
            bail!("影子结算纠正链存在循环凭证关联");
        }
        let corrections = store.list_task_settlement_corrections(project_id, &current.id, 100)?;
        let mut posted_here = corrections
            .iter()
            .filter(|item| item.correction.status == CORRECTION_POSTED);
        let next = posted_here.next().cloned();
        if posted_here.next().is_some() {
            bail!("同一影子凭证存在多条已过账纠正，无法确定有效凭证");
        }
        non_posted.extend(
            corrections
                .into_iter()
                .filter(|item| item.correction.status != CORRECTION_POSTED),
        );
        let Some(correction) = next else {
            return Ok((current, posted, non_posted));
        };
        if posted.len() >= MAX_LINEAGE_DEPTH {
            bail!("影子结算纠正链超过最大深度 {MAX_LINEAGE_DEPTH}");
        }
        let replacement = correction
            .replacement_receipt
            .clone()
            .ok_or_else(|| anyhow!("已过账纠正缺少替换凭证"))?;
        if replacement.receipt_kind != RECEIPT_KIND_CORRECTION_REPLACEMENT
            || replacement.correction_id.as_deref() != Some(correction.correction.id.as_str())
        {
            bail!("已过账纠正的替换凭证关联无效");
        }
        posted.push(correction);
        current = replacement;
    }
}

fn receipt(store: &Store, project_id: &str, receipt_id: &str) -> Result<SettlementReceipt> {
    store
        .task_settlement_receipt(project_id, receipt_id)?
        .ok_or_else(|| anyhow!("影子结算凭证不存在"))
}

fn correction(
    store: &Store,
    project_id: &str,
    correction_id: &str,
) -> Result<SettlementCorrectionDetail> {
    store
        .task_settlement_correction_detail(project_id, correction_id)?
        .ok_or_else(|| anyhow!("影子结算纠正流程不存在"))
}

fn position(
    requested: &SettlementReceipt,
    root: &SettlementReceipt,
    effective: &SettlementReceipt,
) -> String {
    match requested.receipt_kind.as_str() {
        RECEIPT_KIND_CORRECTION_REVERSAL => "correction_reversal".to_string(),
        RECEIPT_KIND_CORRECTION_REPLACEMENT if requested.id == effective.id => {
            "effective_replacement".to_string()
        }
        RECEIPT_KIND_CORRECTION_REPLACEMENT => "superseded_replacement".to_string(),
        RECEIPT_KIND_STANDARD if requested.id == root.id && requested.id == effective.id => {
            "effective_standard".to_string()
        }
        RECEIPT_KIND_STANDARD if requested.id == root.id => "superseded_original".to_string(),
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
#[path = "lineage_service_tests.rs"]
mod tests;
