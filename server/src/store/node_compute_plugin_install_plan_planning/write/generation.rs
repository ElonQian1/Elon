use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use crate::store::{new_id, now, Store};

use super::super::{
    digest::{generation_outcome_json_and_digest, generation_request_json_and_digest},
    readback::{
        read_generation_outcome, read_generation_request, validate_durable_snapshot_readback,
        validate_generation_outcome_readback, validate_generation_request_readback,
        validate_generation_snapshot_authority,
    },
    types::{
        ComputePluginInstallPlanGenerationOutcomeV1, ComputePluginInstallPlanGenerationRequestV1,
        DurableComputePluginInstallPlanGenerationOutcomeV1,
        DurableComputePluginInstallPlanGenerationRequestV1,
        DurableComputePluginInstallPlanPlanningSnapshotV2,
    },
    validation::{validate_generation_outcome, validate_generation_request},
    GENERATION_OUTCOME_SCHEMA_V1, GENERATION_REQUEST_SCHEMA_V1, GENERATION_SIGNER_PROFILE_V2,
    MAX_SAFE_INTEGER,
};

impl Store {
    pub(crate) fn record_compute_plugin_install_plan_signer_unavailable_v1(
        &self,
        request: &DurableComputePluginInstallPlanGenerationRequestV1,
    ) -> Result<DurableComputePluginInstallPlanGenerationOutcomeV1> {
        self.record_compute_plugin_install_plan_generation_outcome_v1(
            request,
            "signer_unavailable",
            "COMPUTE_PLUGIN_INSTALL_PLAN_SIGNER_UNAVAILABLE",
            true,
        )
    }

