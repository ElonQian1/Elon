use serde::{Deserialize, Serialize};

use super::common::{
    ExternalPoolAdapterTaskProductionBoundary, ExternalPoolAdapterTaskRemoteIdentity,
};
use super::polls::ExternalPoolAdapterTaskEventCursor;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskAuthenticatedEventObservation {
    pub remote: ExternalPoolAdapterTaskRemoteIdentity,
    pub cursor_before: ExternalPoolAdapterTaskEventCursor,
    pub cursor_after: ExternalPoolAdapterTaskEventCursor,
    pub previous_batch_root: Option<String>,
    pub batch_root: String,
    pub replay_classification: String,
    pub event_count: u64,
    pub event_roots: Vec<String>,
    pub event_inventory_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskEventBatchMaterial {
    pub event_poll_id: String,
    pub event_poll_digest: String,
    pub exchange_receipt_id: String,
    pub exchange_receipt_digest: String,
    pub predecessor_event_batch_id: Option<String>,
    pub predecessor_event_batch_digest: Option<String>,
    pub remote: ExternalPoolAdapterTaskRemoteIdentity,
    pub authenticated_observation_sha256: String,
    pub cursor_before: ExternalPoolAdapterTaskEventCursor,
    pub cursor_after: ExternalPoolAdapterTaskEventCursor,
    pub previous_batch_root: Option<String>,
    pub batch_root: String,
    pub replay_classification: String,
    pub event_count: u64,
    pub event_roots: Vec<String>,
    pub event_inventory_digest: String,
    pub authenticated_at: String,
    pub received_at: String,
    pub recorded_at: String,
    pub boundary: ExternalPoolAdapterTaskProductionBoundary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskEventBatchEnvelope {
    pub schema: String,
    pub event_batch_id: String,
    pub event_batch_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub batch: ExternalPoolAdapterTaskEventBatchMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskEventMaterial {
    pub event_batch_id: String,
    pub event_batch_digest: String,
    pub remote_identity_digest: String,
    pub event_ordinal: u64,
    pub remote_event_id: String,
    pub event_type: String,
    pub remote_sequence: u64,
    pub previous_event_root: Option<String>,
    pub event_root: String,
    pub canonical_event_digest: String,
    pub observed_at: String,
    pub recorded_at: String,
    pub boundary: ExternalPoolAdapterTaskProductionBoundary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskEventEnvelope {
    pub schema: String,
    pub event_id: String,
    pub event_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub event: ExternalPoolAdapterTaskEventMaterial,
}
