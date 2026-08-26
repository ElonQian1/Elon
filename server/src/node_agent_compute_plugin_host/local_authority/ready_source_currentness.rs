use std::{
    fmt,
    marker::PhantomData,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    rc::Rc,
    time::Instant,
};

use anyhow::{bail, Context, Result};
use rusqlite::Transaction;

use super::{
    plan_application::{read_authority_plan_application_state, AuthorityPlanApplicationState},
    work_admission_store::read_current_work_admission_head_pair_on,
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    OpenedComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    local_authority_schema::verify_schema_v8_read_only,
    manifest_validation::is_sha256,
    ready_capability::{
        revalidate_ready_publication_at_current_authority, ValidatedComputeReadyPublication,
    },
    ready_source_lineage_projection::project_user_node_ready_source_lineage,
    trusted_time::ComputePluginTrustedTimeObservation,
    user_node_ready_source_lineage_contract::{
        ProjectedComputeUserNodeReadySourceLineageV1,
        UntrustedComputeUserNodeHostRuntimeObservationV1,
    },
    work_admission_contract::{
        ComputePluginWorkAdmissionReceiptPair, DurableWorkAdmittedPluginSlot,
    },
};

const MAX_IJSON_INTEGER: i64 = 9_007_199_254_740_991;

/// A local-currentness proof that exists only while one query-only SQLite snapshot is borrowed.
/// It deliberately does not serialize or clone, and its lifetime cannot escape the HRTB callback.
#[must_use = "the current Ready source seal is valid only inside its authority snapshot"]
struct CurrentComputeUserNodeReadySourceLineageSeal<'snapshot> {
    projected: ProjectedComputeUserNodeReadySourceLineageV1,
    authority_state_revision: i64,
    inventory_revision: i64,
    authority_epoch: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms: i64,
    _snapshot_thread: PhantomData<Rc<&'snapshot ()>>,
}

impl CurrentComputeUserNodeReadySourceLineageSeal<'_> {
    /// The projection remains explicitly untrusted and continues to carry all four serialized
    /// gaps. Only possession of this transaction-scoped wrapper proves local currentness.
    #[allow(dead_code)]
    fn projected_lineage(&self) -> &ProjectedComputeUserNodeReadySourceLineageV1 {
        &self.projected
    }
}

impl fmt::Debug for CurrentComputeUserNodeReadySourceLineageSeal<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CurrentComputeUserNodeReadySourceLineageSeal")
            .field("lineage_digest", &"<redacted>")
            .field("authority_state_revision", &self.authority_state_revision)
            .field("inventory_revision", &self.inventory_revision)
            .field("authority_epoch", &self.authority_epoch)
            .field("process_owner_epoch", &self.process_owner_epoch)
            .field(
                "trusted_time_high_water_ms",
                &self.trusted_time_high_water_ms,
            )
            .finish()
    }
}

