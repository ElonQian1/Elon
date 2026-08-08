//! Durable consent, delivery outbox and observed ACKs for the opt-in compute plugin runtime.

use super::{
    clean_optional, new_id,
    node_compute_plugin_sharing_digest::{request_facts_digest, resolved_policy_digest},
    node_compute_plugin_sharing_rows::{
        select_consent_by_request, select_current_intent, upsert_policy,
    },
    node_compute_sharing::{
        normalize_model_ids, sharing_status, NodeComputeSharingStatus,
        UpdateNodeComputeSharingPolicy,
    },
    now, Store,
};
use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::Serialize;

pub const NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1: &str =
    "elon.node_compute_plugin.sharing_consent.v1";
const MAX_SAFE_REVISION: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NodeComputePluginSharingAuthorization {
    pub authorization_ref: String,
    pub revision: i64,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeComputePluginSharingDispatchIntent {
    pub delivery_id: String,
    pub consent_receipt_id: String,
    pub node_id: String,
    pub owner_user_id: String,
    pub installation_identity_digest: String,
    pub policy_revision: i64,
    pub policy_digest: String,
    pub plugin_runtime_requested: bool,
    pub authorization: Option<NodeComputePluginSharingAuthorization>,
    pub replayed: bool,
    pub dispatchable: bool,
}

pub struct NodeComputePluginSharingConsentRequest {
    pub plugin_runtime_requested: bool,
    pub expected_policy_revision: i64,
    pub consent_request_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeComputePluginSharingControlSummary {
    pub latest_delivery_kind: Option<String>,
    pub latest_delivery_detail_code: Option<String>,
    pub latest_delivery_at: Option<String>,
    pub latest_observation: Option<serde_json::Value>,
    pub latest_observed_at: Option<String>,
}

pub struct NodeComputeSharingUpdateOutcome {
    pub status: NodeComputeSharingStatus,
    pub dispatch_intent: Option<NodeComputePluginSharingDispatchIntent>,
}

impl Store {
    pub fn update_node_compute_sharing_policy_with_plugin_runtime(
        &self,
        owner_user_id: &str,
        node_id: &str,
        update: UpdateNodeComputeSharingPolicy,
        plugin_consent: Option<NodeComputePluginSharingConsentRequest>,
    ) -> Result<NodeComputeSharingUpdateOutcome> {
        let node_id = node_id.trim();
        let owner_user_id = owner_user_id.trim();
        let allowed_model_ids = validate_update(node_id, owner_user_id, &update)?;
        if plugin_consent
            .as_ref()
            .is_some_and(|consent| consent.plugin_runtime_requested != update.enabled)
        {
            bail!("同一共享开关的插件运行意图必须与模型共享状态一致");
        }
        let allowed_json = serde_json::to_string(&allowed_model_ids)?;
        let ts = now();
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let install_id = select_owned_credential_install_id(&tx, node_id, owner_user_id)?;

        let Some(consent) = plugin_consent else {
            ensure_legacy_update_preserves_plugin_policy(
                &tx,
                node_id,
                owner_user_id,
                &update,
                &allowed_model_ids,
            )?;
            upsert_policy(
                &tx,
                node_id,
                owner_user_id,
                &update,
                &allowed_json,
                None,
                &ts,
            )?;
            let status = sharing_status(&tx, node_id, owner_user_id, None)?;
            tx.commit()?;
            return Ok(NodeComputeSharingUpdateOutcome {
                status,
                dispatch_intent: None,
            });
        };
        let install_id = install_id
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("节点缺少稳定安装身份，不能授权算力插件"))?;
        let request_id = validate_consent_request(&consent)?;
        let installation_digest = crate::compute_plugin_sharing_directive::
            derive_compute_plugin_installation_identity_digest(&install_id)
            .map_err(|error| anyhow::anyhow!(error.code()))?;
        let request_facts_digest = request_facts_digest(
            node_id,
            owner_user_id,
            &installation_digest,
            &update,
            &allowed_model_ids,
            &consent,
        )?;
        let current_revision = select_current_revision(&tx, node_id)?;
        if let Some(mut stored) = select_consent_by_request(&tx, node_id, &request_id)? {
            if stored.request_facts_digest != request_facts_digest {
                bail!("同一插件同意请求编号不能改变策略事实");
            }
            stored.intent.replayed = true;
            stored.intent.dispatchable = current_revision == stored.intent.policy_revision;
            let status = sharing_status(&tx, node_id, owner_user_id, None)?;
            tx.commit()?;
            return Ok(NodeComputeSharingUpdateOutcome {
                status,
                dispatch_intent: Some(stored.intent),
            });
        }
        if consent.expected_policy_revision != current_revision {
            bail!("算力插件策略修订号已变化，请刷新后重试");
        }
        let policy_revision = current_revision
            .checked_add(1)
            .filter(|value| *value <= MAX_SAFE_REVISION)
            .ok_or_else(|| anyhow::anyhow!("节点算力插件策略修订号已耗尽"))?;
        let consent_receipt_id = new_id("cpsc");
        let authorization_ref = consent.plugin_runtime_requested.then(|| new_id("cpsa"));
        let policy_digest = resolved_policy_digest(
            node_id,
            owner_user_id,
            &installation_digest,
            policy_revision,
            &update,
            &allowed_model_ids,
            consent.plugin_runtime_requested,
            authorization_ref.as_deref(),
        )?;
        let authorization =
            authorization_ref.map(|authorization_ref| NodeComputePluginSharingAuthorization {
                authorization_ref,
                revision: policy_revision,
                digest: policy_digest.clone(),
            });
        tx.execute(
            "INSERT INTO node_compute_plugin_sharing_consents (
               receipt_id, node_id, owner_user_id, consent_schema,
               installation_identity_digest, consent_request_id, request_facts_digest,
               policy_revision, policy_digest, plugin_runtime_requested,
               allowed_model_ids_json, max_concurrent_runs, daily_token_limit,
               authorization_ref, authorization_revision, authorization_digest, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                consent_receipt_id,
                node_id,
                owner_user_id,
                NODE_COMPUTE_PLUGIN_SHARING_CONSENT_SCHEMA_V1,
                installation_digest,
                request_id,
                request_facts_digest,
                policy_revision,
                policy_digest,
                consent.plugin_runtime_requested,
                allowed_json,
                update.max_concurrent_runs,
                update.daily_token_limit,
                authorization
                    .as_ref()
                    .map(|value| value.authorization_ref.as_str()),
                authorization.as_ref().map(|value| value.revision),
                authorization.as_ref().map(|value| value.digest.as_str()),
                ts
            ],
        )?;
        let intent = NodeComputePluginSharingDispatchIntent {
            delivery_id: new_id("cpsd"),
            consent_receipt_id,
            node_id: node_id.to_string(),
            owner_user_id: owner_user_id.to_string(),
            installation_identity_digest: installation_digest,
            policy_revision,
            policy_digest,
            plugin_runtime_requested: consent.plugin_runtime_requested,
            authorization,
            replayed: false,
            dispatchable: true,
        };
        insert_delivery_intent(&tx, &intent, &ts)?;
        upsert_policy(
            &tx,
            node_id,
            owner_user_id,
            &update,
            &allowed_json,
            Some(&intent),
            &ts,
        )?;
        let status = sharing_status(&tx, node_id, owner_user_id, None)?;
        tx.commit()?;
        Ok(NodeComputeSharingUpdateOutcome {
            status,
            dispatch_intent: Some(intent),
        })
    }

    /// Creates one fresh delivery for the current desired snapshot on every authenticated node
    /// session. A historical ACK belongs to an earlier process/session and must not suppress the
    /// replay that rebuilds this process-local Bootstrap state.
    pub fn prepare_node_compute_plugin_sharing_session_delivery(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeComputePluginSharingDispatchIntent>> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(mut intent) = select_current_intent(&tx, node_id.trim())? else {
            tx.commit()?;
            return Ok(None);
        };
        intent.delivery_id = new_id("cpsd");
        intent.replayed = true;
        intent.dispatchable = true;
        insert_delivery_intent(&tx, &intent, &now())?;
        tx.commit()?;
        Ok(Some(intent))
    }

    pub fn record_node_compute_plugin_sharing_delivery(
        &self,
        intent: &NodeComputePluginSharingDispatchIntent,
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
            bail!("未知算力插件下发结果");
        }
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let sequence = tx.query_row(
            "SELECT COALESCE(MAX(event_sequence), 0) + 1
               FROM node_compute_plugin_sharing_delivery_events WHERE delivery_id=?1",
            params![intent.delivery_id],
            |row| row.get::<_, i64>(0),
        )?;
        tx.execute(
            "INSERT INTO node_compute_plugin_sharing_delivery_events (
               id, delivery_id, node_id, consent_receipt_id, policy_revision,
               policy_digest, event_sequence, event_kind, detail_code, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                new_id("cpse"),
                intent.delivery_id,
                intent.node_id,
                intent.consent_receipt_id,
                intent.policy_revision,
                intent.policy_digest,
                sequence,
                event_kind,
                clean_optional(detail_code),
                now()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_node_compute_plugin_sharing_observation(
        &self,
        intent: &NodeComputePluginSharingDispatchIntent,
        accepted: bool,
        observed: &serde_json::Value,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO node_compute_plugin_sharing_observations (
               id, delivery_id, node_id, consent_receipt_id, policy_revision,
               policy_digest, accepted, observed_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                new_id("cpso"),
                intent.delivery_id,
                intent.node_id,
                intent.consent_receipt_id,
                intent.policy_revision,
                intent.policy_digest,
                accepted,
                serde_json::to_string(observed)?,
                now()
            ],
        )?;
        Ok(())
    }

    pub fn node_compute_plugin_sharing_control_summary(
        &self,
        node_id: &str,
    ) -> Result<NodeComputePluginSharingControlSummary> {
        let conn = self.conn.lock().unwrap();
        let delivery = conn.query_row(
            "SELECT event_kind, detail_code, created_at FROM node_compute_plugin_sharing_delivery_events
              WHERE node_id=?1 ORDER BY created_at DESC, id DESC LIMIT 1",
            params![node_id.trim()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional()?;
        let observation = conn
            .query_row(
                "SELECT observed_json, created_at FROM node_compute_plugin_sharing_observations
              WHERE node_id=?1 ORDER BY created_at DESC, id DESC LIMIT 1",
                params![node_id.trim()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let (latest_observation, latest_observed_at) = match observation {
            Some((json, at)) => (Some(serde_json::from_str(&json)?), Some(at)),
            None => (None, None),
        };
        Ok(NodeComputePluginSharingControlSummary {
            latest_delivery_kind: delivery.as_ref().map(|value| value.0.clone()),
            latest_delivery_detail_code: delivery.as_ref().and_then(|value| value.1.clone()),
            latest_delivery_at: delivery.map(|value| value.2),
            latest_observation,
            latest_observed_at,
        })
    }
}

fn validate_update(
    node_id: &str,
    owner_user_id: &str,
    update: &UpdateNodeComputeSharingPolicy,
) -> Result<Vec<String>> {
    if node_id.is_empty() || owner_user_id.is_empty() {
        bail!("节点和所有者不能为空");
    }
    let models = normalize_model_ids(&update.allowed_model_ids)?;
    if update.enabled && models.is_empty() {
        bail!("开启共享前至少选择一个允许共享的模型");
    }
    if !(1..=16).contains(&update.max_concurrent_runs) {
        bail!("共享并发上限必须在 1 到 16 之间");
    }
    if !(0..=1_000_000_000_000).contains(&update.daily_token_limit) {
        bail!("每日 Token 上限必须在 0 到 1000000000000 之间");
    }
    Ok(models)
}

fn validate_consent_request(request: &NodeComputePluginSharingConsentRequest) -> Result<String> {
    let request_id = request.consent_request_id.trim();
    if request.expected_policy_revision < 0 || request.expected_policy_revision > MAX_SAFE_REVISION
    {
        bail!("算力插件期望策略修订号无效");
    }
    if request_id.is_empty() || request_id.len() > 200 || request_id.chars().any(char::is_control) {
        bail!("算力插件同意请求编号无效");
    }
    Ok(request_id.to_string())
}

fn ensure_legacy_update_preserves_plugin_policy(
    tx: &Transaction<'_>,
    node_id: &str,
    owner_user_id: &str,
    update: &UpdateNodeComputeSharingPolicy,
    allowed_model_ids: &[String],
) -> Result<()> {
    let current = sharing_status(tx, node_id, owner_user_id, None)?.policy;
    if current.plugin_policy_revision > 0
        && (current.enabled != update.enabled
            || current.allowed_model_ids != allowed_model_ids
            || current.max_concurrent_runs != update.max_concurrent_runs
            || current.daily_token_limit != update.daily_token_limit)
    {
        bail!("节点已有算力插件策略；旧客户端不能改变已被授权摘要承诺的共享事实，请使用新版客户端重新保存");
    }
    Ok(())
}

fn select_owned_credential_install_id(
    tx: &Transaction<'_>,
    node_id: &str,
    owner: &str,
) -> Result<Option<String>> {
    let install_id = tx
        .query_row(
            "SELECT install_id FROM node_credentials WHERE agent_id=?1 AND owner_user_id=?2",
            params![node_id, owner],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("节点不存在或不属于当前用户"))?;
    Ok(install_id)
}

fn select_current_revision(tx: &Transaction<'_>, node_id: &str) -> Result<i64> {
    Ok(tx
        .query_row(
            "SELECT plugin_policy_revision FROM node_compute_sharing_policies WHERE node_id=?1",
            params![node_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0))
}

fn insert_delivery_intent(
    tx: &Transaction<'_>,
    intent: &NodeComputePluginSharingDispatchIntent,
    ts: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO node_compute_plugin_sharing_delivery_events (
           id, delivery_id, node_id, consent_receipt_id, policy_revision,
           policy_digest, event_sequence, event_kind, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 'intent_committed', ?7)",
        params![
            new_id("cpse"),
            intent.delivery_id,
            intent.node_id,
            intent.consent_receipt_id,
            intent.policy_revision,
            intent.policy_digest,
            ts
        ],
    )?;
    Ok(())
}
