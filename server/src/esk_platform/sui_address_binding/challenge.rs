use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use sha2::{Digest, Sha256};

use super::model::{
    AddressBindingChallenge, AddressBindingError, ChallengeMaterial, PlatformAddressBindingRequest,
    CHALLENGE_SCHEMA, MAX_MESSAGE_BYTES, MAX_TTL_SECONDS, MIN_TTL_SECONDS, NETWORK,
    PLATFORM_REQUEST_SCHEMA, PURPOSE,
};

pub(crate) fn validate_platform_request(
    request: &PlatformAddressBindingRequest,
) -> Result<(), AddressBindingError> {
    if request.schema != PLATFORM_REQUEST_SCHEMA
        || !valid_address(&request.address)
        || !valid_ttl(request.ttl_seconds)
    {
        return Err(AddressBindingError::InvalidInput);
    }
    Ok(())
}

pub(crate) fn assemble_challenge(
    subject_commitment: &str,
    material: &ChallengeMaterial,
) -> Result<AddressBindingChallenge, AddressBindingError> {
    if !valid_subject_commitment(subject_commitment)
        || !valid_address(&material.address)
        || !valid_ttl(material.ttl_seconds)
    {
        return Err(AddressBindingError::InvalidChallenge);
    }
    decode_canonical_base64(&material.nonce_base64, 32, 32)
        .map_err(|_| AddressBindingError::InvalidChallenge)?;
    let issued_at =
        parse_timestamp(&material.issued_at).map_err(|_| AddressBindingError::InvalidChallenge)?;
    let expires_at =
        parse_timestamp(&material.expires_at).map_err(|_| AddressBindingError::InvalidChallenge)?;
    if expires_at
        .signed_duration_since(issued_at)
        .num_milliseconds()
        != i64::from(material.ttl_seconds) * 1_000
    {
        return Err(AddressBindingError::InvalidChallenge);
    }

    let message = message_text(subject_commitment, material).into_bytes();
    if message.is_empty() || message.len() > MAX_MESSAGE_BYTES {
        return Err(AddressBindingError::InvalidChallenge);
    }
    let message_hex = sha256_hex(&message);
    Ok(AddressBindingChallenge {
        schema: CHALLENGE_SCHEMA.to_owned(),
        challenge_id: format!("eab1_{}", &message_hex[..32]),
        network: NETWORK.to_owned(),
        purpose: PURPOSE.to_owned(),
        subject_commitment: subject_commitment.to_owned(),
        address: material.address.clone(),
        ttl_seconds: material.ttl_seconds,
        nonce_base64: material.nonce_base64.clone(),
        issued_at: material.issued_at.clone(),
        expires_at: material.expires_at.clone(),
        message_base64: BASE64.encode(&message),
        message_sha256: format!("sha256:{message_hex}"),
    })
}

pub(crate) fn reconstruct_challenge(
    challenge: &AddressBindingChallenge,
) -> Result<AddressBindingChallenge, AddressBindingError> {
    let rebuilt = assemble_challenge(
        &challenge.subject_commitment,
        &ChallengeMaterial {
            address: challenge.address.clone(),
            ttl_seconds: challenge.ttl_seconds,
            nonce_base64: challenge.nonce_base64.clone(),
            issued_at: challenge.issued_at.clone(),
            expires_at: challenge.expires_at.clone(),
        },
    )
    .map_err(|_| AddressBindingError::InvalidChallenge)?;
    if rebuilt != *challenge {
        return Err(AddressBindingError::InvalidChallenge);
    }
    Ok(rebuilt)
}

pub(crate) fn validate_challenge(
    challenge: &AddressBindingChallenge,
) -> Result<(), AddressBindingError> {
    reconstruct_challenge(challenge).map(|_| ())
}

