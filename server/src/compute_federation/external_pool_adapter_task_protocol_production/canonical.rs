use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::*;

const ATTEMPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-EXCHANGE-ATTEMPT-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-EXCHANGE-RECEIPT-V1";
const RECONCILE_POLL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-RECONCILE-POLL-V1";
const EVENT_POLL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-POLL-V1";
const EVENT_BATCH_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-BATCH-V1";
const EVENT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-V1";
const EVENT_CURSOR_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-CURSOR-V1";
const EVENT_ROOT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-ROOT-V1";
const EVENT_INVENTORY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-INVENTORY-V1";
const EVENT_BATCH_ROOT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-EVENT-BATCH-ROOT-V1";
const REMOTE_IDENTITY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-REMOTE-IDENTITY-V1";

pub(crate) fn canonical_task_production_exchange_attempt_json_and_digest(
    value: &ExternalPoolAdapterTaskExchangeAttemptEnvelope,
) -> Result<(String, String)> {
    envelope_digest(
        value,
        "exchange_attempt_digest",
        ATTEMPT_DOMAIN,
        "task production exchange attempt",
    )
}

pub(crate) fn canonical_task_production_exchange_receipt_json_and_digest(
    value: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<(String, String)> {
    envelope_digest(
        value,
        "exchange_receipt_digest",
        RECEIPT_DOMAIN,
        "task production exchange receipt",
    )
}

pub(crate) fn canonical_task_production_reconcile_poll_json_and_digest(
    value: &ExternalPoolAdapterTaskReconcilePollEnvelope,
) -> Result<(String, String)> {
    envelope_digest(
        value,
        "reconcile_poll_digest",
        RECONCILE_POLL_DOMAIN,
        "task production reconcile poll",
    )
}

pub(crate) fn canonical_task_production_event_poll_json_and_digest(
    value: &ExternalPoolAdapterTaskEventPollEnvelope,
) -> Result<(String, String)> {
    envelope_digest(
        value,
        "event_poll_digest",
        EVENT_POLL_DOMAIN,
        "task production event poll",
    )
}

pub(crate) fn canonical_task_production_event_batch_json_and_digest(
    value: &ExternalPoolAdapterTaskEventBatchEnvelope,
) -> Result<(String, String)> {
    envelope_digest(
        value,
        "event_batch_digest",
        EVENT_BATCH_DOMAIN,
        "task production event batch",
    )
}

pub(crate) fn canonical_task_production_event_json_and_digest(
    value: &ExternalPoolAdapterTaskEventEnvelope,
) -> Result<(String, String)> {
    envelope_digest(value, "event_digest", EVENT_DOMAIN, "task production event")
}

pub(crate) fn task_production_event_cursor_digest(
    remote_sequence: u64,
    previous_event_root: Option<&str>,
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        remote_sequence: u64,
        previous_event_root: Option<&'a str>,
    }
    domain_digest(
        EVENT_CURSOR_DOMAIN,
        &Material {
            remote_sequence,
            previous_event_root,
        },
    )
}

pub(crate) fn task_production_remote_identity_digest(
    executor_binding_digest: &str,
    remote_execution_id: Option<&str>,
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        executor_binding_digest: &'a str,
        remote_execution_id: Option<&'a str>,
    }
    domain_digest(
        REMOTE_IDENTITY_DOMAIN,
        &Material {
            executor_binding_digest,
            remote_execution_id,
        },
    )
}

pub(crate) fn canonical_task_production_remote_subject_json_and_sha256(
    value: &ExternalPoolAdapterTaskAuthenticatedRemoteSubject,
) -> Result<(String, String)> {
    canonical_compute_plugin_ijson_and_sha256(value, TASK_PRODUCTION_MAX_OBSERVATION_BYTES as usize)
}

pub(crate) fn canonical_task_production_authenticated_event_observation_json_and_sha256(
    value: &ExternalPoolAdapterTaskAuthenticatedEventObservation,
) -> Result<(String, String)> {
    canonical_compute_plugin_ijson_and_sha256(value, TASK_PRODUCTION_MAX_OBSERVATION_BYTES as usize)
}

pub(crate) fn task_production_authenticated_event_observation(
    value: &ExternalPoolAdapterTaskEventBatchMaterial,
) -> ExternalPoolAdapterTaskAuthenticatedEventObservation {
    ExternalPoolAdapterTaskAuthenticatedEventObservation {
        remote: value.remote.clone(),
        cursor_before: value.cursor_before.clone(),
        cursor_after: value.cursor_after.clone(),
        previous_batch_root: value.previous_batch_root.clone(),
        batch_root: value.batch_root.clone(),
        replay_classification: value.replay_classification.clone(),
        event_count: value.event_count,
        event_roots: value.event_roots.clone(),
        event_inventory_digest: value.event_inventory_digest.clone(),
    }
}

pub(crate) fn task_production_event_root(
    value: &ExternalPoolAdapterTaskEventMaterial,
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        remote_identity_digest: &'a str,
        event_ordinal: u64,
        remote_event_id: &'a str,
        event_type: &'a str,
        remote_sequence: u64,
        previous_event_root: Option<&'a str>,
        canonical_event_digest: &'a str,
    }
    domain_digest(
        EVENT_ROOT_DOMAIN,
        &Material {
            remote_identity_digest: &value.remote_identity_digest,
            event_ordinal: value.event_ordinal,
            remote_event_id: &value.remote_event_id,
            event_type: &value.event_type,
            remote_sequence: value.remote_sequence,
            previous_event_root: value.previous_event_root.as_deref(),
            canonical_event_digest: &value.canonical_event_digest,
        },
    )
}

pub(crate) fn task_production_event_inventory_digest(roots: &[String]) -> Result<String> {
    domain_digest(EVENT_INVENTORY_DOMAIN, roots)
}

pub(crate) fn task_production_event_batch_root(
    value: &ExternalPoolAdapterTaskEventBatchMaterial,
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        remote_identity_digest: &'a str,
        cursor_before_digest: &'a str,
        cursor_after_digest: &'a str,
        previous_batch_root: Option<&'a str>,
        replay_classification: &'a str,
        event_count: u64,
        event_inventory_digest: &'a str,
    }
    domain_digest(
        EVENT_BATCH_ROOT_DOMAIN,
        &Material {
            remote_identity_digest: &value.remote.remote_identity_digest,
            cursor_before_digest: &value.cursor_before.cursor_digest,
            cursor_after_digest: &value.cursor_after.cursor_digest,
            previous_batch_root: value.previous_batch_root.as_deref(),
            replay_classification: &value.replay_classification,
            event_count: value.event_count,
            event_inventory_digest: &value.event_inventory_digest,
        },
    )
}

fn envelope_digest<T: Serialize>(
    value: &T,
    digest_field: &str,
    domain: &[u8],
    kind: &str,
) -> Result<(String, String)> {
    let object = serde_json::to_value(value)?;
    let mut projection = object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{kind} must be an object"))?
        .clone();
    if projection
        .insert(
            digest_field.into(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("{kind} lacks its digest field")
    }
    Ok((canonical_json(value)?, domain_digest(domain, &projection)?))
}

pub(super) fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_json(value)?.as_bytes());
    Ok(hex::encode(digest.finalize()))
}

pub(super) fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, TASK_PRODUCTION_MAX_LEDGER_JSON_BYTES)
        .map(|item| item.0)
}
