use std::time::Instant;

use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    current::validate_current_admission,
    head::{read_head, WorkAdmissionHead},
    readback::read_pair,
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
    work_admission_contract::{
        ComputePluginWorkAdmissionReceiptPair, ComputePluginWorkAdmissionRecoveryKey,
        ComputePluginWorkAdmissionRecoveryOutcome,
    },
};

mod chain;

use chain::count_chain_membership;

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginWorkAdmissionRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_time: ComputePluginTrustedTimeObservation,
}

struct CurrentAuthorityState {
    installation_id_digest: String,
    state_revision: i64,
    inventory_revision: i64,
    inventory_digest: String,
    authority_epoch: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms: i64,
    updated_at_ms: i64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AbsentAuthorityDisposition {
    ExactBefore,
    Superseded,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CommittedAuthorityDisposition {
    ExactAfter,
    Superseded,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_work_admission_recovery_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginWorkAdmissionRecoveryAuthoritySession<'authority>> {
        observation.ensure_live(Instant::now())?;
        process_fence.ensure_process_owner_current()?;
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observation.observed_at() <= process_fence.acquired_observed_at()
            || observation.trusted_now().timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_SESSION_INVALID");
        }
        Ok(ComputePluginWorkAdmissionRecoveryAuthoritySession {
            authority: self,
            process_fence,
            trusted_time: observation,
        })
    }
}

impl ComputePluginWorkAdmissionRecoveryAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        self.trusted_time.clock_epoch_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn was_observed_strictly_after(
        &self,
        barrier: Instant,
    ) -> bool {
        self.trusted_time.observed_at() > barrier
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_work_admission_outcome(
        &self,
        key: &ComputePluginWorkAdmissionRecoveryKey,
    ) -> Result<ComputePluginWorkAdmissionRecoveryOutcome> {
        self.trusted_time.ensure_live(Instant::now())?;
        self.process_fence.ensure_process_owner_current()?;
        validate_provenance(self, key)?;
        let outcome = self
            .authority
            .with_deferred(|transaction| read_outcome(transaction, &self.trusted_time, key))?;
        self.trusted_time.ensure_live(Instant::now())?;
        self.process_fence.ensure_process_owner_current()?;
        Ok(outcome)
    }
}

