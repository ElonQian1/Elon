//! Cross-platform sealed semantic boundary for one authenticated task exchange.

use anyhow::Result;
use elon_external_pool_adapter_session_core::ExternalPoolAdapterTaskProtocolHostReceipt;

use crate::compute_federation::{
    attempt_gateway::{ComputeAttemptAdapterAckEnvelope, ComputeAttemptAdapterBinding},
    external_pool_adapter_task_protocol_production::{
        ExternalPoolAdapterTaskEventBatchEnvelope, ExternalPoolAdapterTaskEventEnvelope,
        ExternalPoolAdapterTaskEventPollEnvelope, ExternalPoolAdapterTaskReconcilePollEnvelope,
    },
    start_outbox::ComputeStartOutboxRemoteObservationEnvelope,
};

mod sealed {
    pub(super) trait Sealed {}
    pub(super) trait VerifiedObservation {}
}

/// Closed semantic projection produced from the exact child observation bytes. Store callers can
/// ask it to validate durable projections, but cannot implement or substitute this authority.
pub(crate) trait ExternalPoolAdapterBrokerTaskVerifiedObservation:
    sealed::VerifiedObservation + Send
{
    fn validate_reconcile_poll(
        &self,
        poll: &ExternalPoolAdapterTaskReconcilePollEnvelope,
    ) -> Result<()>;

    fn validate_event_poll(&self, poll: &ExternalPoolAdapterTaskEventPollEnvelope) -> Result<()>;

    fn validate_event_ingress(
        &self,
        batch: &ExternalPoolAdapterTaskEventBatchEnvelope,
        events: &[ExternalPoolAdapterTaskEventEnvelope],
        successor: Option<&ExternalPoolAdapterTaskEventPollEnvelope>,
    ) -> Result<()>;

    fn validate_terminal_ack(
        &self,
        adapter: &ComputeAttemptAdapterBinding,
        ack: &ComputeAttemptAdapterAckEnvelope,
        observation: &ComputeStartOutboxRemoteObservationEnvelope,
    ) -> Result<()>;

    fn validate_terminal_no_start(
        &self,
        observation: &ComputeStartOutboxRemoteObservationEnvelope,
    ) -> Result<()>;
}

/// Closed semantic validator boundary. Implementations must live in this transport module, so a
/// crate sibling cannot choose `Vec<u8>` or otherwise return the raw response as an authority.
pub(crate) trait ExternalPoolAdapterBrokerTaskObservationValidator:
    sealed::Sealed + Send
{
    type Output: ExternalPoolAdapterBrokerTaskVerifiedObservation;

    fn validate(self, response: &[u8]) -> Result<Self::Output>;
}

/// The HostReceipt and its typed semantic observation cannot be split and recombined by a caller.
pub(crate) struct VerifiedExternalPoolAdapterBrokerTaskExchange<
    Observation: ExternalPoolAdapterBrokerTaskVerifiedObservation,
> {
    receipt: ExternalPoolAdapterTaskProtocolHostReceipt,
    observation: Observation,
}

impl<Observation: ExternalPoolAdapterBrokerTaskVerifiedObservation>
    VerifiedExternalPoolAdapterBrokerTaskExchange<Observation>
{
    pub(super) fn new(
        receipt: ExternalPoolAdapterTaskProtocolHostReceipt,
        observation: Observation,
    ) -> Self {
        Self {
            receipt,
            observation,
        }
    }

    pub(crate) fn into_parts(self) -> (ExternalPoolAdapterTaskProtocolHostReceipt, Observation) {
        (self.receipt, self.observation)
    }
}
