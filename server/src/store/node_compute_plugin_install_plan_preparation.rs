//! Durable cloud ledger for inert InstallPlan context preparation.

use anyhow::{bail, Result};
use rusqlite::{params, Transaction, TransactionBehavior};
use serde::Serialize;

use super::{
    clean_optional, new_id,
    node_compute_plugin_sharing::{
        NodeComputePluginSharingAuthorization, NodeComputePluginSharingDispatchIntent,
    },
    now, Store,
};

mod digest;
mod idempotency;
mod observation;
mod validation;

use digest::{context_json_and_digest, observed_json_and_digest, preparation_request_digest};
use idempotency::sharing_ack_already_consumed;
use observation::validate_inert_observation;
use validation::{
    accepted_sharing_ack_matches, current_sharing_head_matches, delivery_binding_is_committed,
    observation_binding_is_current, resolve_existing_preparation_id, validate_sharing_binding,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NodeComputePluginInstallPlanPreparationDispatchIntent {
    pub(crate) delivery_id: String,
    pub(crate) sharing_delivery_id: String,
    pub(crate) preparation_id: String,
    pub(crate) request_digest: String,
    pub(crate) consent_receipt_id: String,
    pub(crate) node_id: String,
    pub(crate) owner_user_id: String,
    pub(crate) installation_identity_digest: String,
    pub(crate) policy_revision: i64,
    pub(crate) policy_digest: String,
    pub(crate) policy_snapshot_digest: String,
    pub(crate) authorization: NodeComputePluginSharingAuthorization,
    pub(crate) replayed: bool,
}

impl Store {
    /// Creates or reuses the immutable request bound to the current sharing head, then appends one
    /// fresh delivery. A historical ACK never suppresses reconstruction in a new node process.
    pub(crate) fn prepare_node_compute_plugin_install_plan_preparation_delivery(
        &self,
        sharing: &NodeComputePluginSharingDispatchIntent,
        policy_snapshot_digest: &str,
    ) -> Result<Option<NodeComputePluginInstallPlanPreparationDispatchIntent>> {
        if !sharing.dispatchable || !sharing.plugin_runtime_requested {
            return Ok(None);
        }
        let authorization = sharing
            .authorization
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("算力插件 InstallPlan 准备请求缺少共享授权"))?;
        validate_sharing_binding(sharing, authorization, policy_snapshot_digest)?;

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !current_sharing_head_matches(&tx, sharing, authorization)? {
            tx.commit()?;
            return Ok(None);
        }
        if !accepted_sharing_ack_matches(&tx, sharing, authorization, policy_snapshot_digest)? {
            tx.commit()?;
            return Ok(None);
        }

        let existing =
            resolve_existing_preparation_id(&tx, sharing, authorization, policy_snapshot_digest)?;
        if sharing_ack_already_consumed(&tx, sharing, existing.as_deref())? {
            tx.commit()?;
            return Ok(None);
        }
        let replayed = existing.is_some();
        let preparation_id = existing.unwrap_or_else(|| new_id("cpip"));
        let request_digest = preparation_request_digest(
            &preparation_id,
            sharing,
            authorization,
            policy_snapshot_digest,
        );
        if !replayed {
            tx.execute(
                "INSERT INTO node_compute_plugin_install_plan_preparation_requests (
                   preparation_id, request_schema, request_digest, node_id, owner_user_id,
                   consent_receipt_id, installation_identity_digest, policy_revision,
                   policy_digest, policy_snapshot_digest, authorization_ref,
                   authorization_revision, authorization_digest, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    preparation_id,
                    homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_REQUEST_V1_SCHEMA,
                    request_digest.as_str(),
                    sharing.node_id,
                    sharing.owner_user_id,
                    sharing.consent_receipt_id,
                    sharing.installation_identity_digest,
                    sharing.policy_revision,
                    sharing.policy_digest,
                    policy_snapshot_digest,
                    authorization.authorization_ref,
                    authorization.revision,
                    authorization.digest,
                    now()
                ],
            )?;
        }

        let intent = NodeComputePluginInstallPlanPreparationDispatchIntent {
            delivery_id: new_id("cpid"),
            sharing_delivery_id: sharing.delivery_id.clone(),
            preparation_id,
            request_digest,
            consent_receipt_id: sharing.consent_receipt_id.clone(),
            node_id: sharing.node_id.clone(),
            owner_user_id: sharing.owner_user_id.clone(),
            installation_identity_digest: sharing.installation_identity_digest.clone(),
            policy_revision: sharing.policy_revision,
            policy_digest: sharing.policy_digest.clone(),
            policy_snapshot_digest: policy_snapshot_digest.to_string(),
            authorization: authorization.clone(),
            replayed,
        };
        insert_delivery_intent(&tx, &intent)?;
        tx.commit()?;
        Ok(Some(intent))
    }

    pub(crate) fn record_node_compute_plugin_install_plan_preparation_delivery(
        &self,
        intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
        event_kind: &str,
        detail_code: Option<&str>,
    ) -> Result<()> {
        if !matches!(
            event_kind,
            "dispatched"
                | "capability_missing"
                | "agent_offline"
                | "writer_closed"
                | "ack_timeout"
                | "dispatch_failed"
        ) {
            bail!("未知算力插件 InstallPlan 准备下发结果");
        }
        let detail_code = clean_optional(detail_code);
        if (event_kind == "dispatched") != detail_code.is_none() {
            bail!("算力插件 InstallPlan 准备下发结果与详情码不一致");
        }
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !delivery_binding_is_committed(&tx, intent)? {
            bail!("算力插件 InstallPlan 准备下发绑定不存在或已损坏");
        }
        let sequence = tx.query_row(
            "SELECT COALESCE(MAX(event_sequence), 0) + 1
               FROM node_compute_plugin_install_plan_preparation_delivery_events
              WHERE delivery_id=?1",
            params![intent.delivery_id],
            |row| row.get::<_, i64>(0),
        )?;
        if sequence != 2 {
            bail!("算力插件 InstallPlan 准备下发只能写入唯一终态");
        }
        tx.execute(
            "INSERT INTO node_compute_plugin_install_plan_preparation_delivery_events (
               id, delivery_id, sharing_delivery_id, preparation_id, node_id, consent_receipt_id,
               policy_revision, policy_digest, event_sequence, event_kind,
               detail_code, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                new_id("cpie"),
                intent.delivery_id,
                intent.sharing_delivery_id,
                intent.preparation_id,
                intent.node_id,
                intent.consent_receipt_id,
                intent.policy_revision,
                intent.policy_digest,
                sequence,
                event_kind,
                detail_code,
                now()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_node_compute_plugin_install_plan_preparation_observation(
        &self,
        intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
        accepted: bool,
        replayed: bool,
        context_ready: bool,
        context: Option<&serde_json::Value>,
        bootstrap_instance_id: &str,
        observed: &serde_json::Value,
    ) -> Result<()> {
        validate_inert_observation(
            intent,
            accepted,
            replayed,
            context_ready,
            context,
            bootstrap_instance_id,
            observed,
        )?;
        if context_ready != context.is_some() {
            bail!("算力插件 InstallPlan 准备上下文状态不一致");
        }
        let bootstrap_instance_id = bootstrap_instance_id.trim();
        if bootstrap_instance_id.is_empty()
            || bootstrap_instance_id.len() > 256
            || bootstrap_instance_id.chars().any(char::is_control)
        {
            bail!("算力插件 Bootstrap 实例编号无效");
        }
        let (context_json, context_digest) = match context {
            Some(value) => {
                let (json, digest) = context_json_and_digest(value)?;
                (Some(json), Some(digest))
            }
            None => (None, None),
        };
        let (observed_json, observed_digest) = observed_json_and_digest(observed)?;

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !observation_binding_is_current(&tx, intent)? {
            bail!("算力插件 InstallPlan 准备观察不再绑定当前共享策略");
        }
        tx.execute(
            "INSERT INTO node_compute_plugin_install_plan_preparation_observations (
               id, delivery_id, preparation_id, node_id, consent_receipt_id,
               policy_revision, policy_digest, policy_snapshot_digest, accepted,
               replayed, context_ready, context_json, context_digest,
               bootstrap_instance_id, observed_json, observed_digest, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                new_id("cpio"),
                intent.delivery_id,
                intent.preparation_id,
                intent.node_id,
                intent.consent_receipt_id,
                intent.policy_revision,
                intent.policy_digest,
                intent.policy_snapshot_digest,
                accepted,
                replayed,
                context_ready,
                context_json,
                context_digest,
                bootstrap_instance_id,
                observed_json,
                observed_digest,
                now()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn insert_delivery_intent(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginInstallPlanPreparationDispatchIntent,
) -> Result<()> {
    tx.execute(
        "INSERT INTO node_compute_plugin_install_plan_preparation_delivery_events (
           id, delivery_id, sharing_delivery_id, preparation_id, node_id, consent_receipt_id,
           policy_revision, policy_digest, event_sequence, event_kind, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, 'intent_committed', ?9)",
        params![
            new_id("cpie"),
            intent.delivery_id,
            intent.sharing_delivery_id,
            intent.preparation_id,
            intent.node_id,
            intent.consent_receipt_id,
            intent.policy_revision,
            intent.policy_digest,
            now()
        ],
    )?;
    Ok(())
}