    pub(crate) fn prepare_compute_plugin_install_plan_generation_request_v1(
        &self,
        snapshot: &DurableComputePluginInstallPlanPlanningSnapshotV2,
    ) -> Result<DurableComputePluginInstallPlanGenerationRequestV1> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_durable_snapshot_readback(&tx, snapshot)?;
        validate_generation_snapshot_authority(&tx, snapshot)?;
        let value = &snapshot.snapshot.snapshot;
        let requested_at_ms = current_unix_millis()?;
        if requested_at_ms < value.captured_at_ms || requested_at_ms >= value.expires_at_ms {
            bail!("算力插件 Planning Snapshot V2 不在 generation 有效时间窗");
        }
        if let Some(existing) = read_generation_request(&tx, &snapshot.snapshot_id)? {
            validate_request_matches_snapshot(&existing.request, snapshot)?;
            tx.commit()?;
            return Ok(existing);
        }
        let request = ComputePluginInstallPlanGenerationRequestV1 {
            schema: GENERATION_REQUEST_SCHEMA_V1.to_string(),
            generation_request_id: new_id("cpgr"),
            snapshot_id: snapshot.snapshot_id.clone(),
            snapshot_digest: snapshot.snapshot.snapshot_digest.clone(),
            node_id: value.node_id.clone(),
            owner_user_id: value.owner_user_id.clone(),
            installation_identity_digest: value.installation_identity_digest.clone(),
            policy_revision: value.policy_revision,
            policy_digest: value.policy_digest.clone(),
            authorization_ref: value.authorization.authorization_ref.clone(),
            authorization_revision: value.authorization.revision,
            authorization_digest: value.authorization.digest.clone(),
            requested_control_keyring_revision: value.control_keyring.revision,
            requested_control_keyring_digest: value.control_keyring.digest.clone(),
            signer_profile: GENERATION_SIGNER_PROFILE_V2.to_string(),
            requested_at_ms,
        };
        validate_generation_request(&request)?;
        let (request_json, request_digest) = generation_request_json_and_digest(&request)?;
        tx.execute(
            "INSERT INTO node_compute_plugin_install_plan_generation_requests_v1 (
               generation_request_id, request_schema, request_json, request_digest,
               snapshot_id, snapshot_digest, node_id, owner_user_id,
               installation_identity_digest, policy_revision, policy_digest,
               authorization_ref, authorization_revision, authorization_digest,
               requested_control_keyring_revision, requested_control_keyring_digest,
                signer_profile, requested_at_ms, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                request.generation_request_id,
                request.schema,
                request_json,
                request_digest,
                request.snapshot_id,
                request.snapshot_digest,
                request.node_id,
                request.owner_user_id,
                request.installation_identity_digest,
                i64::try_from(request.policy_revision)?,
                request.policy_digest,
                request.authorization_ref,
                i64::try_from(request.authorization_revision)?,
                request.authorization_digest,
                i64::try_from(request.requested_control_keyring_revision)?,
                request.requested_control_keyring_digest,
                request.signer_profile,
                i64::try_from(request.requested_at_ms)?,
                now(),
            ],
        )?;
        let durable = DurableComputePluginInstallPlanGenerationRequestV1 {
            request,
            request_json,
            request_digest,
        };
        validate_generation_request_readback(&tx, &durable)?;
        tx.commit()?;
        Ok(durable)
    }

    pub(crate) fn record_compute_plugin_install_plan_generation_outcome_v1(
        &self,
        request: &DurableComputePluginInstallPlanGenerationRequestV1,
        outcome_kind: &str,
        detail_code: &str,
        retryable: bool,
    ) -> Result<DurableComputePluginInstallPlanGenerationOutcomeV1> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_generation_request_readback(&tx, request)?;
        if let Some(existing) =
            read_generation_outcome(&tx, &request.request.generation_request_id)?
        {
            if existing.outcome.outcome_kind != outcome_kind
                || existing.outcome.detail_code != detail_code
                || existing.outcome.retryable != retryable
            {
                bail!("同一 InstallPlan generation request 不能改变终态");
            }
            tx.commit()?;
            return Ok(existing);
        }
        let outcome = ComputePluginInstallPlanGenerationOutcomeV1 {
            schema: GENERATION_OUTCOME_SCHEMA_V1.to_string(),
            outcome_id: new_id("cpgo"),
            generation_request_id: request.request.generation_request_id.clone(),
            generation_request_digest: request.request_digest.clone(),
            outcome_kind: outcome_kind.to_string(),
            detail_code: detail_code.to_string(),
            retryable,
        };
        validate_generation_outcome(&outcome)?;
        let (outcome_json, outcome_digest) = generation_outcome_json_and_digest(&outcome)?;
        tx.execute(
            "INSERT INTO node_compute_plugin_install_plan_generation_outcomes_v1 (
               outcome_id, outcome_schema, outcome_json, outcome_digest,
               generation_request_id, generation_request_digest, outcome_kind,
               detail_code, retryable, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                outcome.outcome_id,
                outcome.schema,
                outcome_json,
                outcome_digest,
                outcome.generation_request_id,
                outcome.generation_request_digest,
                outcome.outcome_kind,
                outcome.detail_code,
                outcome.retryable,
                now(),
            ],
        )?;
        let durable = DurableComputePluginInstallPlanGenerationOutcomeV1 {
            outcome,
            outcome_json,
            outcome_digest,
        };
        validate_generation_outcome_readback(&tx, &durable)?;
        tx.commit()?;
        Ok(durable)
    }
}

fn current_unix_millis() -> Result<u64> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let millis = u64::try_from(millis)?;
    if millis > MAX_SAFE_INTEGER {
        bail!("云端 generation 时钟超出 I-JSON 安全范围");
    }
    Ok(millis)
}

fn validate_request_matches_snapshot(
    request: &ComputePluginInstallPlanGenerationRequestV1,
    durable: &DurableComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<()> {
    let snapshot = &durable.snapshot.snapshot;
    if request.snapshot_id != durable.snapshot_id
        || request.snapshot_digest != durable.snapshot.snapshot_digest
        || request.node_id != snapshot.node_id
        || request.owner_user_id != snapshot.owner_user_id
        || request.installation_identity_digest != snapshot.installation_identity_digest
        || request.policy_revision != snapshot.policy_revision
        || request.policy_digest != snapshot.policy_digest
        || request.authorization_ref != snapshot.authorization.authorization_ref
        || request.authorization_revision != snapshot.authorization.revision
        || request.authorization_digest != snapshot.authorization.digest
        || request.requested_control_keyring_revision != snapshot.control_keyring.revision
        || request.requested_control_keyring_digest != snapshot.control_keyring.digest
        || request.requested_at_ms < snapshot.captured_at_ms
        || request.requested_at_ms >= snapshot.expires_at_ms
    {
        bail!("同一 Planning Snapshot V2 不能改变 generation request");
    }
    Ok(())
}
