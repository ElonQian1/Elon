use serde::{Deserialize, Serialize};

use super::common::{
    ExternalPoolAdapterTaskPollLineage, ExternalPoolAdapterTaskProductionBoundary,
    ExternalPoolAdapterTaskRemoteIdentity,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskPollCommandBinding {
    pub command_id: String,
    pub command_digest: String,
    pub outbox_id: String,
    pub outbox_digest: String,
    pub send_attempt_id: String,
    pub send_attempt_digest: String,
    pub route_authorization_id: String,
    pub route_authorization_digest: String,
    pub executor_binding_digest: String,
    pub fencing_generation: u64,
    pub fence_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskReconcilePollIntent {
    pub lineage: ExternalPoolAdapterTaskPollLineage,
    pub uncertain_exchange_attempt_id: String,
    pub uncertain_exchange_attempt_digest: String,
    pub command: ExternalPoolAdapterTaskPollCommandBinding,
    pub remote: ExternalPoolAdapterTaskRemoteIdentity,
    pub authenticated_subject_sha256: Option<String>,
    pub request_digest: String,
    pub not_before: String,
    pub not_after: String,
    pub created_at: String,
    pub boundary: ExternalPoolAdapterTaskProductionBoundary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskReconcilePollEnvelope {
    pub schema: String,
    pub reconcile_poll_id: String,
    pub reconcile_poll_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub poll: ExternalPoolAdapterTaskReconcilePollIntent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskEventCursor {
    pub remote_sequence: u64,
    pub previous_event_root: Option<String>,
    pub cursor_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskEventPollIntent {
    pub lineage: ExternalPoolAdapterTaskPollLineage,
    pub source_exchange_receipt_id: String,
    pub source_exchange_receipt_digest: String,
    pub command: ExternalPoolAdapterTaskPollCommandBinding,
    pub remote: ExternalPoolAdapterTaskRemoteIdentity,
    pub authenticated_subject_sha256: String,
    pub requested_cursor: ExternalPoolAdapterTaskEventCursor,
    pub request_digest: String,
    pub not_before: String,
    pub not_after: String,
    pub created_at: String,
    pub boundary: ExternalPoolAdapterTaskProductionBoundary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskEventPollEnvelope {
    pub schema: String,
    pub event_poll_id: String,
    pub event_poll_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub poll: ExternalPoolAdapterTaskEventPollIntent,
}
