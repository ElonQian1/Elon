//! DNS resolution and per-request public-address pinning for open-commerce HTTPS calls.

use anyhow::{anyhow, bail, Result};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};

pub(crate) struct PinnedHttpsTarget {
    pub client: reqwest::Client,
    pub url: String,
}

pub(crate) async fn pinned_public_https_target(
    value: &str,
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<PinnedHttpsTarget> {
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
    let addresses = tokio::net::lookup_host((host.as_str(), 443))
        .await
        .map_err(|error| anyhow!("HTTPS 出站主机解析失败: {error}"))?
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        bail!("HTTPS 出站主机没有可用地址");
    }
    if addresses.iter().any(|address| !is_public_ip(address.ip())) {
        bail!("HTTPS 出站主机解析到私网或特殊用途地址");
    }
    let selected = preferred_address(&addresses);
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .redirect(reqwest::redirect::Policy::none())
        .resolve(&host, selected)
        .build()?;
    Ok(PinnedHttpsTarget {
        client,
        url: url.to_string(),
    })
}

fn preferred_address(addresses: &[SocketAddr]) -> SocketAddr {
    addresses
        .iter()
        .copied()
        .find(|address| address.is_ipv4())
        .unwrap_or(addresses[0])
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 192 && b == 168)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() {
        return false;
    }
    let segments = ip.segments();
    if (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] & 0xffc0) == 0xfec0
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || segments[0] == 0x2002
        || (segments[0] == 0x0064 && segments[1] == 0xff9b)
    {
        return false;
    }
    if segments[..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff {
        return is_public_ipv4(Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            segments[6] as u8,
            (segments[7] >> 8) as u8,
            segments[7] as u8,
        ));
    }
    segments[..6] != [0, 0, 0, 0, 0, 0]
}
