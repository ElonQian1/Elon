use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use rusqlite::Transaction;

use super::{
    current::{validate_current_admission, validate_current_installed_receipts},
    head::{read_head, WorkAdmissionHead},
    readback::read_pair_required,
};
use crate::node_agent_compute_plugin_host::{
    lifecycle::ComputePluginLocalRecord,
    local_authority::plan_application::AuthorityPlanApplicationState,
    trusted_time::ComputePluginTrustedTimeObservation,
    work_admission_contract::ComputePluginWorkAdmissionReceiptPair,
};

/// Projects a head only while it remains exact-current against the coherent authority read.
/// Historical heads are valid local audit evidence but are intentionally omitted from planning.
pub(in crate::node_agent_compute_plugin_host::local_authority) fn read_planning_work_admission_on(
    transaction: &Transaction<'_>,
    observation: &ComputePluginTrustedTimeObservation,
    authority: &AuthorityPlanApplicationState,
    record: &ComputePluginLocalRecord,
) -> Result<Option<homecli_proto::ComputePluginInstallPlanPlanningWorkAdmissionV2>> {
    let receipt_count = count_receipts_on(
        transaction,
        &authority.installation_id_digest,
        &record.plugin_id,
    )?;
    let Some(head) = read_head(transaction, &record.plugin_id)? else {
        if receipt_count != 0 {
            bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_HEAD_MISSING");
        }
        return Ok(None);
    };
    if head.installation_id_digest != authority.installation_id_digest
        || head.plugin_id != record.plugin_id
        || head.generation <= 0
        || receipt_count != head.generation
    {
        bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_HEAD_INVALID");
    }

    let pair = read_pair_required(transaction, &head.work_admission_id, &head.receipt_digest)?;
    validate_head_pair(&head, &pair)?;
    validate_predecessor_chain(transaction, &head, &pair)?;

    let receipt = pair.receipt().receipt();
    let transition = receipt.authority();
    let authority_is_exact = authority.state_revision
        == transition.authority_state_revision_after()
        && authority.inventory.inventory_revision == transition.inventory_revision_after()
        && authority.inventory_digest == transition.inventory_digest_after()
        && authority.authority_epoch == transition.authority_epoch_after()
        && authority.process_owner_epoch == transition.process_owner_epoch()
        && authority.trusted_time_high_water_ms == receipt.admitted_at_ms();
    if authority_is_exact {
        if observation.installation_id_digest() != authority.installation_id_digest
            || observation.clock_epoch_digest() != receipt.clock_epoch_digest()
        {
            bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_OBSERVATION_CHANGED");
        }
        validate_current_admission(transaction, observation, &pair)?;
        return Ok(Some(
            homecli_proto::ComputePluginInstallPlanPlanningWorkAdmissionV2 {
                generation: u64::try_from(head.generation)
                    .context("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_GENERATION_RANGE")?,
                receipt_digest: head.receipt_digest,
            },
        ));
    }

    validate_legal_successor(authority, receipt)?;
    Ok(None)
}

/// Rebuilds the exact current work-admission head without requiring the authority to remain at the
/// original stopped/no-health state. A Ready-currentness reader must separately prove that the
/// later authority state is the legal exact successor it expects. Receipt counting and predecessor
/// replay are intentionally complete and therefore O(history); this source seam is not a hot-path
/// publisher until a separately bounded/cached owner contract exists.
pub(in crate::node_agent_compute_plugin_host::local_authority) fn read_current_work_admission_head_pair_on(
    transaction: &Transaction<'_>,
    expected: &ComputePluginWorkAdmissionReceiptPair,
) -> Result<ComputePluginWorkAdmissionReceiptPair> {
    let expected_receipt = expected.receipt().receipt();
    let receipt_count = count_receipts_on(
        transaction,
        expected_receipt.installation_id_digest(),
        expected_receipt.plugin_id(),
    )?;
    let head = read_head(transaction, expected_receipt.plugin_id())?.ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_READY_CURRENT_WORK_ADMISSION_HEAD_MISSING")
    })?;
    if head.installation_id_digest != expected_receipt.installation_id_digest()
        || head.plugin_id != expected_receipt.plugin_id()
        || head.generation <= 0
        || receipt_count != head.generation
        || head.work_admission_id != expected_receipt.work_admission_id()
        || head.receipt_digest != expected.receipt().receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_READY_CURRENT_WORK_ADMISSION_HEAD_CHANGED");
    }

    let stored = read_pair_required(transaction, &head.work_admission_id, &head.receipt_digest)?;
    validate_head_pair(&head, &stored)?;
    validate_predecessor_chain(transaction, &head, &stored)?;
    if &stored != expected {
        bail!("COMPUTE_PLUGIN_READY_CURRENT_WORK_ADMISSION_OWNER_CHANGED");
    }
    validate_current_installed_receipts(transaction, &stored)?;
    Ok(stored)
}

