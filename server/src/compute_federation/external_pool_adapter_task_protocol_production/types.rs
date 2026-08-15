mod common;
mod events;
mod exchange;
mod polls;

pub(crate) use common::*;
pub(crate) use events::*;
pub(crate) use exchange::*;
pub(crate) use polls::*;

pub(crate) const TASK_PRODUCTION_EXCHANGE_ATTEMPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_exchange_attempt.v1";
pub(crate) const TASK_PRODUCTION_EXCHANGE_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_exchange_receipt.v1";
pub(crate) const TASK_PRODUCTION_RECONCILE_POLL_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_reconcile_poll.v1";
pub(crate) const TASK_PRODUCTION_EVENT_POLL_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_event_poll.v1";
pub(crate) const TASK_PRODUCTION_EVENT_BATCH_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_event_batch.v1";
pub(crate) const TASK_PRODUCTION_EVENT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_event.v1";

pub(crate) const TASK_PRODUCTION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const TASK_PRODUCTION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const TASK_PRODUCTION_MAX_LEDGER_JSON_BYTES: usize = 512 * 1024;
pub(crate) const TASK_PRODUCTION_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub(crate) const TASK_PRODUCTION_MAX_EXCHANGE_ORDINAL: u64 = 64;
pub(crate) const TASK_PRODUCTION_MAX_REQUEST_BYTES: u64 = 262_144;
pub(crate) const TASK_PRODUCTION_MAX_UPSTREAM_REQUEST_BYTES: u64 = 65_536;
pub(crate) const TASK_PRODUCTION_MAX_RESPONSE_BYTES: u64 = 262_144;
pub(crate) const TASK_PRODUCTION_MAX_OBSERVATION_BYTES: u64 = 262_144;
pub(crate) const TASK_PRODUCTION_MAX_EVENTS_PER_BATCH: u64 = 256;

pub(crate) const TASK_PRODUCTION_NO_V213_AUTHORITY: &str =
    "production_transport_evidence_no_v213_authority";
pub(crate) const TASK_PRODUCTION_NO_EFFECT: &str = "none";

pub(crate) const TASK_PRODUCTION_SOURCE_START_SEND: &str = "start_outbox_send_attempt";
pub(crate) const TASK_PRODUCTION_SOURCE_RECONCILE_POLL: &str = "reconcile_poll";
pub(crate) const TASK_PRODUCTION_SOURCE_EVENT_POLL: &str = "event_poll";

pub(crate) const TASK_PRODUCTION_POLL_PENDING: &str = "pending";
pub(crate) const TASK_PRODUCTION_POLL_CLAIMED: &str = "claimed";
pub(crate) const TASK_PRODUCTION_POLL_IN_FLIGHT_UNKNOWN: &str = "in_flight_unknown";
pub(crate) const TASK_PRODUCTION_POLL_DELIVERY_OBSERVED: &str = "delivery_observed";
pub(crate) const TASK_PRODUCTION_POLL_QUARANTINED: &str = "quarantined";
