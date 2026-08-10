use super::*;
use crate::open_commerce_consumer_vault_model::{ConsumerDataVaultCipher, ConsumerDataVaultKdf};

fn valid_envelope() -> ConsumerDataVaultEnvelope {
    ConsumerDataVaultEnvelope {
        schema: CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA.to_string(),
        record_id: "record_123456".to_string(),
        revision: 1,
        kdf: ConsumerDataVaultKdf {
            name: "PBKDF2".to_string(),
            hash: "SHA-256".to_string(),
            iterations: VAULT_KDF_ITERATIONS,
            salt_base64: BASE64.encode([7_u8; 16]),
        },
        cipher: ConsumerDataVaultCipher {
            name: "AES-256-GCM".to_string(),
            nonce_base64: BASE64.encode([9_u8; 12]),
            auth_tag_length_bits: 128,
        },
        ciphertext_base64: BASE64.encode([11_u8; 17]),
        created_at: "2026-08-10T10:30:00Z".to_string(),
    }
}

fn validation_error(envelope: &ConsumerDataVaultEnvelope) -> String {
    validate_envelope(envelope, "record_123456", 1)
        .expect_err("envelope should be rejected")
        .to_string()
}

#[test]
fn accepts_the_fixed_v1_envelope_contract() {
    assert_eq!(
        validate_envelope(&valid_envelope(), "record_123456", 1).unwrap(),
        vec![11_u8; 17]
    );
}

#[test]
fn accepts_the_exact_ciphertext_limit() {
    let mut envelope = valid_envelope();
    envelope.ciphertext_base64 = BASE64.encode(vec![11_u8; MAX_CIPHERTEXT_BYTES]);
    assert_eq!(
        validate_envelope(&envelope, "record_123456", 1)
            .unwrap()
            .len(),
        MAX_CIPHERTEXT_BYTES
    );
}

#[test]
fn rejects_noncanonical_or_oversized_ciphertext() {
    let mut malformed = valid_envelope();
    malformed.kdf.salt_base64.replace_range(0..1, " ");
    assert!(validation_error(&malformed).contains("不是规范 Base64"));

    let mut overpadded = valid_envelope();
    overpadded.cipher.nonce_base64.push('=');
    assert!(validation_error(&overpadded).contains("必须为 12 字节"));

    let mut too_small = valid_envelope();
    too_small.ciphertext_base64 = BASE64.encode([1_u8; 16]);
    assert!(validation_error(&too_small).contains("17 字节到 1 MiB"));

    let mut too_large = valid_envelope();
    too_large.ciphertext_base64 = BASE64.encode(vec![1_u8; MAX_CIPHERTEXT_BYTES + 1]);
    assert!(validation_error(&too_large).contains("17 字节到 1 MiB"));
}

#[test]
fn rejects_identity_crypto_parameter_and_time_substitution() {
    let mut wrong_id = valid_envelope();
    wrong_id.record_id = "record_654321".to_string();
    assert!(validation_error(&wrong_id).contains("记录 ID"));

    let mut wrong_revision = valid_envelope();
    wrong_revision.revision = 2;
    assert!(validation_error(&wrong_revision).contains("修订号"));

    let mut wrong_iterations = valid_envelope();
    wrong_iterations.kdf.iterations -= 1;
    assert!(validation_error(&wrong_iterations).contains("加密参数"));

    let mut wrong_time = valid_envelope();
    wrong_time.created_at = "not-rfc3339".to_string();
    assert!(validation_error(&wrong_time).contains("RFC3339"));
}
