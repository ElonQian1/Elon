use std::net::{IpAddr, SocketAddr};

use anyhow::Result;

pub(super) fn validate_and_order_dns_answers(
    addresses: impl IntoIterator<Item = SocketAddr>,
    expected_port: u16,
    max_answers: usize,
) -> Result<Vec<SocketAddr>> {
    crate::open_commerce_outbound_security::address_policy::validate_and_order_public_addresses(
        addresses,
        expected_port,
        max_answers,
    )
}

pub(super) fn is_public_unicast(ip: IpAddr) -> bool {
    crate::open_commerce_outbound_security::address_policy::is_public_unicast(ip)
}
