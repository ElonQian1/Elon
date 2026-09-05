use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use blake2::{digest::VariableOutput, Blake2bVar};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature as Ed25519Signature, VerifyingKey as Ed25519VerifyingKey};
use k256::ecdsa::{
    signature::Verifier as _, Signature as Secp256k1Signature,
    VerifyingKey as Secp256k1VerifyingKey,
};
use p256::ecdsa::{Signature as Secp256r1Signature, VerifyingKey as Secp256r1VerifyingKey};

use super::{
    challenge::{
        canonical_timestamp, decode_canonical_base64, parse_timestamp, reconstruct_challenge,
        sha256_prefixed,
    },
    model::{
        AddressBindingChallenge, AddressBindingError, SignatureScheme, VerifiedWalletResponse,
        WalletResponseBody, MAX_MESSAGE_BYTES, MAX_SIGNATURE_BYTES, WALLET_RESPONSE_SCHEMA,
    },
};

const PERSONAL_MESSAGE_INTENT: [u8; 3] = [3, 0, 0];
const ED25519_SERIALIZED_LENGTH: usize = 1 + 64 + 32;
const ECDSA_SERIALIZED_LENGTH: usize = 1 + 64 + 33;

pub(crate) fn subject_commitment(seed: &[u8; 32]) -> String {
    sha256_prefixed(seed)
}

pub(crate) fn verify_wallet_response(
    challenge: &AddressBindingChallenge,
    response: &WalletResponseBody,
    now: DateTime<Utc>,
) -> Result<VerifiedWalletResponse, AddressBindingError> {
    let challenge = reconstruct_challenge(challenge)?;
    validate_wallet_response(response)?;
    if response.challenge_id != challenge.challenge_id {
        return Err(AddressBindingError::ChallengeIdMismatch);
    }

    let expected_message = decode_canonical_base64(&challenge.message_base64, 1, MAX_MESSAGE_BYTES)
        .map_err(|_| AddressBindingError::InvalidChallenge)?;
    let actual_message = decode_canonical_base64(&response.message_base64, 1, MAX_MESSAGE_BYTES)
        .map_err(|_| AddressBindingError::InvalidResponse)?;
    if expected_message != actual_message {
        return Err(AddressBindingError::MessageMismatch);
    }

    let issued_at =
        parse_timestamp(&challenge.issued_at).map_err(|_| AddressBindingError::InvalidChallenge)?;
    let expires_at = parse_timestamp(&challenge.expires_at)
        .map_err(|_| AddressBindingError::InvalidChallenge)?;
    if now < issued_at {
        return Err(AddressBindingError::NotYetValid);
    }
    if now >= expires_at {
        return Err(AddressBindingError::Expired);
    }

    let serialized_signature = decode_canonical_base64(&response.signature, 2, MAX_SIGNATURE_BYTES)
        .map_err(|_| AddressBindingError::InvalidResponse)?;
    let (scheme, signature, public_key) = split_serialized_signature(&serialized_signature)?;
    let digest = personal_message_digest(&expected_message);
    verify_signature(scheme, signature, public_key, &digest)?;
    if derive_sui_address(scheme, public_key) != challenge.address {
        return Err(AddressBindingError::SignatureInvalid);
    }

    let wallet_response_json =
        serde_json::to_string(response).map_err(|_| AddressBindingError::InvalidResponse)?;
    Ok(VerifiedWalletResponse {
        challenge_id: challenge.challenge_id,
        address: challenge.address,
        subject_commitment: challenge.subject_commitment,
        message_sha256: sha256_prefixed(&expected_message),
        signature_scheme: scheme,
        signature_sha256: sha256_prefixed(&serialized_signature),
        response_digest: sha256_prefixed(wallet_response_json.as_bytes()),
        verified_at: canonical_timestamp(now),
        wallet_response_json,
    })
}

pub(crate) fn personal_message_digest(message: &[u8]) -> [u8; 32] {
    let mut intent_message = Vec::with_capacity(message.len() + 6);
    intent_message.extend_from_slice(&PERSONAL_MESSAGE_INTENT);
    encode_uleb128(message.len(), &mut intent_message);
    intent_message.extend_from_slice(message);
    blake2b_256(&intent_message)
}

