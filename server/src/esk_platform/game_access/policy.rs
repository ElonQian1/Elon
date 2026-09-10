use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::model::{Error, Result, CLIENT};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyInput {
    pub schema: String,
    pub main_issuer: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub service_secret_sha256: String,
    pub key_id: String,
    pub signing_seed_hex: String,
}

/// Constructed from operator configuration, never a browser or token request.
pub struct Policy {
    pub(super) issuer: String,
    pub(super) redirect: String,
    pub(super) service_hash: String,
    pub(super) key_id: String,
    pub(super) signing_key: SigningKey,
    pub(super) digest: String,
}
impl Policy {
    pub fn from_input(input: PolicyInput) -> Result<Self> {
        if input.schema != "esk.game.access.policy.v1"
            || input.client_id != CLIENT
            || !identifier(&input.main_issuer)
            || !identifier(&input.key_id)
            || !lower_hex(&input.service_secret_sha256, 32)
            || !lower_hex(&input.signing_seed_hex, 32)
        {
            return Err(Error::InvalidInput.into());
        }
        let url = reqwest::Url::parse(&input.redirect_uri).map_err(|_| Error::InvalidInput)?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/api/account/callback"
            || url.as_str() != input.redirect_uri
        {
            return Err(Error::InvalidInput.into());
        }
        let seed: [u8; 32] = hex::decode(&input.signing_seed_hex)?
            .try_into()
            .map_err(|_| Error::InvalidInput)?;
        let signing_key = SigningKey::from_bytes(&seed);
        let digest = hash(&serde_json::to_string(&[
            "esk.game.access.policy.v1",
            &input.main_issuer,
            CLIENT,
            &input.redirect_uri,
            &input.service_secret_sha256,
            &input.key_id,
            &hex::encode(signing_key.verifying_key().as_bytes()),
        ])?);
        Ok(Self {
            issuer: input.main_issuer,
            redirect: input.redirect_uri,
            service_hash: input.service_secret_sha256,
            key_id: input.key_id,
            signing_key,
            digest,
        })
    }
    pub fn check_service(&self, secret: &str) -> Result<()> {
        if !secret_shape(secret, "egs_") || !equal(&hash(secret), &self.service_hash) {
            return Err(Error::Unauthorized.into());
        }
        Ok(())
    }
    pub fn public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }
}

pub fn load() -> Result<Policy> {
    #[cfg(test)]
    if let Some(value) = super::test_support::current_policy() {
        return value;
    }
    use std::io::Read;
    let path = std::env::var_os("ELON_GAME_ACCESS_CONFIG").ok_or(Error::Disabled)?;
    let file = std::fs::File::open(path).map_err(|_| Error::Unavailable)?;
    let mut bytes = Vec::new();
    file.take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() > 16_384 {
        return Err(Error::Unavailable.into());
    }
    let input = serde_json::from_slice(&bytes).map_err(|_| Error::Unavailable)?;
    Policy::from_input(input).map_err(|_| Error::Unavailable.into())
}
pub fn hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
pub fn equal(a: &str, b: &str) -> bool {
    ring::constant_time::verify_slices_are_equal(a.as_bytes(), b.as_bytes()).is_ok()
}
pub fn identifier(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}
pub fn lower_hex(s: &str, bytes: usize) -> bool {
    s.len() == bytes * 2
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn secret_shape(s: &str, prefix: &str) -> bool {
    s.strip_prefix(prefix)
        .is_some_and(|rest| lower_hex(rest, 32))
}
pub fn unreserved(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
}
pub fn pkce(verifier: &str) -> Result<String> {
    if !unreserved(verifier, 43, 128) {
        return Err(Error::InvalidInput.into());
    }
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())))
}
pub fn valid_challenge(value: &str) -> bool {
    URL_SAFE_NO_PAD
        .decode(value)
        .is_ok_and(|bytes| bytes.len() == 32 && URL_SAFE_NO_PAD.encode(bytes) == value)
}
pub fn valid_scopes(scopes: &[String]) -> bool {
    let order = ["play", "inventory_read", "redeem", "principal_withdraw"];
    let mut previous = None;
    !scopes.is_empty()
        && scopes.len() <= 4
        && scopes[0] == "play"
        && scopes.iter().all(|scope| {
            let Some(index) = order.iter().position(|s| s == scope) else {
                return false;
            };
            let valid = previous.is_none_or(|p| index > p);
            previous = Some(index);
            valid
        })
}
