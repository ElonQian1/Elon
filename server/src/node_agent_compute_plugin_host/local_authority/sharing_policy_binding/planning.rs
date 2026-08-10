use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    revocation::{read_exact_revocation, validate_terminalized_work},
    types::{
        ComputePluginSharingPolicyBindingReceipt, PreparedSharingPolicyBindingRequest,
        SharingPolicyBindingRequestDigest, COMPUTE_PLUGIN_SHARING_POLICY_BINDING_REQUEST_SCHEMA,
    },
    write::read_exact_receipt,
};
use crate::{
    compute_plugin_sharing_directive::{
        compute_plugin_sharing_policy_snapshot_digest,
        validate_compute_plugin_sharing_policy_snapshot_v1,
    },
    node_agent_compute_plugin_host::{
        local_authority::plan_application::AuthorityPlanApplicationState,
        signed_artifact_verification::jcs_sha256_hex,
    },
};

/// Planning-only proof that the exact enabled durable policy receipt is the current authority
/// head and that all work revoked by that transition remains terminalized.
pub(in crate::node_agent_compute_plugin_host::local_authority) struct PlanningPolicyBinding {
    snapshot: homecli_proto::ComputePluginSharingPolicySnapshotV1,
    binding_receipt_digest: String,
    revocation_receipt_digest: String,
    source_preparation_id: String,
    source_bootstrap_instance_id: String,
    source_configuration_generation: u64,
    source_cancellation_generation: u64,
    authority_epoch: u64,
    process_owner_epoch: u64,
    trusted_time_before_ms: u64,
    bound_at_ms: u64,
}