pub(crate) fn derive_sui_address(scheme: SignatureScheme, public_key: &[u8]) -> String {
    let mut address_material = Vec::with_capacity(public_key.len() + 1);
    address_material.push(scheme.flag());
    address_material.extend_from_slice(public_key);
    format!("0x{}", hex::encode(blake2b_256(&address_material)))
}

pub(crate) fn validate_wallet_response(
    response: &WalletResponseBody,
) -> Result<(), AddressBindingError> {
    if response.schema != WALLET_RESPONSE_SCHEMA || !valid_challenge_id(&response.challenge_id) {
        return Err(AddressBindingError::InvalidResponse);
    }
    decode_canonical_base64(&response.message_base64, 1, MAX_MESSAGE_BYTES)
        .map_err(|_| AddressBindingError::InvalidResponse)?;
    decode_canonical_base64(&response.signature, 2, MAX_SIGNATURE_BYTES)
        .map_err(|_| AddressBindingError::InvalidResponse)?;
    Ok(())
}

fn split_serialized_signature(
    bytes: &[u8],
) -> Result<(SignatureScheme, &[u8], &[u8]), AddressBindingError> {
    let (scheme, expected_length, public_key_offset) = match bytes.first().copied() {
        Some(0) => (SignatureScheme::Ed25519, ED25519_SERIALIZED_LENGTH, 65),
        Some(1) => (SignatureScheme::Secp256k1, ECDSA_SERIALIZED_LENGTH, 65),
        Some(2) => (SignatureScheme::Secp256r1, ECDSA_SERIALIZED_LENGTH, 65),
        _ => return Err(AddressBindingError::UnsupportedSignatureScheme),
    };
    if bytes.len() != expected_length {
        return Err(AddressBindingError::SignatureInvalid);
    }
    Ok((
        scheme,
        &bytes[1..public_key_offset],
        &bytes[public_key_offset..],
    ))
}

fn verify_signature(
    scheme: SignatureScheme,
    signature: &[u8],
    public_key: &[u8],
    digest: &[u8; 32],
) -> Result<(), AddressBindingError> {
    match scheme {
        SignatureScheme::Ed25519 => {
            let key_bytes: &[u8; 32] = public_key
                .try_into()
                .map_err(|_| AddressBindingError::SignatureInvalid)?;
            let key = Ed25519VerifyingKey::from_bytes(key_bytes)
                .map_err(|_| AddressBindingError::SignatureInvalid)?;
            let signature = Ed25519Signature::from_slice(signature)
                .map_err(|_| AddressBindingError::SignatureInvalid)?;
            key.verify_strict(digest, &signature)
                .map_err(|_| AddressBindingError::SignatureInvalid)
        }
        SignatureScheme::Secp256k1 => {
            let key = Secp256k1VerifyingKey::from_sec1_bytes(public_key)
                .map_err(|_| AddressBindingError::SignatureInvalid)?;
            let signature = Secp256k1Signature::from_slice(signature)
                .map_err(|_| AddressBindingError::SignatureInvalid)?;
            if signature.normalize_s().is_some() {
                return Err(AddressBindingError::SignatureInvalid);
            }
            // Match @mysten/sui 2.29.0 with @noble/curves 2.4.0: noble ECDSA
            // hashes the 32-byte Blake2b intent digest once more with SHA-256.
            key.verify(digest, &signature)
                .map_err(|_| AddressBindingError::SignatureInvalid)
        }
        SignatureScheme::Secp256r1 => {
            let key = Secp256r1VerifyingKey::from_sec1_bytes(public_key)
                .map_err(|_| AddressBindingError::SignatureInvalid)?;
            let signature = Secp256r1Signature::from_slice(signature)
                .map_err(|_| AddressBindingError::SignatureInvalid)?;
            if signature.normalize_s().is_some() {
                return Err(AddressBindingError::SignatureInvalid);
            }
            key.verify(digest, &signature)
                .map_err(|_| AddressBindingError::SignatureInvalid)
        }
    }
}