fn validate_provenance(
    session: &ComputePluginWorkAdmissionRecoveryAuthoritySession<'_>,
    key: &ComputePluginWorkAdmissionRecoveryKey,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.expectation().process_owner_epoch() != session.process_fence.process_owner_epoch()
        || session.trusted_time.observed_at() <= session.process_fence.acquired_observed_at()
        || session.trusted_time.trusted_now().timestamp_millis()
            < key.expectation().admitted_at_ms()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn read_outcome(
    transaction: &Transaction<'_>,
    observation: &ComputePluginTrustedTimeObservation,
    key: &ComputePluginWorkAdmissionRecoveryKey,
) -> Result<ComputePluginWorkAdmissionRecoveryOutcome> {
    let pair = read_pair(
        transaction,
        key.work_admission_id(),
        key.expectation().expected_receipt_digest(),
    )?;
    let identity_matches = count_identity_matches(transaction, key)?;
    let head = read_head(transaction, key.plugin_id())?;
    let authority = read_current_authority(transaction)?;
    match pair {
        Some(pair) => {
            if identity_matches != 1 {
                bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_IDENTITY_AMBIGUOUS");
            }
            validate_expected_pair(key, &pair)?;
            let authority = validate_committed_authority(key, &authority)?;
            classify_committed(transaction, observation, key, pair, head, authority)
        }
        None => {
            if identity_matches != 0 {
                bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_ABSENCE_AMBIGUOUS");
            }
            let disposition = classify_absent_authority(key, &authority)?;
            classify_absent(transaction, key, head, disposition)
        }
    }
}

fn read_current_authority(transaction: &Transaction<'_>) -> Result<CurrentAuthorityState> {
    transaction
        .query_row(
            r#"SELECT installation_id_digest, state_revision, inventory_revision,
                inventory_digest, authority_epoch, process_owner_epoch,
                trusted_time_high_water_ms, updated_at_ms
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok(CurrentAuthorityState {
                    installation_id_digest: row.get(0)?,
                    state_revision: row.get(1)?,
                    inventory_revision: row.get(2)?,
                    inventory_digest: row.get(3)?,
                    authority_epoch: row.get(4)?,
                    process_owner_epoch: row.get(5)?,
                    trusted_time_high_water_ms: row.get(6)?,
                    updated_at_ms: row.get(7)?,
                })
            },
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_AUTHORITY_READ")
}

fn validate_committed_authority(
    key: &ComputePluginWorkAdmissionRecoveryKey,
    current: &CurrentAuthorityState,
) -> Result<CommittedAuthorityDisposition> {
    let expected = key.expectation();
    if current.installation_id_digest != key.installation_id_digest()
        || current.process_owner_epoch != expected.process_owner_epoch()
        || current.updated_at_ms != current.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_AUTHORITY_CHANGED");
    }
    let exact_after = current.state_revision == expected.authority_state_revision_after()
        && current.inventory_revision == expected.inventory_revision_after()
        && current.inventory_digest == expected.inventory_digest_after()
        && current.authority_epoch == expected.authority_epoch_after()
        && current.trusted_time_high_water_ms == expected.admitted_at_ms()
        && current.updated_at_ms == expected.admitted_at_ms();
    if exact_after {
        return Ok(CommittedAuthorityDisposition::ExactAfter);
    }
    let legal_successor = current.state_revision >= expected.authority_state_revision_after()
        && current.inventory_revision >= expected.inventory_revision_after()
        && (current.inventory_revision > expected.inventory_revision_after()
            || current.inventory_digest == expected.inventory_digest_after())
        && current.authority_epoch >= expected.authority_epoch_after()
        && current.trusted_time_high_water_ms >= expected.admitted_at_ms()
        && current.updated_at_ms >= expected.admitted_at_ms()
        && (current.state_revision > expected.authority_state_revision_after()
            || current.inventory_revision > expected.inventory_revision_after()
            || current.authority_epoch > expected.authority_epoch_after()
            || current.trusted_time_high_water_ms > expected.admitted_at_ms()
            || current.updated_at_ms > expected.admitted_at_ms());
    if !legal_successor {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_AUTHORITY_FORKED");
    }
    Ok(CommittedAuthorityDisposition::Superseded)
}

fn classify_absent_authority(
    key: &ComputePluginWorkAdmissionRecoveryKey,
    current: &CurrentAuthorityState,
) -> Result<AbsentAuthorityDisposition> {
    let expected = key.expectation();
    if current.installation_id_digest != key.installation_id_digest()
        || current.process_owner_epoch != expected.process_owner_epoch()
        || current.updated_at_ms != current.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_AUTHORITY_IDENTITY_CHANGED");
    }
    let exact_before = current.state_revision == expected.authority_state_revision_before()
        && current.inventory_revision == expected.inventory_revision_before()
        && current.inventory_digest == expected.inventory_digest_before()
        && current.authority_epoch == expected.authority_epoch_before()
        && current.trusted_time_high_water_ms == expected.trusted_time_high_water_ms_before()
        && current.updated_at_ms == expected.authority_updated_at_ms_before();
    if exact_before {
        return Ok(AbsentAuthorityDisposition::ExactBefore);
    }
    let legal_successor = current.state_revision >= expected.authority_state_revision_before()
        && current.inventory_revision >= expected.inventory_revision_before()
        && (current.inventory_revision > expected.inventory_revision_before()
            || current.inventory_digest == expected.inventory_digest_before())
        && current.authority_epoch >= expected.authority_epoch_before()
        && current.trusted_time_high_water_ms >= expected.trusted_time_high_water_ms_before()
        && current.updated_at_ms >= expected.authority_updated_at_ms_before()
        && (current.state_revision > expected.authority_state_revision_before()
            || current.inventory_revision > expected.inventory_revision_before()
            || current.authority_epoch > expected.authority_epoch_before()
            || current.trusted_time_high_water_ms > expected.trusted_time_high_water_ms_before()
            || current.updated_at_ms > expected.authority_updated_at_ms_before());
    if !legal_successor {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_AUTHORITY_FORKED");
    }
    Ok(AbsentAuthorityDisposition::Superseded)
}

