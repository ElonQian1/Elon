use anyhow::{bail, Context, Error, Result};
use rusqlite::Transaction;

use super::{
    custody::ComputePluginPlanningSnapshotReadCustody,
    meta::{ensure_planning_authority_unchanged_on, read_planning_authority_on},
    projection::{
        ComputePluginPlanningAuthorityProjectionBlocked,
        ComputePluginPlanningAuthorityProjectionFields,
        ComputePluginPlanningAuthorityProjectionOutcome,
        PreparedComputePluginPlanningAuthorityProjection,
    },
    records::project_planning_records_on,
    rollback::project_and_match_rollback_checkpoint,
};
use crate::{
    compute_plugin_sharing_directive::compute_plugin_sharing_policy_snapshot_digest,
    node_agent_compute_plugin_host::local_authority::{
        manifest_catalog_binding::read_planning_catalog_binding_on,
        sharing_policy_binding::read_planning_policy_binding_on, OpenedComputePluginLocalAuthority,
    },
};

const CUSTODY_BLOCKED: &str = "COMPUTE_PLUGIN_PLANNING_AUTHORITY_CUSTODY_BLOCKED";
const META_BLOCKED: &str = "COMPUTE_PLUGIN_PLANNING_AUTHORITY_META_BLOCKED";
const POLICY_BLOCKED: &str = "COMPUTE_PLUGIN_PLANNING_AUTHORITY_POLICY_BLOCKED";
const CATALOG_BLOCKED: &str = "COMPUTE_PLUGIN_PLANNING_AUTHORITY_CATALOG_BLOCKED";
const RECORDS_BLOCKED: &str = "COMPUTE_PLUGIN_PLANNING_AUTHORITY_RECORDS_BLOCKED";
const ROLLBACK_BLOCKED: &str = "COMPUTE_PLUGIN_PLANNING_AUTHORITY_ROLLBACK_BLOCKED";
const SEAL_BLOCKED: &str = "COMPUTE_PLUGIN_PLANNING_AUTHORITY_SEAL_BLOCKED";
const TRANSACTION_BLOCKED: &str = "COMPUTE_PLUGIN_PLANNING_AUTHORITY_TRANSACTION_BLOCKED";

struct ProjectionFailure {
    code: &'static str,
    error: Error,
}

impl ProjectionFailure {
    fn at<T>(code: &'static str, result: Result<T>) -> std::result::Result<T, Self> {
        result.map_err(|error| Self { code, error })
    }

    fn blocked(self) -> ComputePluginPlanningAuthorityProjectionBlocked {
        ComputePluginPlanningAuthorityProjectionBlocked::from_error(self.code, &self.error)
    }
}

impl OpenedComputePluginLocalAuthority {
    /// Consumes one unconstructable A1 custody and either seals a complete authority projection or
    /// returns a redacted blocker. This never creates a wire snapshot or any execution capability.
    pub(in crate::node_agent_compute_plugin_host) fn project_install_plan_planning_authority<'a>(
        &'a mut self,
        custody: ComputePluginPlanningSnapshotReadCustody<'a>,
    ) -> ComputePluginPlanningAuthorityProjectionOutcome<'a> {
        if let Err(error) = custody.ensure_for_opened(self) {
            return blocked(CUSTODY_BLOCKED, &error);
        }
        let opened_installation_id_digest = self.installation_id_digest().to_string();
        let projected = self.with_deferred_read(|transaction| {
            Ok(project_planning_authority_on(
                transaction,
                &opened_installation_id_digest,
                &custody,
            ))
        });
        if let Err(error) = custody.ensure_for_opened(self) {
            return blocked(CUSTODY_BLOCKED, &error);
        }
        match projected {
            Ok(Ok(projection)) => ComputePluginPlanningAuthorityProjectionOutcome::Projected(
                projection.bind_custody(custody, self),
            ),
            Ok(Err(failure)) => {
                ComputePluginPlanningAuthorityProjectionOutcome::Blocked(failure.blocked())
            }
            Err(error) => blocked(TRANSACTION_BLOCKED, &error),
        }
    }
}

