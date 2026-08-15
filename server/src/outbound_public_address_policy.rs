//! Shared fail-closed public-address policy for server-owned outbound transports.

use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use anyhow::{bail, Result};

pub(crate) fn validate_and_order_public_addresses(
    addresses: impl IntoIterator<Item = SocketAddr>,
    expected_port: u16,
    max_answers: usize,
) -> Result<Vec<SocketAddr>> {
    if expected_port == 0 || max_answers == 0 {
        bail!("outbound DNS policy rejected");
    }
    let mut unique = BTreeSet::new();
    let mut answer_count = 0_usize;
    for address in addresses {
        answer_count = answer_count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("outbound DNS answer count overflow"))?;
        if answer_count > max_answers
            || address.port() != expected_port
            || !is_public_unicast(address.ip())
        {
            bail!("outbound DNS policy rejected");
        }
        unique.insert(address);
    }
    if unique.is_empty() {
        bail!("outbound DNS policy rejected");
    }
    let mut ordered = unique.into_iter().collect::<Vec<_>>();
    ordered.sort_by_key(|address| (address.is_ipv6(), address.ip(), address.port()));
    Ok(ordered)
}

pub(crate) fn is_public_unicast(ip: IpAddr) -> bool {
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
        || (segments[0] == 0x0064
            && segments[1] == 0xff9b
            && (segments[2..6] == [0, 0, 0, 0] || segments[2] == 0x0001))
        || (segments[0] == 0x0100 && segments[1..4] == [0, 0, 0])
        || segments[..4] == [0x0100, 0, 0, 1]
        || (segments[0] == 0x2001
            && segments[1] <= 0x01ff
            && !is_allowed_ietf_protocol_assignment(segments))
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || segments[0] == 0x2002
        || (segments[0] == 0x3fff && (segments[1] & 0xf000) == 0)
        || segments[0] == 0x5f00
    {
        return false;
    }
    segments[..6] != [0, 0, 0, 0, 0, 0]
}

fn is_allowed_ietf_protocol_assignment(segments: [u16; 8]) -> bool {
    let exact_anycast =
        segments[1] == 0x0001 && segments[2..7] == [0, 0, 0, 0, 0] && matches!(segments[7], 1..=3);
    exact_anycast
        || segments[1] == 0x0003
        || (segments[1] == 0x0004 && segments[2] == 0x0112)
        || (segments[1] & 0xfff0) == 0x0030
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_unicast_policy_rejects_special_ranges() {
        for address in [
            "1.1.1.1",
            "8.8.8.8",
            "2001:1::1",
            "2001:1::2",
            "2001:1::3",
            "2001:3::1",
            "2001:4:112::1",
            "2001:30::1",
            "2606:4700:4700::1111",
        ] {
            let address = address.parse::<IpAddr>().unwrap();
            assert!(is_public_unicast(address), "expected public: {address}");
        }

        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "172.16.0.1",
            "192.0.0.1",
            "192.0.2.1",
            "192.88.99.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "::ffff:192.0.2.1",
            "fc00::1",
            "fe80::1",
            "fec0::1",
            "64:ff9b::1",
            "64:ff9b:1::1",
            "100::1",
            "100:0:0:1::1",
            "2001::1",
            "2001:2::1",
            "2001:1::4",
            "2001:1:1::1",
            "2001:5::1",
            "2001:10::1",
            "2001:20::1",
            "2001:db8::1",
            "2002::1",
            "3fff::1",
            "5f00::1",
            "ff02::1",
        ] {
            let address = address.parse::<IpAddr>().unwrap();
            assert!(!is_public_unicast(address), "expected rejection: {address}");
        }
    }

    #[test]
    fn dns_answers_fail_closed_and_are_deterministically_deduplicated() {
        let public_v4 = SocketAddr::from(([8, 8, 8, 8], 443));
        let public_v6 = SocketAddr::new("2606:4700:4700::1111".parse().unwrap(), 443);
        let ordered =
            validate_and_order_public_addresses([public_v6, public_v4, public_v4], 443, 4).unwrap();
        assert_eq!(ordered, vec![public_v4, public_v6]);
        assert!(validate_and_order_public_addresses(
            [public_v4, SocketAddr::from(([127, 0, 0, 1], 443))],
            443,
            4,
        )
        .is_err());
        assert!(validate_and_order_public_addresses([public_v4], 8443, 4).is_err());
        assert!(validate_and_order_public_addresses([public_v4, public_v4], 443, 1).is_err());
        assert!(validate_and_order_public_addresses([], 443, 4).is_err());
    }
}
