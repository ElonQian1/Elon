use std::net::SocketAddr;

use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use ring::constant_time::verify_slices_are_equal;
use sha2::Sha256;

pub(super) type HmacSha256 = Hmac<Sha256>;

pub(super) fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
}

pub(super) fn constant_time_equal(left: &str, right: &str) -> bool {
    verify_slices_are_equal(left.as_bytes(), right.as_bytes()).is_ok()
}

pub(super) fn keyed_commitment(
    key: &[u8],
    domain: &[u8],
    epoch: &[u8],
    update: impl FnOnce(&mut HmacSha256),
) -> Result<String> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key)
        .map_err(|_| anyhow!("Provider readiness HMAC key length was rejected"))?;
    update_field(&mut mac, domain);
    update_field(&mut mac, epoch);
    update(&mut mac);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

pub(super) fn update_field(mac: &mut HmacSha256, value: &[u8]) {
    mac.update(&(value.len() as u64).to_be_bytes());
    mac.update(value);
}

pub(super) fn update_u64(mac: &mut HmacSha256, value: u64) {
    mac.update(&value.to_be_bytes());
}

pub(super) fn update_socket_address(mac: &mut HmacSha256, address: SocketAddr) {
    match address.ip() {
        std::net::IpAddr::V4(ip) => {
            mac.update(&[4]);
            mac.update(&ip.octets());
        }
        std::net::IpAddr::V6(ip) => {
            mac.update(&[6]);
            mac.update(&ip.octets());
        }
    }
    mac.update(&address.port().to_be_bytes());
}
