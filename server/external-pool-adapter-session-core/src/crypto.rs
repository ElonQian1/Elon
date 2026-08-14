use anyhow::{anyhow, bail, Result};
use hmac::{Hmac, Mac};
use ring::{hkdf, rand::SecureRandom};
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

use super::roots::ExternalPoolAdapterSessionRoots;

type HmacSha256 = Hmac<Sha256>;

const KEY_INFO_DOMAIN: &[u8] = b"elon.external_pool_adapter.supervisor_session.key.v1\0";
const MAC_DOMAIN: &[u8] = b"elon.external_pool_adapter.supervisor_session.mac.v1\0";
pub(super) const HOST_TO_CHILD_DIRECTION: u8 = 1;
pub(super) const CHILD_TO_HOST_DIRECTION: u8 = 2;

struct KeyLength32;

impl hkdf::KeyType for KeyLength32 {
    fn len(&self) -> usize {
        32
    }
}

pub(super) struct Secret32(Zeroizing<[u8; 32]>);

impl Secret32 {
    pub(super) fn new(bytes: [u8; 32]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    pub(super) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub(super) fn zeroize_now(&mut self) {
        self.0.zeroize();
    }
}

pub(super) struct DirectionalKeys {
    host_to_child: Secret32,
    child_to_host: Secret32,
    transcript_digest: [u8; 32],
}

impl DirectionalKeys {
    pub(super) fn into_host(self) -> (Secret32, Secret32, [u8; 32]) {
        (
            self.host_to_child,
            self.child_to_host,
            self.transcript_digest,
        )
    }

    pub(super) fn into_child(self) -> (Secret32, Secret32, [u8; 32]) {
        (
            self.child_to_host,
            self.host_to_child,
            self.transcript_digest,
        )
    }
}

pub(super) fn random_secret32() -> Result<Secret32> {
    Ok(Secret32::new(random_array32()?))
}

pub(super) fn random_array32() -> Result<[u8; 32]> {
    let mut output = [0_u8; 32];
    ring::rand::SystemRandom::new()
        .fill(&mut output)
        .map_err(|_| anyhow!("operating-system CSPRNG failed"))?;
    Ok(output)
}

pub(super) fn derive_directional_keys(
    seed: &Secret32,
    host_nonce: &[u8; 32],
    child_nonce: &[u8; 32],
    roots: &ExternalPoolAdapterSessionRoots,
) -> Result<DirectionalKeys> {
    let salt_bytes = roots.kdf_salt(host_nonce, child_nonce);
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, &salt_bytes);
    let prk = salt.extract(seed.as_bytes());
    let transcript_digest = roots.transcript_digest();
    let host_to_child = expand_directional_key(&prk, HOST_TO_CHILD_DIRECTION, &transcript_digest)?;
    let child_to_host = expand_directional_key(&prk, CHILD_TO_HOST_DIRECTION, &transcript_digest)?;
    Ok(DirectionalKeys {
        host_to_child,
        child_to_host,
        transcript_digest,
    })
}

fn expand_directional_key(
    prk: &hkdf::Prk,
    direction: u8,
    transcript_digest: &[u8; 32],
) -> Result<Secret32> {
    let direction_bytes = [direction];
    let info: [&[u8]; 3] = [
        KEY_INFO_DOMAIN,
        direction_bytes.as_slice(),
        transcript_digest.as_slice(),
    ];
    let okm = prk
        .expand(&info, KeyLength32)
        .map_err(|_| anyhow!("HKDF-SHA256 expand rejected the V260 key context"))?;
    let mut output = [0_u8; 32];
    okm.fill(&mut output)
        .map_err(|_| anyhow!("HKDF-SHA256 failed to fill the V260 key"))?;
    Ok(Secret32::new(output))
}

pub(super) fn mac_tag(
    key: &Secret32,
    direction: u8,
    transcript_digest: &[u8; 32],
    label: &[u8],
    parts: &[&[u8]],
) -> Result<[u8; 32]> {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|_| anyhow!("initialize HMAC-SHA256"))?;
    mac.update(MAC_DOMAIN);
    mac.update(&[direction]);
    mac.update(transcript_digest);
    mac.update(label);
    for part in parts {
        mac.update(part);
    }
    Ok(mac.finalize().into_bytes().into())
}

pub(super) fn verify_mac(
    key: &Secret32,
    direction: u8,
    transcript_digest: &[u8; 32],
    label: &[u8],
    parts: &[&[u8]],
    expected_tag: &[u8],
) -> Result<()> {
    if expected_tag.len() != 32 {
        bail!("authenticated session proof rejected");
    }
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|_| anyhow!("initialize HMAC-SHA256"))?;
    mac.update(MAC_DOMAIN);
    mac.update(&[direction]);
    mac.update(transcript_digest);
    mac.update(label);
    for part in parts {
        mac.update(part);
    }
    mac.verify_slice(expected_tag)
        .map_err(|_| anyhow!("authenticated session proof rejected"))
}
