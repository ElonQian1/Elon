//! DNS resolution and per-request public-address pinning for open-commerce HTTPS calls.

#[path = "outbound_public_address_policy.rs"]
pub(crate) mod address_policy;

use anyhow::{anyhow, bail, Result};
use std::{future::Future, net::SocketAddr, time::Duration};

const DNS_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_DNS_ANSWERS: usize = 32;

pub(crate) struct PinnedHttpsTarget {
    pub client: reqwest::Client,
    pub url: String,
    #[cfg(test)]
    pub pinned_addresses: Vec<SocketAddr>,
}

pub(crate) async fn pinned_public_https_target(
    value: &str,
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<PinnedHttpsTarget> {
    let (url, host) = parse_standard_https_url(value)?;
    let addresses = collect_dns_answers_with_timeout(
        tokio::net::lookup_host((host.as_str(), 443)),
        DNS_TIMEOUT,
        MAX_DNS_ANSWERS,
    )
    .await?;
    build_pinned_target(url, host, addresses, connect_timeout, request_timeout)
}

async fn collect_dns_answers_with_timeout<F, I>(
    lookup: F,
    timeout: Duration,
    max_answers: usize,
) -> Result<Vec<SocketAddr>>
where
    F: Future<Output = std::io::Result<I>>,
    I: Iterator<Item = SocketAddr>,
{
    let lookup = tokio::time::timeout(timeout, lookup)
        .await
        .map_err(|_| anyhow!("HTTPS 出站主机解析超时"))?
        .map_err(|error| anyhow!("HTTPS 出站主机解析失败: {error}"))?;
    Ok(lookup
        .take(max_answers.saturating_add(1))
        .collect::<Vec<_>>())
}

fn parse_standard_https_url(value: &str) -> Result<(reqwest::Url, String)> {
    let url = reqwest::Url::parse(value).map_err(|_| anyhow!("HTTPS 出站地址无效"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(443)
    {
        bail!("HTTPS 出站地址必须使用标准 443 端口且不能包含账号信息");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("HTTPS 出站地址缺少主机"))?
        .to_ascii_lowercase();
    Ok((url, host))
}

fn build_pinned_target(
    url: reqwest::Url,
    host: String,
    addresses: Vec<SocketAddr>,
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<PinnedHttpsTarget> {
    let addresses =
        address_policy::validate_and_order_public_addresses(addresses, 443, MAX_DNS_ANSWERS)
            .map_err(|_| anyhow!("HTTPS 出站主机解析结果不符合公网地址策略"))?;
    let client = build_pinned_client(&host, &addresses, connect_timeout, request_timeout)?;
    Ok(PinnedHttpsTarget {
        client,
        url: url.to_string(),
        #[cfg(test)]
        pinned_addresses: addresses,
    })
}

fn build_pinned_client(
    host: &str,
    addresses: &[SocketAddr],
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(host, addresses)
        .build()?)
}

#[cfg(test)]
pub(crate) async fn pinned_public_https_target_or_local_test(
    value: &str,
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<PinnedHttpsTarget> {
    let url = reqwest::Url::parse(value).map_err(|_| anyhow!("HTTPS 出站地址无效"))?;
    let local_host = url
        .host_str()
        .is_some_and(|host| matches!(host, "127.0.0.1" | "localhost" | "::1"));
    if local_host && matches!(url.scheme(), "http" | "https") {
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(connect_timeout)
            .timeout(request_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        return Ok(PinnedHttpsTarget {
            client,
            url: url.to_string(),
            pinned_addresses: Vec::new(),
        });
    }
    pinned_public_https_target(value, connect_timeout, request_timeout).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    const TEST_TIMEOUT: Duration = Duration::from_secs(1);

    #[test]
    fn public_answers_are_all_pinned_in_deterministic_order() {
        let (url, host) = parse_standard_https_url("https://merchant.example/callback").unwrap();
        let public_v4 = SocketAddr::from(([8, 8, 8, 8], 443));
        let public_v6 = SocketAddr::new("2606:4700:4700::1111".parse::<IpAddr>().unwrap(), 443);
        let target = build_pinned_target(
            url,
            host,
            vec![public_v6, public_v4, public_v4],
            TEST_TIMEOUT,
            TEST_TIMEOUT,
        )
        .unwrap();
        assert_eq!(target.url, "https://merchant.example/callback");
        assert_eq!(target.pinned_addresses, vec![public_v4, public_v6]);
    }

    #[test]
    fn unsafe_mixed_empty_excessive_and_wrong_port_answers_fail_closed() {
        let public = SocketAddr::from(([8, 8, 8, 8], 443));
        let private = SocketAddr::from(([127, 0, 0, 1], 443));
        for answers in [vec![public, private], Vec::new()] {
            assert!(target_for_answers(answers).is_err());
        }
        assert!(target_for_answers(vec![SocketAddr::from(([8, 8, 8, 8], 8443))]).is_err());
        assert!(target_for_answers(vec![public; MAX_DNS_ANSWERS + 1]).is_err());
    }

    #[test]
    fn endpoint_requires_standard_https_without_credentials() {
        assert!(parse_standard_https_url("http://merchant.example/callback").is_err());
        assert!(parse_standard_https_url("https://merchant.example:8443/callback").is_err());
        assert!(parse_standard_https_url("https://user:pass@merchant.example/callback").is_err());
        assert!(parse_standard_https_url("https://merchant.example/callback?nonce=1").is_ok());
    }

    #[tokio::test]
    async fn dns_lookup_timeout_fails_closed() {
        let lookup = std::future::pending::<std::io::Result<std::vec::IntoIter<SocketAddr>>>();
        let error =
            collect_dns_answers_with_timeout(lookup, Duration::from_millis(1), MAX_DNS_ANSWERS)
                .await
                .unwrap_err();
        assert!(error.to_string().contains("解析超时"));
    }

    #[test]
    fn all_open_commerce_outbound_callers_use_the_shared_entrypoint() {
        for source in [
            include_str!("open_commerce_developer_domain_service.rs"),
            include_str!("open_commerce_webhook_verification.rs"),
            include_str!("open_commerce_webhook_worker.rs"),
            include_str!("open_commerce_runtime_client.rs"),
        ] {
            assert!(source.contains("open_commerce_outbound_security::pinned_public_https_target"));
        }
    }

    #[tokio::test]
    async fn pinned_client_uses_override_and_does_not_follow_redirects() -> Result<()> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut request = [0_u8; 1024];
            let read = stream.read(&mut request).await?;
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request
                .lines()
                .any(|line| line.eq_ignore_ascii_case("host: pinned.invalid")));
            stream
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: http://redirect.invalid/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await?;
            Ok::<(), anyhow::Error>(())
        });
        let client = build_pinned_client("pinned.invalid", &[address], TEST_TIMEOUT, TEST_TIMEOUT)?;
        let response = client.get("http://pinned.invalid/").send().await?;
        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        server.await??;
        Ok(())
    }

    fn target_for_answers(addresses: Vec<SocketAddr>) -> Result<PinnedHttpsTarget> {
        let (url, host) = parse_standard_https_url("https://merchant.example/callback")?;
        build_pinned_target(url, host, addresses, TEST_TIMEOUT, TEST_TIMEOUT)
    }
}
