use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    node_compute_plugin_sharing::{
        NodeComputePluginSharingAuthorization, NodeComputePluginSharingDispatchIntent,
        NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
    },
    node_compute_sharing::UpdateNodeComputeSharingPolicy,
};

pub(super) struct StoredConsent {
    pub(super) intent: NodeComputePluginSharingDispatchIntent,
    pub(super) request_facts_digest: String,
}

pub(super) fn select_consent_by_request(
    tx: &Transaction<'_>,
    node_id: &str,
    request_id: &str,
) -> Result<Option<StoredConsent>> {
    tx.query_row(
        "SELECT c.receipt_id, c.owner_user_id, c.installation_identity_digest,
                c.policy_revision, c.policy_digest, c.consent_schema, c.plugin_runtime_requested,
                c.authorization_ref, c.authorization_revision, c.authorization_digest,
                c.request_facts_digest,
                (SELECT d.delivery_id FROM node_compute_plugin_sharing_delivery_events d
                  WHERE d.consent_receipt_id=c.receipt_id
                    AND d.node_id=c.node_id
                    AND d.policy_revision=c.policy_revision
                    AND d.policy_digest=c.policy_digest
                    AND d.event_sequence=1 AND d.event_kind='intent_committed'
                  ORDER BY d.created_at, d.id LIMIT 1)
           FROM node_compute_plugin_sharing_consents c
          WHERE c.node_id=?1 AND c.consent_request_id=?2",
        params![node_id, request_id],
        |row| stored_consent_from_row(row, node_id),
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn select_current_intent(
    tx: &Transaction<'_>,
    node_id: &str,
) -> Result<Option<NodeComputePluginSharingDispatchIntent>> {
    let intent = tx
        .query_row(
            "SELECT p.plugin_consent_receipt_id, p.owner_user_id,
                p.plugin_installation_identity_digest, p.plugin_policy_revision,
                p.plugin_policy_digest, p.plugin_consent_schema, p.plugin_runtime_requested,
                p.plugin_authorization_ref, p.plugin_authorization_revision,
                p.plugin_authorization_digest
           FROM node_compute_sharing_policies p
           JOIN node_compute_plugin_sharing_consents c
             ON c.receipt_id=p.plugin_consent_receipt_id
            AND c.node_id=p.node_id
            AND c.owner_user_id=p.owner_user_id
            AND c.installation_identity_digest=p.plugin_installation_identity_digest
            AND c.policy_revision=p.plugin_policy_revision
            AND c.policy_digest=p.plugin_policy_digest
            AND c.consent_schema=p.plugin_consent_schema
            AND c.plugin_runtime_requested=p.plugin_runtime_requested
            AND c.plugin_runtime_requested=p.enabled
            AND c.allowed_model_ids_json=p.allowed_model_ids_json
            AND c.max_concurrent_runs=p.max_concurrent_runs
            AND c.daily_token_limit=p.daily_token_limit
            AND c.authorization_ref IS p.plugin_authorization_ref
            AND c.authorization_revision IS p.plugin_authorization_revision
            AND c.authorization_digest IS p.plugin_authorization_digest
          WHERE p.node_id=?1 AND p.plugin_policy_revision > 0",
            params![node_id],
            |row| {
                let schema: String = row.get(5)?;
                let plugin_runtime_requested = row.get::<_, i64>(6)? != 0;
                let authorization = authorization_from_row(row, 7, 8, 9)?;
                let intent = NodeComputePluginSharingDispatchIntent {
                    delivery_id: String::new(),
                    consent_receipt_id: row.get(0)?,
                    node_id: node_id.to_string(),
                    owner_user_id: row.get(1)?,
                    installation_identity_digest: row.get(2)?,
                    policy_revision: row.get(3)?,
                    policy_digest: row.get(4)?,
                    plugin_runtime_requested,
                    authorization,
                    replayed: true,
                    dispatchable: true,
                };
                validate_stored_intent(&intent, &schema)?;
                Ok(intent)
            },
        )
        .optional()?;
    if intent.is_none() {
        let has_plugin_head = tx
            .query_row(
                "SELECT 1 FROM node_compute_sharing_policies
              WHERE node_id=?1 AND plugin_policy_revision > 0",
                params![node_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if has_plugin_head {
            bail!("算力插件当前策略与不可变同意回执不一致");
        }
    }
    Ok(intent)
}

fn stored_consent_from_row(
    row: &rusqlite::Row<'_>,
    node_id: &str,
) -> rusqlite::Result<StoredConsent> {
    let schema: String = row.get(5)?;
    let plugin_runtime_requested = row.get::<_, i64>(6)? != 0;
    let authorization = authorization_from_row(row, 7, 8, 9)?;
    let intent = NodeComputePluginSharingDispatchIntent {
        delivery_id: row.get(11)?,
        consent_receipt_id: row.get(0)?,
        node_id: node_id.to_string(),
        owner_user_id: row.get(1)?,
        installation_identity_digest: row.get(2)?,
        policy_revision: row.get(3)?,
        policy_digest: row.get(4)?,
        plugin_runtime_requested,
        authorization,
        replayed: true,
        dispatchable: true,
    };
    validate_stored_intent(&intent, &schema)?;
    Ok(StoredConsent {
        intent,
        request_facts_digest: row.get(10)?,
    })
}

fn validate_stored_intent(
    intent: &NodeComputePluginSharingDispatchIntent,
    schema: &str,
) -> rusqlite::Result<()> {
    let authorization_valid = intent.plugin_runtime_requested == intent.authorization.is_some()
        && intent.authorization.as_ref().is_none_or(|authorization| {
            authorization.revision == intent.policy_revision
                && authorization.digest == intent.policy_digest
        });
    if schema != NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1
        || intent.policy_revision <= 0
        || intent.policy_digest.len() != 64
        || intent.installation_identity_digest.len() != 64
        || !authorization_valid
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

fn authorization_from_row(
    row: &rusqlite::Row<'_>,
    reference: usize,
    revision: usize,
    digest: usize,
) -> rusqlite::Result<Option<NodeComputePluginSharingAuthorization>> {
    match (row.get(reference)?, row.get(revision)?, row.get(digest)?) {
        (Some(authorization_ref), Some(revision), Some(digest)) => {
            Ok(Some(NodeComputePluginSharingAuthorization {
                authorization_ref,
                revision,
                digest,
            }))
        }
        (None, None, None) => Ok(None),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

pub(super) fn upsert_policy(
    tx: &Transaction<'_>,
    node_id: &str,
    owner: &str,
    update: &UpdateNodeComputeSharingPolicy,
    allowed_json: &str,
    intent: Option<&NodeComputePluginSharingDispatchIntent>,
    ts: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO node_compute_sharing_policies (
           node_id, owner_user_id, enabled, allowed_model_ids_json,
           max_concurrent_runs, daily_token_limit, plugin_runtime_requested,
           plugin_policy_revision, plugin_policy_digest, plugin_consent_schema,
           plugin_consent_receipt_id, plugin_installation_identity_digest,
           plugin_authorization_ref, plugin_authorization_revision,
           plugin_authorization_digest, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
         ON CONFLICT(node_id) DO UPDATE SET
           owner_user_id=excluded.owner_user_id, enabled=excluded.enabled,
           allowed_model_ids_json=excluded.allowed_model_ids_json,
           max_concurrent_runs=excluded.max_concurrent_runs,
           daily_token_limit=excluded.daily_token_limit,
           plugin_runtime_requested=CASE WHEN ?17 THEN excluded.plugin_runtime_requested ELSE plugin_runtime_requested END,
           plugin_policy_revision=CASE WHEN ?17 THEN excluded.plugin_policy_revision ELSE plugin_policy_revision END,
           plugin_policy_digest=CASE WHEN ?17 THEN excluded.plugin_policy_digest ELSE plugin_policy_digest END,
           plugin_consent_schema=CASE WHEN ?17 THEN excluded.plugin_consent_schema ELSE plugin_consent_schema END,
           plugin_consent_receipt_id=CASE WHEN ?17 THEN excluded.plugin_consent_receipt_id ELSE plugin_consent_receipt_id END,
           plugin_installation_identity_digest=CASE WHEN ?17 THEN excluded.plugin_installation_identity_digest ELSE plugin_installation_identity_digest END,
           plugin_authorization_ref=CASE WHEN ?17 THEN excluded.plugin_authorization_ref ELSE plugin_authorization_ref END,
           plugin_authorization_revision=CASE WHEN ?17 THEN excluded.plugin_authorization_revision ELSE plugin_authorization_revision END,
           plugin_authorization_digest=CASE WHEN ?17 THEN excluded.plugin_authorization_digest ELSE plugin_authorization_digest END,
           updated_at=excluded.updated_at",
        params![node_id, owner, update.enabled, allowed_json, update.max_concurrent_runs,
            update.daily_token_limit, intent.is_some_and(|value| value.plugin_runtime_requested),
            intent.map_or(0, |value| value.policy_revision), intent.map(|value| value.policy_digest.as_str()),
            intent.map(|_| NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1),
            intent.map(|value| value.consent_receipt_id.as_str()),
            intent.map(|value| value.installation_identity_digest.as_str()),
            intent.and_then(|value| value.authorization.as_ref()).map(|value| value.authorization_ref.as_str()),
            intent.and_then(|value| value.authorization.as_ref()).map(|value| value.revision),
            intent.and_then(|value| value.authorization.as_ref()).map(|value| value.digest.as_str()), ts,
            intent.is_some()],
    )?;
    Ok(())
}
