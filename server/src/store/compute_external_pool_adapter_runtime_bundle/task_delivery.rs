use std::time::Duration;

use anyhow::{bail, Result};
use elon_external_pool_adapter_session_core::{
    ExternalPoolAdapterTaskProtocolHost, PreparedExternalPoolAdapterTaskRequest,
};

use crate::compute_federation::external_pool_adapter_broker_tls::{
    exchange_external_pool_adapter_broker_task, ExternalPoolAdapterBrokerTaskObservationValidator,
    ExternalPoolAdapterBrokerTlsChannel, VerifiedExternalPoolAdapterBrokerTaskExchange,
};

const MAX_TOTAL_EXCHANGE_TIMEOUT: Duration = Duration::from_millis(15_000);

/// Dormant transaction-free production relay kernel. It grants no v213 send, route, executor,
/// fence, ACK, event, Lease, or activation authority and deliberately exposes no raw payload.
/// The semantic validator must be pure and bounded: synchronous validation is not preempted, and
/// crossing the shared absolute deadline is terminal and cannot produce a receipt.
#[allow(dead_code)]
pub(super) async fn exchange_external_pool_adapter_task_delivery<
    Validator: ExternalPoolAdapterBrokerTaskObservationValidator,
>(
    host: &mut ExternalPoolAdapterTaskProtocolHost<'_>,
    channel: ExternalPoolAdapterBrokerTlsChannel,
    request: PreparedExternalPoolAdapterTaskRequest,
    delivery_attempt_digest: &str,
    timeout: Duration,
    validator: Validator,
) -> Result<VerifiedExternalPoolAdapterBrokerTaskExchange<Validator::Output>> {
    if timeout.is_zero() || timeout > MAX_TOTAL_EXCHANGE_TIMEOUT {
        bail!("task delivery total exchange timeout is invalid");
    }
    let exchange = host.begin(request, delivery_attempt_digest, timeout)?;
    exchange_external_pool_adapter_broker_task(channel, exchange, validator).await
}