fn count_receipts_on(
    transaction: &Transaction<'_>,
    installation_id_digest: &str,
    plugin_id: &str,
) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT COUNT(*)
               FROM compute_plugin_work_admission_receipts
               WHERE installation_id_digest = ?1 AND plugin_id = ?2"#,
            (installation_id_digest, plugin_id),
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_RECEIPT_COUNT")
}

fn validate_head_pair(
    head: &WorkAdmissionHead,
    pair: &ComputePluginWorkAdmissionReceiptPair,
) -> Result<()> {
    let receipt = pair.receipt().receipt();
    let generations = receipt.generations();
    if receipt.installation_id_digest() != head.installation_id_digest
        || receipt.plugin_id() != head.plugin_id
        || receipt.work_admission_id() != head.work_admission_id
        || pair.receipt().receipt_digest() != head.receipt_digest
        || generations.work_admission_generation_after() != head.generation
        || generations.previous_work_admission_id() != head.previous_id.as_deref()
        || generations.previous_work_admission_receipt_digest()
            != head.previous_receipt_digest.as_deref()
        || receipt.admitted_at_ms() != head.updated_at_ms
    {
        bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_HEAD_CHANGED");
    }
    Ok(())
}

fn validate_predecessor_chain(
    transaction: &Transaction<'_>,
    head: &WorkAdmissionHead,
    head_pair: &ComputePluginWorkAdmissionReceiptPair,
) -> Result<()> {
    let head_receipt = head_pair.receipt().receipt();
    let mut next_id = head_receipt
        .generations()
        .previous_work_admission_id()
        .map(str::to_string);
    let mut next_digest = head_receipt
        .generations()
        .previous_work_admission_receipt_digest()
        .map(str::to_string);
    let mut expected_generation = head.generation - 1;
    let mut successor_admitted_at_ms = head_receipt.admitted_at_ms();
    let mut seen = HashSet::from([(head.work_admission_id.clone(), head.receipt_digest.clone())]);

    loop {
        let (work_admission_id, receipt_digest) = match (next_id.take(), next_digest.take()) {
            (Some(work_admission_id), Some(receipt_digest)) => (work_admission_id, receipt_digest),
            (None, None) => break,
            _ => bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_CHAIN_TRUNCATED"),
        };
        if expected_generation <= 0
            || !seen.insert((work_admission_id.clone(), receipt_digest.clone()))
        {
            bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_CHAIN_CYCLE");
        }
        let pair = read_pair_required(transaction, &work_admission_id, &receipt_digest)?;
        let receipt = pair.receipt().receipt();
        let generations = receipt.generations();
        if receipt.installation_id_digest() != head.installation_id_digest
            || receipt.plugin_id() != head.plugin_id
            || receipt.work_admission_id() != work_admission_id
            || pair.receipt().receipt_digest() != receipt_digest
            || generations.work_admission_generation_after() != expected_generation
            || receipt.admitted_at_ms() >= successor_admitted_at_ms
        {
            bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_CHAIN_CHANGED");
        }
        next_id = generations.previous_work_admission_id().map(str::to_string);
        next_digest = generations
            .previous_work_admission_receipt_digest()
            .map(str::to_string);
        successor_admitted_at_ms = receipt.admitted_at_ms();
        expected_generation -= 1;
    }
    if expected_generation != 0 {
        bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_CHAIN_TRUNCATED");
    }
    Ok(())
}

fn validate_legal_successor(
    authority: &AuthorityPlanApplicationState,
    receipt: &crate::node_agent_compute_plugin_host::work_admission_contract::ComputePluginWorkAdmissionReceipt,
) -> Result<()> {
    let transition = receipt.authority();
    let inventory_is_monotonic = authority.inventory.inventory_revision
        >= transition.inventory_revision_after()
        && (authority.inventory.inventory_revision > transition.inventory_revision_after()
            || authority.inventory_digest == transition.inventory_digest_after());
    let advanced = authority.state_revision > transition.authority_state_revision_after()
        || authority.inventory.inventory_revision > transition.inventory_revision_after()
        || authority.authority_epoch > transition.authority_epoch_after()
        || authority.process_owner_epoch > transition.process_owner_epoch()
        || authority.trusted_time_high_water_ms > receipt.admitted_at_ms();
    if authority.installation_id_digest != receipt.installation_id_digest()
        || authority.state_revision < transition.authority_state_revision_after()
        || !inventory_is_monotonic
        || authority.authority_epoch < transition.authority_epoch_after()
        || authority.process_owner_epoch < transition.process_owner_epoch()
        || authority.trusted_time_high_water_ms < receipt.admitted_at_ms()
        || !advanced
    {
        bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_AUTHORITY_FORKED");
    }
    Ok(())
}