fn count_identity_matches(
    transaction: &Transaction<'_>,
    key: &ComputePluginWorkAdmissionRecoveryKey,
) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM compute_plugin_work_admission_receipts
            WHERE work_admission_id = ?1 OR source_digest = ?2 OR receipt_digest = ?3"#,
            params![
                key.work_admission_id(),
                key.expectation().source_digest(),
                key.expectation().expected_receipt_digest(),
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_IDENTITY_READ")
}

fn classify_committed(
    transaction: &Transaction<'_>,
    observation: &ComputePluginTrustedTimeObservation,
    key: &ComputePluginWorkAdmissionRecoveryKey,
    pair: ComputePluginWorkAdmissionReceiptPair,
    head: Option<WorkAdmissionHead>,
    authority: CommittedAuthorityDisposition,
) -> Result<ComputePluginWorkAdmissionRecoveryOutcome> {
    let head = head.ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_COMMITTED_HEAD_MISSING")
    })?;
    if head.installation_id_digest != key.installation_id_digest()
        || head.plugin_id != key.plugin_id()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_HEAD_CHANGED");
    }
    if head.work_admission_id == key.work_admission_id()
        && head.receipt_digest == key.expectation().expected_receipt_digest()
        && head.generation == key.expectation().work_admission_generation_after()
    {
        if authority == CommittedAuthorityDisposition::ExactAfter {
            validate_current_admission(transaction, observation, &pair)?;
            return Ok(ComputePluginWorkAdmissionRecoveryOutcome::AdmittedCurrent(
                pair,
            ));
        }
        return Ok(ComputePluginWorkAdmissionRecoveryOutcome::CommittedHistorical(pair));
    }
    if head.generation <= key.expectation().work_admission_generation_after()
        || count_chain_membership(
            transaction,
            key.installation_id_digest(),
            key.plugin_id(),
            &head,
            key.work_admission_id(),
            key.expectation().expected_receipt_digest(),
            key.expectation().work_admission_generation_after(),
        )? != 1
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_HISTORY_CHANGED");
    }
    Ok(ComputePluginWorkAdmissionRecoveryOutcome::CommittedHistorical(pair))
}

fn classify_absent(
    transaction: &Transaction<'_>,
    key: &ComputePluginWorkAdmissionRecoveryKey,
    head: Option<WorkAdmissionHead>,
    authority: AbsentAuthorityDisposition,
) -> Result<ComputePluginWorkAdmissionRecoveryOutcome> {
    let expected = key.expectation();
    if authority == AbsentAuthorityDisposition::Superseded {
        if head.as_ref().is_some_and(|head| {
            head.installation_id_digest != key.installation_id_digest()
                || head.plugin_id != key.plugin_id()
                || head.generation < expected.work_admission_generation_before()
        }) {
            bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_HEAD_FORKED");
        }
        if expected.work_admission_generation_before() > 0 {
            let head = head.as_ref().ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_HEAD_DISAPPEARED")
            })?;
            let previous_id = expected.previous_work_admission_id().ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_PREVIOUS_ID_MISSING")
            })?;
            let previous_digest = expected
                .previous_work_admission_receipt_digest()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_PREVIOUS_DIGEST_MISSING"
                    )
                })?;
            let previous_is_head = head.generation == expected.work_admission_generation_before()
                && head.work_admission_id == previous_id
                && head.receipt_digest == previous_digest;
            if !previous_is_head
                && (head.generation <= expected.work_admission_generation_before()
                    || count_chain_membership(
                        transaction,
                        key.installation_id_digest(),
                        key.plugin_id(),
                        head,
                        previous_id,
                        previous_digest,
                        expected.work_admission_generation_before(),
                    )? != 1)
            {
                bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_PREVIOUS_CHAIN_CHANGED");
            }
        }
        return Ok(ComputePluginWorkAdmissionRecoveryOutcome::NotCreatedSuperseded);
    }
    match head {
        None if expected.work_admission_generation_before() == 0 => {
            Ok(ComputePluginWorkAdmissionRecoveryOutcome::NotCreated)
        }
        None => bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_PREVIOUS_HEAD_MISSING"),
        Some(head)
            if head.installation_id_digest == key.installation_id_digest()
                && head.plugin_id == key.plugin_id()
                && head.generation == expected.work_admission_generation_before()
                && Some(head.work_admission_id.as_str())
                    == expected.previous_work_admission_id()
                && Some(head.receipt_digest.as_str())
                    == expected.previous_work_admission_receipt_digest() =>
        {
            Ok(ComputePluginWorkAdmissionRecoveryOutcome::NotCreated)
        }
        Some(_) => bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_HEAD_AMBIGUOUS"),
    }
}

