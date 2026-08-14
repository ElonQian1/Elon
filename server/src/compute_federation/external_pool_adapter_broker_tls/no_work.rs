use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zeroize::Zeroizing;

use super::ExternalPoolAdapterBrokerTlsChannel;

const MAX_REQUEST_BYTES: usize = 16_384;
const MAX_RESPONSE_BYTES: usize = 65_536;
const MAX_PROBE_TIMEOUT: Duration = Duration::from_millis(15_000);

/// Performs one exact request/write and one exact-length response/read on a current TLS channel.
/// The channel cannot be reused, and no generic stream authority escapes this module.
pub(crate) async fn exchange_external_pool_adapter_broker_no_work(
    channel: &mut ExternalPoolAdapterBrokerTlsChannel,
    request: &[u8],
    expected_response_bytes: usize,
    timeout: Duration,
) -> Result<Zeroizing<Vec<u8>>> {
    if request.is_empty()
        || request.len() > MAX_REQUEST_BYTES
        || expected_response_bytes == 0
        || expected_response_bytes > MAX_RESPONSE_BYTES
        || timeout.is_zero()
        || timeout > MAX_PROBE_TIMEOUT
    {
        bail!("broker no-work exchange bounds rejected");
    }
    let stream = channel.begin_application_exchange()?;
    let mut response = Zeroizing::new(vec![0_u8; expected_response_bytes]);
    tokio::time::timeout(timeout, async {
        stream.write_all(request).await?;
        stream.flush().await?;
        stream.read_exact(&mut response[..]).await?;
        Result::<_, std::io::Error>::Ok(())
    })
    .await
    .map_err(|_| anyhow!("broker no-work exchange timed out"))??;
    Ok(response)
}
