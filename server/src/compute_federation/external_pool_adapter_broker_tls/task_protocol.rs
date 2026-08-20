use anyhow::{anyhow, bail, Result};
use elon_external_pool_adapter_session_core::ExternalPoolAdapterTaskProtocolHostExchange;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zeroize::Zeroizing;

use super::{
    ExternalPoolAdapterBrokerTaskObservationValidator, ExternalPoolAdapterBrokerTlsChannel,
    VerifiedExternalPoolAdapterBrokerTaskExchange,
};

const MAX_REQUEST_BYTES: usize = 65_536;
const MAX_RESPONSE_BYTES: usize = 262_144;

/// Relays one exact child-produced request over one current TLS channel, then consumes the exact
/// response inside the ELTP exchange. No stream or raw request/response authority escapes.
pub(crate) async fn exchange_external_pool_adapter_broker_task<
    Validator: ExternalPoolAdapterBrokerTaskObservationValidator,
>(
    mut channel: ExternalPoolAdapterBrokerTlsChannel,
    exchange: ExternalPoolAdapterTaskProtocolHostExchange<'_>,
    validator: Validator,
) -> Result<VerifiedExternalPoolAdapterBrokerTaskExchange<Validator::Output>> {
    let request = exchange.request();
    let expected_response_bytes = exchange.expected_response_bytes();
    if request.is_empty()
        || request.len() > MAX_REQUEST_BYTES
        || expected_response_bytes == 0
        || expected_response_bytes > MAX_RESPONSE_BYTES
    {
        bail!("broker task exchange bounds rejected");
    }

    let stream = channel.begin_application_exchange()?;
    let mut response = Zeroizing::new(vec![0_u8; expected_response_bytes]);
    let timeout = exchange.remaining_timeout()?;
    tokio::time::timeout(timeout, async {
        stream.write_all(request).await?;
        stream.flush().await?;
        stream.read_exact(&mut response[..]).await?;
        Result::<_, std::io::Error>::Ok(())
    })
    .await
    .map_err(|_| anyhow!("broker task exchange timed out"))??;

    let (receipt, observation) = exchange.complete(&response, |raw| validator.validate(raw))?;
    Ok(VerifiedExternalPoolAdapterBrokerTaskExchange::new(
        receipt,
        observation,
    ))
}
