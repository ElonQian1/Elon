use std::time::Duration;

use anyhow::{bail, Result};
use elon_external_pool_adapter_session_core::{
    ExternalPoolAdapterTaskProtocolHost, ExternalPoolAdapterTaskProtocolHostReceipt,
    PreparedExternalPoolAdapterTaskRequest,
};

use crate::compute_federation::external_pool_adapter_broker_tls::{
    exchange_external_pool_adapter_broker_task, ExternalPoolAdapterBrokerTlsChannel,
};

const MAX_TOTAL_EXCHANGE_TIMEOUT: Duration = Duration::from_millis(15_000);

/// Dormant transaction-free production relay kernel. It grants no v213 send, route, executor,
/// fence, ACK, event, Lease, or activation authority and deliberately exposes no raw payload.
/// The semantic validator must be pure and bounded: synchronous validation is not preempted, and
/// crossing the shared absolute deadline is terminal and cannot produce a receipt.
#[allow(dead_code)]
pub(super) async fn exchange_external_pool_adapter_task_delivery(
    host: &mut ExternalPoolAdapterTaskProtocolHost<'_>,
    channel: ExternalPoolAdapterBrokerTlsChannel,
    request: PreparedExternalPoolAdapterTaskRequest,
    delivery_attempt_digest: &str,
    timeout: Duration,
    validate_observation: impl FnOnce(&[u8]) -> Result<()> + Send,
) -> Result<ExternalPoolAdapterTaskProtocolHostReceipt> {
    if timeout.is_zero() || timeout > MAX_TOTAL_EXCHANGE_TIMEOUT {
        bail!("task delivery total exchange timeout is invalid");
    }
    let exchange = host.begin(request, delivery_attempt_digest, timeout)?;
    exchange_external_pool_adapter_broker_task(channel, exchange, validate_observation).await
}