fn validate_expected_pair(
    key: &ComputePluginWorkAdmissionRecoveryKey,
    pair: &ComputePluginWorkAdmissionReceiptPair,
) -> Result<()> {
    pair.validate()?;
    let expected = key.expectation();
    let source = pair.source().source();
    let receipt = pair.receipt().receipt();
    let generations = receipt.generations();
    let authority = receipt.authority();
    if pair.source().source_digest() != expected.source_digest()
        || pair.receipt().receipt_digest() != expected.expected_receipt_digest()
        || receipt.work_admission_id() != key.work_admission_id()
        || receipt.installation_id_digest() != key.installation_id_digest()
        || receipt.clock_epoch_digest() != key.clock_epoch_digest()
        || receipt.plugin_id() != key.plugin_id()
        || receipt.slot_ref() != key.slot_ref()
        || receipt.release() != key.release()
        || source.installation_id_digest() != key.installation_id_digest()
        || source.plugin_id() != key.plugin_id()
        || source.slot_ref() != key.slot_ref()
        || source.release() != key.release()
        || receipt.install_receipt_digest() != expected.install_receipt_digest()
        || receipt.promotion_receipt_digest() != expected.promotion_receipt_digest()
        || generations.install_generation() != expected.install_generation()
        || generations.activation_generation() != expected.activation_generation()
        || generations.runtime_generation() != expected.runtime_generation()
        || generations.work_admission_generation_before()
            != expected.work_admission_generation_before()
        || generations.work_admission_generation_after()
            != expected.work_admission_generation_after()
        || generations.previous_work_admission_id() != expected.previous_work_admission_id()
        || generations.previous_work_admission_receipt_digest()
            != expected.previous_work_admission_receipt_digest()
        || authority.authority_state_revision_before() != expected.authority_state_revision_before()
        || authority.authority_state_revision_after() != expected.authority_state_revision_after()
        || authority.inventory_revision_before() != expected.inventory_revision_before()
        || authority.inventory_revision_after() != expected.inventory_revision_after()
        || authority.inventory_digest_before() != expected.inventory_digest_before()
        || authority.inventory_digest_after() != expected.inventory_digest_after()
        || authority.authority_epoch_before() != expected.authority_epoch_before()
        || authority.authority_epoch_after() != expected.authority_epoch_after()
        || authority.process_owner_epoch() != expected.process_owner_epoch()
        || authority.trusted_time_high_water_ms_before()
            != expected.trusted_time_high_water_ms_before()
        || authority.authority_updated_at_ms_before() != expected.authority_updated_at_ms_before()
        || receipt.admitted_at_ms() != expected.admitted_at_ms()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_RECEIPT_CHANGED");
    }
    Ok(())
}
