use super::model::*;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyInput {
    pub schema: String,
    pub main_issuer: String,
    pub network: String,
    pub chain_identifier: String,
    pub genesis_checkpoint_digest: String,
    pub rewards_package_id: String,
    pub registry_id: String,
    pub asset_type: String,
    pub asset_decimals: u8,
    pub reconciler_public_key_hex: String,
    pub funding_observer_public_key_hex: String,
}
pub struct Policy {
    pub input: PolicyInput,
    pub digest: String,
    pub(super) reconciler: VerifyingKey,
    pub(super) observer: VerifyingKey,
}
pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub fn hex_value(value: &str, bytes: usize) -> bool {
    value.len() == bytes * 2
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-:".contains(&b))
}
pub fn address(value: &str) -> bool {
    value.starts_with("0x") && hex_value(&value[2..], 32) && value[2..].bytes().any(|b| b != b'0')
}
pub fn chain_digest(value: &str) -> bool {
    // Canonical base58 encoding is checked by the pinned chain observer as well.
    (32..=44).contains(&value.len())
        && value
            .bytes()
            .all(|b| b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(&b))
}
pub fn integer(value: &str) -> Result<i64> {
    let parsed = value.parse::<i64>().map_err(|_| Error::Invalid)?;
    if parsed.to_string() != value {
        return Err(Error::Invalid.into());
    }
    Ok(parsed)
}
pub fn natural(value: &str) -> Result<i64> {
    let n = integer(value)?;
    if n < 0 {
        return Err(Error::Invalid.into());
    }
    Ok(n)
}
pub fn canonical<T: Serialize>(purpose: &str, payload: &T) -> Result<Vec<u8>> {
    let mut bytes = format!("ESK_GAME_REWARDS_V1\n{purpose}\n").into_bytes();
    bytes.extend(serde_json::to_vec(payload)?);
    Ok(bytes)
}
pub fn verify<T: Serialize>(
    key: &VerifyingKey,
    purpose: &str,
    proof: &Signed<T>,
) -> Result<String> {
    if !hex_value(&proof.signature_hex, 64) {
        return Err(Error::Invalid.into());
    }
    let bytes = canonical(purpose, &proof.payload)?;
    let sig =
        Signature::from_slice(&hex::decode(&proof.signature_hex)?).map_err(|_| Error::Invalid)?;
    key.verify_strict(&bytes, &sig)
        .map_err(|_| Error::Unauthorized)?;
    Ok(hash(bytes))
}
impl Policy {
    pub fn from_input(input: PolicyInput) -> Result<Self> {
        let parts: Vec<_> = input.asset_type.split("::").collect();
        if input.schema != "esk.game.rewards.policy.v1"
            || !id(&input.main_issuer)
            || input.network != "testnet"
            || !hex_value(&input.chain_identifier, 4)
            || !chain_digest(&input.genesis_checkpoint_digest)
            || !address(&input.rewards_package_id)
            || !address(&input.registry_id)
            || input.asset_decimals > 18
            || parts.len() != 3
            || !address(parts[0])
            || !parts[1..].iter().all(|v| {
                !v.is_empty()
                    && v.len() <= 64
                    && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
            || !hex_value(&input.reconciler_public_key_hex, 32)
            || !hex_value(&input.funding_observer_public_key_hex, 32)
            || input.reconciler_public_key_hex == input.funding_observer_public_key_hex
        {
            return Err(Error::Invalid.into());
        }
        let key = |s: &str| -> Result<VerifyingKey> {
            let bytes: [u8; 32] = hex::decode(s)?.try_into().map_err(|_| Error::Invalid)?;
            let key = VerifyingKey::from_bytes(&bytes).map_err(|_| Error::Invalid)?;
            if key.is_weak() {
                return Err(Error::Invalid.into());
            }
            Ok(key)
        };
        let digest = hash(canonical("policy", &input)?);
        Ok(Self {
            reconciler: key(&input.reconciler_public_key_hex)?,
            observer: key(&input.funding_observer_public_key_hex)?,
            input,
            digest,
        })
    }
}
pub fn load() -> Result<Policy> {
    #[cfg(test)]
    if let Some(input) = super::tests::override_policy() {
        return Policy::from_input(input);
    }
    let path = std::env::var_os("ELON_GAME_REWARDS_CONFIG").ok_or(Error::Disabled)?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(16385)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16384 {
        return Err(Error::Invalid.into());
    }
    Policy::from_input(serde_json::from_slice(&bytes)?)
}