fn project_planning_authority_on(
    transaction: &Transaction<'_>,
    opened_installation_id_digest: &str,
    custody: &ComputePluginPlanningSnapshotReadCustody<'_>,
) -> std::result::Result<PreparedComputePluginPlanningAuthorityProjection, ProjectionFailure> {
    ProjectionFailure::at(CUSTODY_BLOCKED, custody.ensure_external_current())?;
    let trusted_now = custody.trusted_time().trusted_now().clone();
    let authority = ProjectionFailure::at(
        META_BLOCKED,
        read_planning_authority_on(transaction, &trusted_now),
    )?;
    ProjectionFailure::at(
        CUSTODY_BLOCKED,
        validate_authority_custody(opened_installation_id_digest, &authority, custody),
    )?;
    let policy = ProjectionFailure::at(
        POLICY_BLOCKED,
        read_planning_policy_binding_on(transaction, &authority.state),
    )?;
    ProjectionFailure::at(POLICY_BLOCKED, validate_policy_custody(&policy, custody))?;
    let catalog = ProjectionFailure::at(
        CATALOG_BLOCKED,
        read_planning_catalog_binding_on(
            transaction,
            trusted_now,
            &authority.state,
            custody.bootstrap_roots(),
        ),
    )?;
    let records = ProjectionFailure::at(
        RECORDS_BLOCKED,
        project_planning_records_on(
            transaction,
            custody.trusted_time(),
            &authority.state,
            &catalog,
        ),
    )?;
    let rollback = ProjectionFailure::at(
        ROLLBACK_BLOCKED,
        project_and_match_rollback_checkpoint(&authority, &catalog, custody),
    )?;
    ProjectionFailure::at(
        META_BLOCKED,
        ensure_planning_authority_unchanged_on(
            transaction,
            custody.trusted_time().trusted_now(),
            &authority,
        ),
    )?;
    ProjectionFailure::at(CUSTODY_BLOCKED, custody.ensure_external_current())?;

    let snapshot = policy.snapshot();
    let authorization = snapshot
        .authorization
        .clone()
        .ok_or_else(|| ProjectionFailure {
            code: POLICY_BLOCKED,
            error: anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_POLICY_AUTHORIZATION_UNAVAILABLE"),
        })?;
    let policy_snapshot_digest = ProjectionFailure::at(
        POLICY_BLOCKED,
        compute_plugin_sharing_policy_snapshot_digest(snapshot)
            .map_err(|error| anyhow::anyhow!(error.code())),
    )?;
    ProjectionFailure::at(
        SEAL_BLOCKED,
        PreparedComputePluginPlanningAuthorityProjection::seal(
            ComputePluginPlanningAuthorityProjectionFields {
                installation_id_digest: authority.state.installation_id_digest.clone(),
                bootstrap_instance_id: custody.bootstrap_instance_id().to_string(),
                configuration_generation: custody.configuration_generation(),
                cancellation_generation: custody.cancellation_generation(),
                planning_request_digest: custody.planning_request_digest().to_string(),
                account_binding_digest: custody.account_binding_digest().to_string(),
                bootstrap_root_set_digest: custody.bootstrap_root_set_digest().to_string(),
                authority_schema_version: to_u64(
                    authority.schema_version,
                    "AUTHORITY_SCHEMA_VERSION",
                )?,
                state_revision: to_u64(authority.state.state_revision, "STATE_REVISION")?,
                authority_epoch: to_u64(authority.state.authority_epoch, "AUTHORITY_EPOCH")?,
                process_owner_epoch: to_u64(
                    authority.state.process_owner_epoch,
                    "PROCESS_OWNER_EPOCH",
                )?,
                clock_epoch_digest: custody.process_fence().clock_epoch_digest().to_string(),
                trusted_time_high_water_ms: to_u64(
                    authority.state.trusted_time_high_water_ms,
                    "TRUSTED_TIME_HIGH_WATER",
                )?,
                captured_at_ms: authority.captured_at_ms,
                inventory_revision: to_u64(
                    authority.state.inventory.inventory_revision,
                    "INVENTORY_REVISION",
                )?,
                inventory_digest: authority.state.inventory_digest.clone(),
                inventory: authority.state.inventory.clone(),
                node_id: snapshot.node_id.clone(),
                owner_user_id: snapshot.owner_user_id.clone(),
                sharing_enabled: authority.state.sharing_enabled,
                authorization: Some(authorization),
                policy_revision: snapshot.policy_revision,
                policy_digest: snapshot.policy_digest.clone(),
                policy_snapshot_digest,
                policy_binding_receipt_digest: policy.binding_receipt_digest().to_string(),
                policy_revocation_receipt_digest: policy.revocation_receipt_digest().to_string(),
                policy_source_preparation_id: Some(policy.source_preparation_id().to_string()),
                policy_source_bootstrap_instance_id: policy
                    .source_bootstrap_instance_id()
                    .to_string(),
                policy_source_configuration_generation: policy.source_configuration_generation(),
                policy_source_cancellation_generation: policy.source_cancellation_generation(),
                policy_binding_authority_epoch: policy.authority_epoch(),
                policy_binding_process_owner_epoch: policy.process_owner_epoch(),
                policy_trusted_time_before_ms: policy.trusted_time_before_ms(),
                policy_bound_at_ms: policy.bound_at_ms(),
                node_profile_digest: catalog.node_profile_digest().to_string(),
                manifest_catalog_revision: catalog.catalog_revision(),
                manifest_catalog_digest: catalog.catalog_digest().to_string(),
                manifest_catalog_binding_receipt_digest: catalog
                    .binding_receipt_digest()
                    .to_string(),
                keyring_bundle_revision: catalog.keyring_bundle_revision(),
                publisher_keyring: catalog.publisher_keyring().clone(),
                control_keyring: catalog.control_keyring().clone(),
                target_id: catalog.target_id().to_string(),
                host_api_protocol_id: catalog.host_api_protocol_id().to_string(),
                host_api_revision: catalog.host_api_revision(),
                rollback_anchor_id: custody.rollback_permit().anchor_id().to_string(),
                rollback_anchor_sequence: to_u64(
                    custody.rollback_permit().anchor_sequence(),
                    "ROLLBACK_ANCHOR_SEQUENCE",
                )?,
                rollback_checkpoint_digest: rollback.checkpoint_digest,
                rollback_attestation_digest: custody
                    .rollback_permit()
                    .attestation_digest()
                    .to_string(),
                rollback_signing_key_fingerprint: custody
                    .rollback_permit()
                    .signing_key_fingerprint()
                    .to_string(),
                rollback_witness_digest: custody.rollback_permit().witness_digest().to_string(),
                installed_records: records,
            },
        ),
    )
}