pub(crate) fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, AddressBindingError> {
    if value.len() != 24 || !value.ends_with('Z') {
        return Err(AddressBindingError::InvalidInput);
    }
    let parsed: DateTime<FixedOffset> =
        DateTime::parse_from_rfc3339(value).map_err(|_| AddressBindingError::InvalidInput)?;
    let utc = parsed.with_timezone(&Utc);
    if utc.to_rfc3339_opts(SecondsFormat::Millis, true) != value {
        return Err(AddressBindingError::InvalidInput);
    }
    Ok(utc)
}

pub(crate) fn canonical_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn decode_canonical_base64(
    value: &str,
    minimum_bytes: usize,
    maximum_bytes: usize,
) -> Result<Vec<u8>, AddressBindingError> {
    if value.is_empty() || value.len() > maximum_bytes.saturating_mul(2) || value.len() % 4 != 0 {
        return Err(AddressBindingError::InvalidInput);
    }
    let bytes = BASE64
        .decode(value)
        .map_err(|_| AddressBindingError::InvalidInput)?;
    if bytes.len() < minimum_bytes || bytes.len() > maximum_bytes || BASE64.encode(&bytes) != value
    {
        return Err(AddressBindingError::InvalidInput);
    }
    Ok(bytes)
}

pub(crate) fn sha256_prefixed(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256_hex(bytes))
}

fn message_text(subject_commitment: &str, material: &ChallengeMaterial) -> String {
    [
        "YILONG_ESK_SUI_ADDRESS_BINDING_V1".to_owned(),
        format!("network={NETWORK}"),
        format!("purpose={PURPOSE}"),
        format!("subject_commitment={subject_commitment}"),
        format!("address={}", material.address),
        format!("nonce_base64={}", material.nonce_base64),
        format!("issued_at={}", material.issued_at),
        format!("expires_at={}", material.expires_at),
    ]
    .join("\n")
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn valid_ttl(value: u32) -> bool {
    (MIN_TTL_SECONDS..=MAX_TTL_SECONDS).contains(&value)
}

fn valid_address(value: &str) -> bool {
    value
        .strip_prefix("0x")
        .is_some_and(|body| lower_hex(body, 64) && body.bytes().any(|byte| byte != b'0'))
}

fn valid_subject_commitment(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|body| lower_hex(body, 64) && body.bytes().any(|byte| byte != b'0'))
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector_material() -> ChallengeMaterial {
        ChallengeMaterial {
            address: format!("0x{}", "b".repeat(64)),
            ttl_seconds: 600,
            nonce_base64: "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=".to_owned(),
            issued_at: "2026-09-05T08:00:00.000Z".to_owned(),
            expires_at: "2026-09-05T08:10:00.000Z".to_owned(),
        }
    }

    #[test]
    fn challenge_matches_v1_cross_implementation_vector() {
        let challenge =
            assemble_challenge(&format!("sha256:{}", "a".repeat(64)), &vector_material())
                .expect("fixed material must assemble");
        assert_eq!(
            challenge.challenge_id,
            "eab1_fcdc075cbdf1d5b7f484161218766d57"
        );
        assert_eq!(
            challenge.message_sha256,
            "sha256:fcdc075cbdf1d5b7f484161218766d57efd0604697e53ebfb0f075e0a4c9d4ce"
        );
        assert_eq!(reconstruct_challenge(&challenge), Ok(challenge));
    }

    #[test]
    fn challenge_rejects_noncanonical_material_and_drift() {
        let mut material = vector_material();
        material.nonce_base64.pop();
        assert_eq!(
            assemble_challenge(&format!("sha256:{}", "a".repeat(64)), &material),
            Err(AddressBindingError::InvalidChallenge)
        );

        let mut challenge =
            assemble_challenge(&format!("sha256:{}", "a".repeat(64)), &vector_material())
                .expect("fixed material must assemble");
        challenge.message_base64.push('A');
        assert_eq!(
            reconstruct_challenge(&challenge),
            Err(AddressBindingError::InvalidChallenge)
        );
    }
}