impl OpenedComputePluginLocalAuthority {
    /// Reproves the exact work-admission head and Ready inventory inside one handle-bound,
    /// query-only snapshot. This does not mint Ready, runtime, Host, session or dispatch authority.
    /// The callback is a pure in-process derivation seam: it must not perform I/O, invoke a writer,
    /// publish, dispatch, or otherwise create an external effect before the post-callback checks.
    #[allow(dead_code)]
    fn with_current_user_node_ready_source_lineage(
        &mut self,
        process_fence: &ComputePluginFetchProcessFence,
        fresh_time: &ComputePluginTrustedTimeObservation,
        admitted: &DurableWorkAdmittedPluginSlot<'_>,
        ready: &ValidatedComputeReadyPublication,
        host_runtime_observation: UntrustedComputeUserNodeHostRuntimeObservationV1,
        derive: impl for<'snapshot> FnOnce(
            CurrentComputeUserNodeReadySourceLineageSeal<'snapshot>,
        ) -> Result<()>,
    ) -> Result<()> {
        self.ensure_current()?;
        validate_external_custody(self, process_fence, fresh_time, admitted, ready)?;

        let opened_binding = self.authority_instance_binding().clone();
        let opened_installation_id_digest = self.installation_id_digest().to_string();
        let opened_root_identity_digest = self.root_identity_digest().to_string();
        let callback_outcome = self.with_deferred_read(|transaction| {
            let seal = build_current_seal_on(
                transaction,
                &opened_binding,
                &opened_installation_id_digest,
                &opened_root_identity_digest,
                process_fence,
                fresh_time,
                admitted,
                ready,
                host_runtime_observation,
            )?;
            // Retain a panic as data while returning through `with_deferred_read`. When its commit
            // and query-only restoration both succeed, the original panic resumes outside the
            // snapshot. A commit/restore failure remains the owner's higher-priority Store error.
            Ok(catch_unwind(AssertUnwindSafe(|| derive(seal))))
        })?;

        let post_callback_custody: Result<()> = (|| {
            self.ensure_current()?;
            process_fence.ensure_process_owner_current()?;
            fresh_time.ensure_live(Instant::now())?;
            Ok(())
        })();
        match callback_outcome {
            Ok(value) => {
                post_callback_custody?;
                value
            }
            Err(payload) => {
                // The checks above still run, but a panic keeps its original payload and cannot be
                // converted into a success or an authority error after query-only was restored.
                drop(post_callback_custody);
                resume_unwind(payload)
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_current_seal_on<'snapshot>(
    transaction: &'snapshot Transaction<'_>,
    opened_binding: &ComputePluginAuthorityInstanceBinding,
    opened_installation_id_digest: &str,
    opened_root_identity_digest: &str,
    process_fence: &ComputePluginFetchProcessFence,
    fresh_time: &ComputePluginTrustedTimeObservation,
    admitted: &DurableWorkAdmittedPluginSlot<'_>,
    ready: &ValidatedComputeReadyPublication,
    host_runtime_observation: UntrustedComputeUserNodeHostRuntimeObservationV1,
) -> Result<CurrentComputeUserNodeReadySourceLineageSeal<'snapshot>> {
    validate_scalar_custody(
        opened_binding,
        opened_installation_id_digest,
        opened_root_identity_digest,
        process_fence,
        fresh_time,
        admitted,
        ready,
    )?;

    verify_schema_v8_read_only(transaction)?;
    let authority = read_authority_plan_application_state(transaction, fresh_time.trusted_now())?;
    let authority_witness = CurrentAuthorityWitness::read_on(transaction, &authority)?;
    let stored_pair = read_current_work_admission_head_pair_on(transaction, admitted.receipts())?;
    validate_current_authority_successor(
        &authority,
        &authority_witness,
        process_fence,
        fresh_time,
        &stored_pair,
        ready,
    )?;
    revalidate_ready_publication_at_current_authority(ready, &authority.inventory, fresh_time)?;

    let projected =
        project_user_node_ready_source_lineage(admitted, ready, host_runtime_observation)?;
    authority_witness.ensure_unchanged_on(transaction)?;
    process_fence.ensure_process_owner_current()?;
    fresh_time.ensure_live(Instant::now())?;

    Ok(CurrentComputeUserNodeReadySourceLineageSeal {
        projected,
        authority_state_revision: authority.state_revision,
        inventory_revision: authority.inventory.inventory_revision,
        authority_epoch: authority.authority_epoch,
        process_owner_epoch: authority.process_owner_epoch,
        trusted_time_high_water_ms: authority.trusted_time_high_water_ms,
        _snapshot_thread: PhantomData,
    })
}

fn validate_external_custody(
    opened: &OpenedComputePluginLocalAuthority,
    process_fence: &ComputePluginFetchProcessFence,
    fresh_time: &ComputePluginTrustedTimeObservation,
    admitted: &DurableWorkAdmittedPluginSlot<'_>,
    ready: &ValidatedComputeReadyPublication,
) -> Result<()> {
    process_fence.ensure_process_owner_current()?;
    fresh_time.ensure_live(Instant::now())?;
    validate_scalar_custody(
        opened.authority_instance_binding(),
        opened.installation_id_digest(),
        opened.root_identity_digest(),
        process_fence,
        fresh_time,
        admitted,
        ready,
    )
}

fn validate_scalar_custody(
    opened_binding: &ComputePluginAuthorityInstanceBinding,
    opened_installation_id_digest: &str,
    opened_root_identity_digest: &str,
    process_fence: &ComputePluginFetchProcessFence,
    fresh_time: &ComputePluginTrustedTimeObservation,
    admitted: &DurableWorkAdmittedPluginSlot<'_>,
    ready: &ValidatedComputeReadyPublication,
) -> Result<()> {
    let staging_key = admitted.installed().revalidated().staged().recovery_key();
    let receipt = admitted.receipts().receipt().receipt();
    let prior_time = ready.trusted_time();
    let admitted_time = admitted.trusted_time();
    if !opened_binding.matches(process_fence.authority_instance_binding())
        || !opened_binding.matches(staging_key.authority_instance_binding())
        || opened_installation_id_digest != process_fence.installation_id_digest()
        || opened_installation_id_digest != staging_key.installation_id_digest()
        || opened_installation_id_digest != receipt.installation_id_digest()
        || opened_installation_id_digest != fresh_time.installation_id_digest()
        || opened_installation_id_digest != prior_time.installation_id_digest()
        || opened_root_identity_digest != staging_key.root_identity_digest()
        || process_fence.process_owner_epoch() != staging_key.process_owner_epoch()
        || process_fence.process_owner_epoch() != receipt.authority().process_owner_epoch()
        || process_fence.clock_epoch_digest() != staging_key.clock_epoch_digest()
        || process_fence.clock_epoch_digest() != receipt.clock_epoch_digest()
        || process_fence.clock_epoch_digest() != fresh_time.clock_epoch_digest()
        || process_fence.clock_epoch_digest() != prior_time.clock_epoch_digest()
        || process_fence.clock_epoch_digest() != admitted_time.clock_epoch_digest()
        || opened_installation_id_digest != admitted_time.installation_id_digest()
        || fresh_time.observed_at() <= process_fence.acquired_observed_at()
        || fresh_time.observed_at() <= admitted.revalidated_at()
        || fresh_time.observed_at() <= admitted_time.observed_at()
        || fresh_time.trusted_now().timestamp_millis() < process_fence.acquired_at_ms()
        || fresh_time.trusted_now() <= admitted_time.trusted_now()
    {
        bail!("COMPUTE_PLUGIN_READY_CURRENT_CUSTODY_CHANGED");
    }
    Ok(())
}

fn validate_current_authority_successor(
    authority: &AuthorityPlanApplicationState,
    witness: &CurrentAuthorityWitness,
    process_fence: &ComputePluginFetchProcessFence,
    fresh_time: &ComputePluginTrustedTimeObservation,
    stored_pair: &ComputePluginWorkAdmissionReceiptPair,
    ready: &ValidatedComputeReadyPublication,
) -> Result<()> {
    let source = stored_pair.source().source();
    let plan = source.plan();
    let receipt = stored_pair.receipt().receipt();
    let transition = receipt.authority();
    let current_authorization_matches =
        authority
            .sharing_authorization
            .as_ref()
            .is_some_and(|authorization| {
                authorization.authorization_ref == plan.sharing_authorization_ref()
                    && authorization.revision == plan.sharing_authorization_revision()
                    && authorization.digest == plan.sharing_authorization_digest()
            });
    let bounded_positive_facts = [
        authority.state_revision,
        authority.inventory.inventory_revision,
        authority.desired_policy_revision,
        authority.manifest_catalog_revision,
        authority.keyring_bundle_revision,
        authority.publisher_keyring.revision,
        authority.control_keyring.revision,
        authority.authority_epoch,
        authority.process_owner_epoch,
        authority.trusted_time_high_water_ms,
        witness.updated_at_ms,
    ];
    if bounded_positive_facts
        .into_iter()
        .any(|value| !(1..=MAX_IJSON_INTEGER).contains(&value))
        || witness.updated_at_ms != authority.trusted_time_high_water_ms
        || fresh_time.trusted_now().timestamp_millis() <= authority.trusted_time_high_water_ms
        || !is_sha256(&authority.publisher_keyring.digest)
        || !is_sha256(&authority.control_keyring.digest)
        || authority.publisher_keyring == authority.control_keyring
        || authority.host_api_revision == 0
        || authority.installation_id_digest != receipt.installation_id_digest()
        || authority.state_revision <= transition.authority_state_revision_after()
        || authority.inventory.inventory_revision <= transition.inventory_revision_after()
        || authority.authority_epoch < transition.authority_epoch_after()
        || authority.process_owner_epoch != transition.process_owner_epoch()
        || authority.process_owner_epoch != process_fence.process_owner_epoch()
        || authority.trusted_time_high_water_ms < receipt.admitted_at_ms()
        || authority.trusted_time_high_water_ms > fresh_time.trusted_now().timestamp_millis()
        || witness.updated_at_ms < receipt.admitted_at_ms()
        || !authority.sharing_enabled
        || !current_authorization_matches
        || authority.desired_policy_revision != plan.policy_revision()
        || authority.desired_policy_revision != ready.desired_policy_revision()
        || authority.inventory.inventory_revision != ready.inventory_revision()
        || authority.node_profile_digest != plan.node_profile_digest()
        || authority.manifest_catalog_revision != plan.manifest_catalog_revision()
        || authority.keyring_bundle_revision != plan.keyring_bundle_revision()
        || authority.publisher_keyring.revision != plan.publisher_keyring_revision()
        || authority.publisher_keyring.digest != plan.publisher_keyring_digest()
        || authority.control_keyring.revision != plan.control_keyring_revision()
        || authority.control_keyring.digest != plan.control_keyring_digest()
        || authority.target_id != source.launch_profile().target_id()
        || authority.host_api_protocol_id != source.launch_profile().host_api_protocol_id()
        || authority.host_api_revision != source.launch_profile().host_api_revision()
    {
        bail!("COMPUTE_PLUGIN_READY_CURRENT_AUTHORITY_NOT_LEGAL_SUCCESSOR");
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
struct CurrentAuthorityWitness {
    installation_id_digest: String,
    state_revision: i64,
    inventory_revision: i64,
    inventory_digest: String,
    desired_policy_revision: i64,
    sharing_enabled: i64,
    sharing_authorization_ref: Option<String>,
    sharing_authorization_revision: Option<i64>,
    sharing_authorization_digest: Option<String>,
    authority_epoch: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms: i64,
    updated_at_ms: i64,
}

impl CurrentAuthorityWitness {
    fn read_on(
        transaction: &Transaction<'_>,
        authority: &AuthorityPlanApplicationState,
    ) -> Result<Self> {
        let witness = Self::query_on(transaction)?;
        let (authorization_ref, authorization_revision, authorization_digest) = authority
            .sharing_authorization
            .as_ref()
            .map(|authorization| {
                (
                    Some(authorization.authorization_ref.as_str()),
                    Some(authorization.revision),
                    Some(authorization.digest.as_str()),
                )
            })
            .unwrap_or((None, None, None));
        if witness.installation_id_digest != authority.installation_id_digest
            || witness.state_revision != authority.state_revision
            || witness.inventory_revision != authority.inventory.inventory_revision
            || witness.inventory_digest != authority.inventory_digest
            || witness.desired_policy_revision != authority.desired_policy_revision
            || witness.sharing_enabled != if authority.sharing_enabled { 1 } else { 0 }
            || witness.sharing_authorization_ref.as_deref() != authorization_ref
            || witness.sharing_authorization_revision != authorization_revision
            || witness.sharing_authorization_digest.as_deref() != authorization_digest
            || witness.authority_epoch != authority.authority_epoch
            || witness.process_owner_epoch != authority.process_owner_epoch
            || witness.trusted_time_high_water_ms != authority.trusted_time_high_water_ms
        {
            bail!("COMPUTE_PLUGIN_READY_CURRENT_AUTHORITY_WITNESS_CHANGED");
        }
        Ok(witness)
    }

    fn query_on(transaction: &Transaction<'_>) -> Result<Self> {
        transaction
            .query_row(
                r#"SELECT installation_id_digest, state_revision, inventory_revision,
                    inventory_digest, desired_policy_revision, sharing_enabled,
                    sharing_authorization_ref, sharing_authorization_revision,
                    sharing_authorization_digest, authority_epoch, process_owner_epoch,
                    trusted_time_high_water_ms, updated_at_ms
                FROM authority_meta WHERE singleton = 1"#,
                [],
                |row| {
                    Ok(Self {
                        installation_id_digest: row.get(0)?,
                        state_revision: row.get(1)?,
                        inventory_revision: row.get(2)?,
                        inventory_digest: row.get(3)?,
                        desired_policy_revision: row.get(4)?,
                        sharing_enabled: row.get(5)?,
                        sharing_authorization_ref: row.get(6)?,
                        sharing_authorization_revision: row.get(7)?,
                        sharing_authorization_digest: row.get(8)?,
                        authority_epoch: row.get(9)?,
                        process_owner_epoch: row.get(10)?,
                        trusted_time_high_water_ms: row.get(11)?,
                        updated_at_ms: row.get(12)?,
                    })
                },
            )
            .context("COMPUTE_PLUGIN_READY_CURRENT_AUTHORITY_WITNESS_READ")
    }

    fn ensure_unchanged_on(&self, transaction: &Transaction<'_>) -> Result<()> {
        let current = Self::query_on(transaction)?;
        if current != *self {
            bail!("COMPUTE_PLUGIN_READY_CURRENT_AUTHORITY_DRIFTED");
        }
        Ok(())
    }
}