fn validate_authority_custody(
    opened_installation_id_digest: &str,
    authority: &super::meta::PlanningAuthorityRead,
    custody: &ComputePluginPlanningSnapshotReadCustody<'_>,
) -> Result<()> {
    if opened_installation_id_digest != authority.state.installation_id_digest
        || custody.process_fence().installation_id_digest()
            != authority.state.installation_id_digest
        || custody.process_fence().process_owner_epoch() != authority.state.process_owner_epoch
        || custody.node_profile_digest() != authority.state.node_profile_digest
        || custody.target_id() != authority.state.target_id
        || custody.host_api_protocol_id() != authority.state.host_api_protocol_id
        || custody.host_api_revision() != authority.state.host_api_revision
        || custody.process_fence().acquired_at_ms() > authority.state.trusted_time_high_water_ms
        || custody.process_fence().acquired_observed_at() > std::time::Instant::now()
        || custody.trusted_time().observed_at() > std::time::Instant::now()
    {
        bail!("COMPUTE_PLUGIN_PLANNING_AUTHORITY_CUSTODY_CHANGED");
    }
    Ok(())
}

fn validate_policy_custody(
    policy: &super::super::sharing_policy_binding::PlanningPolicyBinding,
    custody: &ComputePluginPlanningSnapshotReadCustody<'_>,
) -> Result<()> {
    let snapshot = policy.snapshot();
    if policy.source_preparation_id() != custody.source_preparation_id()
        || policy.source_bootstrap_instance_id() != custody.bootstrap_instance_id()
        || policy.source_configuration_generation() != custody.configuration_generation()
        || policy.source_cancellation_generation() != custody.cancellation_generation()
        || snapshot.node_id != custody.node_id()
        || snapshot.owner_user_id != custody.owner_user_id()
    {
        bail!("COMPUTE_PLUGIN_PLANNING_POLICY_CUSTODY_CHANGED");
    }
    Ok(())
}

fn to_u64(value: i64, field: &'static str) -> std::result::Result<u64, ProjectionFailure> {
    u64::try_from(value).map_err(|error| ProjectionFailure {
        code: SEAL_BLOCKED,
        error: Error::new(error).context(format!("COMPUTE_PLUGIN_PLANNING_{field}_RANGE")),
    })
}

fn blocked<'a>(
    code: &'static str,
    error: &Error,
) -> ComputePluginPlanningAuthorityProjectionOutcome<'a> {
    ComputePluginPlanningAuthorityProjectionOutcome::Blocked(
        ComputePluginPlanningAuthorityProjectionBlocked::from_error(code, error),
    )
}