fn blake2b_256(bytes: &[u8]) -> [u8; 32] {
    let mut digest = [0_u8; 32];
    let mut hasher = Blake2bVar::new(digest.len()).expect("32 is a valid Blake2b output length");
    blake2::digest::Update::update(&mut hasher, bytes);
    hasher
        .finalize_variable(&mut digest)
        .expect("the fixed output buffer has the configured length");
    digest
}

fn encode_uleb128(mut value: usize, output: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            return;
        }
    }
}

pub(crate) fn valid_challenge_id(value: &str) -> bool {
    value.strip_prefix("eab1_").is_some_and(|body| {
        body.len() == 32
            && body
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::esk_asset::platform::sui_address_binding::{assemble_challenge, ChallengeMaterial};
    use ed25519_dalek::{Signer as _, SigningKey as Ed25519SigningKey};
    use k256::ecdsa::SigningKey as Secp256k1SigningKey;
    use p256::ecdsa::SigningKey as Secp256r1SigningKey;

    fn material(address: String) -> ChallengeMaterial {
        ChallengeMaterial {
            address,
            ttl_seconds: 600,
            nonce_base64: BASE64.encode([7_u8; 32]),
            issued_at: "2026-09-05T08:00:00.000Z".to_owned(),
            expires_at: "2026-09-05T08:10:00.000Z".to_owned(),
        }
    }

    fn response(challenge: &AddressBindingChallenge, signature: Vec<u8>) -> WalletResponseBody {
        WalletResponseBody {
            schema: WALLET_RESPONSE_SCHEMA.to_owned(),
            challenge_id: challenge.challenge_id.clone(),
            message_base64: challenge.message_base64.clone(),
            signature: BASE64.encode(signature),
        }
    }

    fn high_s_signature<const N: usize>(signature: &[u8; N], order: [u8; 32]) -> [u8; N] {
        let mut result = *signature;
        let mut borrow = 0_u16;
        for index in (0..32).rev() {
            let minuend = u16::from(order[index]);
            let subtrahend = u16::from(signature[32 + index]) + borrow;
            if minuend >= subtrahend {
                result[32 + index] = (minuend - subtrahend) as u8;
                borrow = 0;
            } else {
                result[32 + index] = (minuend + 256 - subtrahend) as u8;
                borrow = 1;
            }
        }
        assert_eq!(borrow, 0, "valid scalar must be below the curve order");
        result
    }

    #[test]
    fn verifies_all_supported_sui_single_signature_schemes() {
        let now = parse_timestamp("2026-09-05T08:05:00.000Z").expect("fixed time");
        let subject = subject_commitment(&[9_u8; 32]);

        let ed25519 = Ed25519SigningKey::from_bytes(&[1_u8; 32]);
        let ed_public = ed25519.verifying_key().to_bytes();
        let ed_address = derive_sui_address(SignatureScheme::Ed25519, &ed_public);
        let ed_challenge = assemble_challenge(&subject, &material(ed_address)).expect("challenge");
        let ed_digest = personal_message_digest(
            &BASE64
                .decode(&ed_challenge.message_base64)
                .expect("message"),
        );
        let mut ed_serialized = vec![SignatureScheme::Ed25519.flag()];
        ed_serialized.extend_from_slice(&ed25519.sign(&ed_digest).to_bytes());
        ed_serialized.extend_from_slice(&ed_public);
        let verified =
            verify_wallet_response(&ed_challenge, &response(&ed_challenge, ed_serialized), now)
                .expect("ed25519 response");
        assert_eq!(verified.signature_scheme, SignatureScheme::Ed25519);

        let k1 = Secp256k1SigningKey::from_slice(&[2_u8; 32]).expect("fixed key");
        let k1_public = k1.verifying_key().to_encoded_point(true);
        let k1_address = derive_sui_address(SignatureScheme::Secp256k1, k1_public.as_bytes());
        let k1_challenge = assemble_challenge(&subject, &material(k1_address)).expect("challenge");
        let k1_digest = personal_message_digest(
            &BASE64
                .decode(&k1_challenge.message_base64)
                .expect("message"),
        );
        let k1_signature: Secp256k1Signature = k1.sign(&k1_digest);
        let mut k1_serialized = vec![SignatureScheme::Secp256k1.flag()];
        k1_serialized.extend_from_slice(&k1_signature.to_bytes());
        k1_serialized.extend_from_slice(k1_public.as_bytes());
        let verified =
            verify_wallet_response(&k1_challenge, &response(&k1_challenge, k1_serialized), now)
                .expect("secp256k1 response");
        assert_eq!(verified.signature_scheme, SignatureScheme::Secp256k1);

        let r1 = Secp256r1SigningKey::from_slice(&[3_u8; 32]).expect("fixed key");
        let r1_public = r1.verifying_key().to_encoded_point(true);
        let r1_address = derive_sui_address(SignatureScheme::Secp256r1, r1_public.as_bytes());
        let r1_challenge = assemble_challenge(&subject, &material(r1_address)).expect("challenge");
        let r1_digest = personal_message_digest(
            &BASE64
                .decode(&r1_challenge.message_base64)
                .expect("message"),
        );
        let r1_signature: Secp256r1Signature = r1.sign(&r1_digest);
        let r1_signature = r1_signature.normalize_s().unwrap_or(r1_signature);
        let mut r1_serialized = vec![SignatureScheme::Secp256r1.flag()];
        r1_serialized.extend_from_slice(&r1_signature.to_bytes());
        r1_serialized.extend_from_slice(r1_public.as_bytes());
        let verified =
            verify_wallet_response(&r1_challenge, &response(&r1_challenge, r1_serialized), now)
                .expect("secp256r1 response");
        assert_eq!(verified.signature_scheme, SignatureScheme::Secp256r1);
    }

    #[test]
    fn rejects_unknown_scheme_noncanonical_base64_and_wrong_message() {
        let subject = subject_commitment(&[9_u8; 32]);
        let address = format!("0x{}", "b".repeat(64));
        let challenge = assemble_challenge(&subject, &material(address)).expect("challenge");
        let now = parse_timestamp("2026-09-05T08:05:00.000Z").expect("fixed time");

        let unknown = response(&challenge, vec![3, 0]);
        assert_eq!(
            verify_wallet_response(&challenge, &unknown, now),
            Err(AddressBindingError::UnsupportedSignatureScheme)
        );

        let mut noncanonical = unknown.clone();
        noncanonical.signature = "AwA".to_owned();
        assert_eq!(
            verify_wallet_response(&challenge, &noncanonical, now),
            Err(AddressBindingError::InvalidResponse)
        );

        let mut wrong_message = unknown;
        wrong_message.message_base64 = BASE64.encode(b"wrong");
        assert_eq!(
            verify_wallet_response(&challenge, &wrong_message, now),
            Err(AddressBindingError::MessageMismatch)
        );
    }

    #[test]
    fn rejects_wrong_signature_length_public_key_and_signature_bytes() {
        let now = parse_timestamp("2026-09-05T08:05:00.000Z").expect("fixed time");
        let subject = subject_commitment(&[9_u8; 32]);
        let ed25519 = Ed25519SigningKey::from_bytes(&[1_u8; 32]);
        let ed_public = ed25519.verifying_key().to_bytes();
        let challenge = assemble_challenge(
            &subject,
            &material(derive_sui_address(SignatureScheme::Ed25519, &ed_public)),
        )
        .expect("challenge");

        let mut wrong_length = vec![SignatureScheme::Ed25519.flag()];
        wrong_length.extend_from_slice(&[0_u8; 64]);
        wrong_length.extend_from_slice(&ed_public[..31]);
        assert_eq!(
            verify_wallet_response(&challenge, &response(&challenge, wrong_length), now),
            Err(AddressBindingError::SignatureInvalid)
        );

        let mut invalid_public_key = vec![SignatureScheme::Secp256k1.flag()];
        invalid_public_key.extend_from_slice(&[0_u8; 64]);
        invalid_public_key.extend_from_slice(&[0_u8; 33]);
        assert_eq!(
            verify_wallet_response(&challenge, &response(&challenge, invalid_public_key), now,),
            Err(AddressBindingError::SignatureInvalid)
        );

        let mut invalid_signature = vec![SignatureScheme::Ed25519.flag()];
        invalid_signature.extend_from_slice(&[0_u8; 64]);
        invalid_signature.extend_from_slice(&ed_public);
        assert_eq!(
            verify_wallet_response(&challenge, &response(&challenge, invalid_signature), now),
            Err(AddressBindingError::SignatureInvalid)
        );
    }

    #[test]
    fn rejects_high_s_ecdsa_and_wrong_derived_address() {
        const K1_ORDER: [u8; 32] = [
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xfe, 0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c,
            0xd0, 0x36, 0x41, 0x41,
        ];
        const R1_ORDER: [u8; 32] = [
            0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xbc, 0xe6, 0xfa, 0xad, 0xa7, 0x17, 0x9e, 0x84, 0xf3, 0xb9, 0xca, 0xc2,
            0xfc, 0x63, 0x25, 0x51,
        ];
        let now = parse_timestamp("2026-09-05T08:05:00.000Z").expect("fixed time");
        let subject = subject_commitment(&[9_u8; 32]);

        let k1 = Secp256k1SigningKey::from_slice(&[2_u8; 32]).expect("fixed key");
        let k1_public = k1.verifying_key().to_encoded_point(true);
        let k1_challenge = assemble_challenge(
            &subject,
            &material(derive_sui_address(
                SignatureScheme::Secp256k1,
                k1_public.as_bytes(),
            )),
        )
        .expect("challenge");
        let k1_digest = personal_message_digest(
            &BASE64
                .decode(&k1_challenge.message_base64)
                .expect("message"),
        );
        let k1_signature: Secp256k1Signature = k1.sign(&k1_digest);
        let k1_signature_bytes: [u8; 64] = k1_signature.to_bytes().into();
        let high_k1 = high_s_signature(&k1_signature_bytes, K1_ORDER);
        let mut serialized = vec![SignatureScheme::Secp256k1.flag()];
        serialized.extend_from_slice(&high_k1);
        serialized.extend_from_slice(k1_public.as_bytes());
        assert_eq!(
            verify_wallet_response(&k1_challenge, &response(&k1_challenge, serialized), now),
            Err(AddressBindingError::SignatureInvalid)
        );

        let r1 = Secp256r1SigningKey::from_slice(&[3_u8; 32]).expect("fixed key");
        let r1_public = r1.verifying_key().to_encoded_point(true);
        let r1_challenge = assemble_challenge(
            &subject,
            &material(derive_sui_address(
                SignatureScheme::Secp256r1,
                r1_public.as_bytes(),
            )),
        )
        .expect("challenge");
        let r1_digest = personal_message_digest(
            &BASE64
                .decode(&r1_challenge.message_base64)
                .expect("message"),
        );
        let r1_signature: Secp256r1Signature = r1.sign(&r1_digest);
        let r1_signature = r1_signature.normalize_s().unwrap_or(r1_signature);
        let r1_signature_bytes: [u8; 64] = r1_signature.to_bytes().into();
        let high_r1 = high_s_signature(&r1_signature_bytes, R1_ORDER);
        let mut serialized = vec![SignatureScheme::Secp256r1.flag()];
        serialized.extend_from_slice(&high_r1);
        serialized.extend_from_slice(r1_public.as_bytes());
        assert_eq!(
            verify_wallet_response(&r1_challenge, &response(&r1_challenge, serialized), now),
            Err(AddressBindingError::SignatureInvalid)
        );

        let ed25519 = Ed25519SigningKey::from_bytes(&[1_u8; 32]);
        let ed_public = ed25519.verifying_key().to_bytes();
        let wrong_address_challenge =
            assemble_challenge(&subject, &material(format!("0x{}", "b".repeat(64))))
                .expect("challenge");
        let ed_digest = personal_message_digest(
            &BASE64
                .decode(&wrong_address_challenge.message_base64)
                .expect("message"),
        );
        let mut serialized = vec![SignatureScheme::Ed25519.flag()];
        serialized.extend_from_slice(&ed25519.sign(&ed_digest).to_bytes());
        serialized.extend_from_slice(&ed_public);
        assert_eq!(
            verify_wallet_response(
                &wrong_address_challenge,
                &response(&wrong_address_challenge, serialized),
                now,
            ),
            Err(AddressBindingError::SignatureInvalid)
        );
    }
}