impl PlanningPolicyBinding {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn snapshot(
        &self,
    ) -> &homecli_proto::ComputePluginSharingPolicySnapshotV1 {
        &self.snapshot
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn binding_receipt_digest(
        &self,
    ) -> &str {
        &self.binding_receipt_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn revocation_receipt_digest(
        &self,
    ) -> &str {
        &self.revocation_receipt_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn source_preparation_id(
        &self,
    ) -> &str {
        &self.source_preparation_id
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn source_bootstrap_instance_id(
        &self,
    ) -> &str {
        &self.source_bootstrap_instance_id
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn source_configuration_generation(
        &self,
    ) -> u64 {
        self.source_configuration_generation
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn source_cancellation_generation(
        &self,
    ) -> u64 {
        self.source_cancellation_generation
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn authority_epoch(
        &self,
    ) -> u64 {
        self.authority_epoch
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn process_owner_epoch(
        &self,
    ) -> u64 {
        self.process_owner_epoch
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn trusted_time_before_ms(
        &self,
    ) -> u64 {
        self.trusted_time_before_ms
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn bound_at_ms(&self) -> u64 {
        self.bound_at_ms
    }
}

pub(in crate::node_agent_compute_plugin_host::local_authority) fn read_planning_policy_binding_on(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
) -> Result<PlanningPolicyBinding> {
    let (request, snapshot) = read_enabled_request(transaction, authority)?;
    let binding = read_exact_receipt(transaction, &request)?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_POLICY_BINDING_UNAVAILABLE"))?;
    let receipt = binding.receipt();
    if receipt.source_preparation_id.as_deref() != request.source_preparation_id.as_deref()
        || receipt.source_bootstrap_instance_id != request.source_bootstrap_instance_id
        || receipt.source_configuration_generation != request.source_configuration_generation
        || receipt.source_cancellation_generation != request.source_cancellation_generation
    {
        bail!("COMPUTE_PLUGIN_PLANNING_POLICY_BINDING_SOURCE_CHANGED");
    }
    validate_current_enabled_head(transaction, authority, &request)?;
    let revocation = read_exact_revocation(transaction, &request, &binding)?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_POLICY_REVOCATION_UNAVAILABLE"))?;
    validate_terminalized_work(transaction, &revocation)?;
    if authority.state_revision < receipt.state_revision_after
        || authority.authority_epoch < receipt.authority_epoch_after
        || authority.process_owner_epoch < receipt.process_owner_epoch
        || authority.trusted_time_high_water_ms < receipt.bound_at_ms
    {
        bail!("COMPUTE_PLUGIN_PLANNING_POLICY_BINDING_FENCE_CHANGED");
    }
    let source_preparation_id = request
        .source_preparation_id
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_POLICY_PREPARATION_UNAVAILABLE"))?;

    Ok(PlanningPolicyBinding {
        snapshot,
        binding_receipt_digest: binding.receipt_digest().to_string(),
        revocation_receipt_digest: revocation.hashed_receipt.receipt_digest().to_string(),
        source_preparation_id,
        source_bootstrap_instance_id: receipt.source_bootstrap_instance_id.clone(),
        source_configuration_generation: receipt.source_configuration_generation,
        source_cancellation_generation: receipt.source_cancellation_generation,
        authority_epoch: u64::try_from(receipt.authority_epoch_after)
            .context("COMPUTE_PLUGIN_PLANNING_POLICY_AUTHORITY_EPOCH_RANGE")?,
        process_owner_epoch: u64::try_from(receipt.process_owner_epoch)
            .context("COMPUTE_PLUGIN_PLANNING_POLICY_PROCESS_OWNER_EPOCH_RANGE")?,
        trusted_time_before_ms: u64::try_from(receipt.trusted_time_before_ms)
            .context("COMPUTE_PLUGIN_PLANNING_POLICY_TRUSTED_TIME_RANGE")?,
        bound_at_ms: u64::try_from(receipt.bound_at_ms)
            .context("COMPUTE_PLUGIN_PLANNING_POLICY_BOUND_TIME_RANGE")?,
    })
}

fn read_enabled_request(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
) -> Result<(
    PreparedSharingPolicyBindingRequest,
    homecli_proto::ComputePluginSharingPolicySnapshotV1,
)> {
    let (policy_snapshot_json, policy_snapshot_digest, receipt_json): (String, String, String) =
        transaction
            .query_row(
                r#"SELECT policy_snapshot_json, policy_snapshot_digest, receipt_json
                   FROM sharing_policy_binding_receipts WHERE policy_revision = ?1"#,
                [authority.desired_policy_revision],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .context("COMPUTE_PLUGIN_PLANNING_POLICY_SOURCE_READ")?
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_POLICY_SOURCE_UNAVAILABLE"))?;
    let snapshot: homecli_proto::ComputePluginSharingPolicySnapshotV1 =
        serde_json::from_str(&policy_snapshot_json)
            .context("COMPUTE_PLUGIN_PLANNING_POLICY_SNAPSHOT_JSON")?;
    let receipt: ComputePluginSharingPolicyBindingReceipt = serde_json::from_str(&receipt_json)
        .context("COMPUTE_PLUGIN_PLANNING_POLICY_RECEIPT_JSON")?;
    validate_compute_plugin_sharing_policy_snapshot_v1(&snapshot)
        .map_err(|error| anyhow::anyhow!(error.code()))?;
    let snapshot_digest = compute_plugin_sharing_policy_snapshot_digest(&snapshot)
        .map_err(|error| anyhow::anyhow!(error.code()))?;
    let authorization = snapshot.authorization.as_ref().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_POLICY_AUTHORIZATION_UNAVAILABLE")
    })?;
    let current_authorization = authority.sharing_authorization.as_ref().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_POLICY_CURRENT_AUTHORIZATION_UNAVAILABLE")
    })?;
    if !snapshot.plugin_runtime_requested
        || !authority.sharing_enabled
        || receipt
            .source_preparation_id
            .as_deref()
            .is_none_or(str::is_empty)
        || snapshot_digest != policy_snapshot_digest
        || snapshot.installation_identity_digest != authority.installation_id_digest
        || i64::try_from(snapshot.policy_revision).ok() != Some(authority.desired_policy_revision)
        || authorization.revision != snapshot.policy_revision
        || authorization.digest != snapshot.policy_digest
        || current_authorization.authorization_ref != authorization.authorization_ref
        || u64::try_from(current_authorization.revision).ok() != Some(authorization.revision)
        || current_authorization.digest != authorization.digest
        || receipt.source_bootstrap_instance_id.is_empty()
        || receipt.source_configuration_generation == 0
        || receipt.source_cancellation_generation == 0
    {
        bail!("COMPUTE_PLUGIN_PLANNING_POLICY_SOURCE_CHANGED");
    }
    if serde_json::to_string(&snapshot)? != policy_snapshot_json
        || policy_snapshot_json.len() > 65_536
    {
        bail!("COMPUTE_PLUGIN_PLANNING_POLICY_SNAPSHOT_LIMIT");
    }
    let request_digest = jcs_sha256_hex(&SharingPolicyBindingRequestDigest {
        schema: COMPUTE_PLUGIN_SHARING_POLICY_BINDING_REQUEST_SCHEMA,
        policy_snapshot: &snapshot,
        policy_snapshot_digest: &policy_snapshot_digest,
    })?;
    let request = PreparedSharingPolicyBindingRequest {
        node_id: snapshot.node_id.clone(),
        owner_user_id: snapshot.owner_user_id.clone(),
        installation_id_digest: snapshot.installation_identity_digest.clone(),
        policy_revision: i64::try_from(snapshot.policy_revision)
            .context("COMPUTE_PLUGIN_PLANNING_POLICY_REVISION_RANGE")?,
        policy_digest: snapshot.policy_digest.clone(),
        policy_snapshot_json,
        policy_snapshot_digest,
        sharing_enabled: true,
        sharing_authorization_ref: Some(authorization.authorization_ref.clone()),
        sharing_authorization_revision: Some(
            i64::try_from(authorization.revision)
                .context("COMPUTE_PLUGIN_PLANNING_POLICY_AUTHORIZATION_REVISION_RANGE")?,
        ),
        sharing_authorization_digest: Some(authorization.digest.clone()),
        source_preparation_id: receipt.source_preparation_id,
        source_bootstrap_instance_id: receipt.source_bootstrap_instance_id,
        source_configuration_generation: receipt.source_configuration_generation,
        source_cancellation_generation: receipt.source_cancellation_generation,
        request_digest,
    };
    Ok((request, snapshot))
}

fn validate_current_enabled_head(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    request: &PreparedSharingPolicyBindingRequest,
) -> Result<()> {
    let matches = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM authority_meta WHERE singleton = 1
              AND installation_id_digest = ?1
              AND desired_policy_revision = ?2 AND sharing_enabled = 1
              AND sharing_authorization_ref = ?3
              AND sharing_authorization_revision = ?4
              AND sharing_authorization_digest = ?5
              AND state_revision = ?6
              AND inventory_revision = ?7 AND inventory_digest = ?8 AND inventory_json = ?9
              AND node_profile_digest = ?10 AND manifest_catalog_revision = ?11
              AND target_id = ?12 AND host_api_protocol_id = ?13 AND host_api_revision = ?14
              AND authority_epoch = ?15 AND process_owner_epoch = ?16
              AND active_bundle_revision = ?17
              AND publisher_keyring_revision = ?18 AND publisher_keyring_digest = ?19
              AND control_keyring_revision = ?20 AND control_keyring_digest = ?21
              AND clock_status = 'trusted'
              AND trusted_time_high_water_ms = ?22
              AND updated_at_ms = trusted_time_high_water_ms"#,
            params![
                &request.installation_id_digest,
                request.policy_revision,
                request.sharing_authorization_ref.as_deref(),
                request.sharing_authorization_revision,
                request.sharing_authorization_digest.as_deref(),
                authority.state_revision,
                authority.inventory.inventory_revision,
                &authority.inventory_digest,
                &authority.inventory_json,
                &authority.node_profile_digest,
                authority.manifest_catalog_revision,
                &authority.target_id,
                &authority.host_api_protocol_id,
                authority.host_api_revision,
                authority.authority_epoch,
                authority.process_owner_epoch,
                authority.keyring_bundle_revision,
                authority.publisher_keyring.revision,
                &authority.publisher_keyring.digest,
                authority.control_keyring.revision,
                &authority.control_keyring.digest,
                authority.trusted_time_high_water_ms,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_PLANNING_POLICY_CURRENT_HEAD_READ")?;
    if matches != 1 {
        bail!("COMPUTE_PLUGIN_PLANNING_POLICY_BINDING_NOT_CURRENT");
    }
    Ok(())
}
