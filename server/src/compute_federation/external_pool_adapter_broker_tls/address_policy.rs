use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use anyhow::{bail, Result};

pub(super) fn validate_and_order_dns_answers(
    addresses: impl IntoIterator<Item = SocketAddr>,
    expected_port: u16,
    max_answers: usize,
) -> Result<Vec<SocketAddr>> {
    if expected_port == 0 || max_answers == 0 {
        bail!("broker DNS policy rejected");
    }
    let mut unique = BTreeSet::new();
    let mut answer_count = 0_usize;
    for address in addresses {
        answer_count = answer_count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("broker DNS answer count overflow"))?;
        if answer_count > max_answers
            || address.port() != expected_port
            || !is_public_unicast(address.ip())
        {
            bail!("broker DNS policy rejected");
        }
        unique.insert(address);
    }
    if unique.is_empty() {
        bail!("broker DNS policy rejected");
    }
    let mut ordered = unique.into_iter().collect::<Vec<_>>();
    ordered.sort_by_key(|address| (address.is_ipv6(), address.ip(), address.port()));
    Ok(ordered)
}

pub(super) fn is_public_unicast(ip: IpAddr) -> bool {
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
        || (a == 192 && b == 88 && c == 99)
        || (a == 192 && b == 168)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
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
    segments[..6] != [0, 0, 0, 0, 0, 0]
}
