use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{Duration, TimeZone, Utc};
use ed25519_dalek::{Signer as _, SigningKey};
use rusqlite::params;
use sha2::{Digest, Sha256};

use super::super::Fixture;
use crate::esk_asset::platform::sui_address_binding::*;

fn key() -> SigningKey {
    SigningKey::from_bytes(&[1_u8; 32])
}

pub(super) fn address() -> String {
    derive_sui_address(SignatureScheme::Ed25519, &key().verifying_key().to_bytes())
}

pub(super) fn material(address: String, nonce: u8) -> ChallengeMaterial {
    material_at(
        address,
        nonce,
        "2026-09-04T09:55:00.000Z",
        "2026-09-04T10:05:00.000Z",
    )
}

pub(super) fn material_at(
    address: String,
    nonce: u8,
    issued_at: &str,
    expires_at: &str,
) -> ChallengeMaterial {
    ChallengeMaterial {
        address,
        ttl_seconds: 600,
        nonce_base64: BASE64.encode([nonce; 32]),
        issued_at: issued_at.into(),
        expires_at: expires_at.into(),
    }
}

pub(super) fn wallet_response(challenge: &AddressBindingChallenge) -> WalletResponseBody {
    let message = BASE64.decode(&challenge.message_base64).unwrap();
    let digest = personal_message_digest(&message);
    let signing_key = key();
    let mut serialized = vec![SignatureScheme::Ed25519.flag()];
    serialized.extend_from_slice(&signing_key.sign(&digest).to_bytes());
    serialized.extend_from_slice(&signing_key.verifying_key().to_bytes());
    WalletResponseBody {
        schema: WALLET_RESPONSE_SCHEMA.into(),
        challenge_id: challenge.challenge_id.clone(),
        message_base64: challenge.message_base64.clone(),
        signature: BASE64.encode(serialized),
    }
}

pub(super) fn verified(challenge: &AddressBindingChallenge) -> VerifiedWalletResponse {
    let verified_at = Utc.with_ymd_and_hms(2026, 9, 4, 10, 0, 0).single().unwrap();
    verify_wallet_response(challenge, &wallet_response(challenge), verified_at).unwrap()
}

pub(super) fn insert_twenty_recent_expired_challenges(fixture: &Fixture) {
    let subject = subject_commitment(&[9_u8; 32]);
    let conn = fixture.store.conn().unwrap();
    conn.execute(
        "INSERT INTO esk_platform_sui_subjects(
           user_id,subject_commitment,created_session_id,created_at
         ) VALUES('alice',?1,'alice','2026-09-03T13:55:00.000Z')",
        [&subject],
    )
    .unwrap();
    let base = Utc.with_ymd_and_hms(2026, 9, 3, 14, 0, 0).single().unwrap();
    for index in 0..20 {
        let issued = base + Duration::hours(index);
        let material = ChallengeMaterial {
            address: format!("0x{:064x}", index + 100),
            ttl_seconds: 300,
            nonce_base64: BASE64.encode([index as u8 + 10; 32]),
            issued_at: canonical_timestamp(issued),
            expires_at: canonical_timestamp(issued + Duration::minutes(5)),
        };
        let challenge = assemble_challenge(&subject, &material).unwrap();
        let recorded = canonical_timestamp(issued + Duration::minutes(1));
        conn.execute(
            "INSERT INTO esk_platform_sui_address_binding_challenges(
               challenge_id,user_id,subject_commitment,created_session_id,schema,network,purpose,
               address,ttl_seconds,nonce_base64,issued_at,expires_at,message_base64,
               message_sha256,recorded_at
             ) VALUES(?1,'alice',?2,'alice',?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                challenge.challenge_id,
                challenge.subject_commitment,
                challenge.schema,
                challenge.network,
                challenge.purpose,
                challenge.address,
                challenge.ttl_seconds,
                challenge.nonce_base64,
                challenge.issued_at,
                challenge.expires_at,
                challenge.message_base64,
                challenge.message_sha256,
                recorded,
            ],
        )
        .unwrap();
    }
}

pub(super) fn insert_challenge(
    fixture: &Fixture,
    challenge: &AddressBindingChallenge,
    recorded_at: &str,
) {
    let conn = fixture.store.conn().unwrap();
    conn.execute(
        "INSERT INTO esk_platform_sui_subjects(
           user_id,subject_commitment,created_session_id,created_at
         ) VALUES('alice',?1,'alice','2026-09-04T09:40:00.000Z')",
        [&challenge.subject_commitment],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO esk_platform_sui_address_binding_challenges(
           challenge_id,user_id,subject_commitment,created_session_id,schema,network,purpose,
           address,ttl_seconds,nonce_base64,issued_at,expires_at,message_base64,
           message_sha256,recorded_at
         ) VALUES(?1,'alice',?2,'alice',?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            challenge.challenge_id,
            challenge.subject_commitment,
            challenge.schema,
            challenge.network,
            challenge.purpose,
            challenge.address,
            challenge.ttl_seconds,
            challenge.nonce_base64,
            challenge.issued_at,
            challenge.expires_at,
            challenge.message_base64,
            challenge.message_sha256,
            recorded_at,
        ],
    )
    .unwrap();
}

pub(super) fn insert_binding(
    fixture: &Fixture,
    challenge: &AddressBindingChallenge,
    proof: &VerifiedWalletResponse,
    bound_at: &str,
) {
    let id_material = format!(
        "YILONG_ESK_SUI_PLATFORM_BINDING_ID_V2\nchallenge_id={}\nresponse_digest={}",
        challenge.challenge_id, proof.response_digest
    );
    let id_digest = hex::encode(Sha256::digest(id_material.as_bytes()));
    let binding_id = format!("eskpsb_{}", &id_digest[..32]);
    let receipt_material = [
        "YILONG_ESK_SUI_PLATFORM_BINDING_RECEIPT_V2".to_owned(),
        format!("binding_id={binding_id}"),
        format!("challenge_id={}", challenge.challenge_id),
        format!("subject_commitment={}", challenge.subject_commitment),
        format!("address={}", challenge.address),
        format!("network={}", challenge.network),
        format!("message_sha256={}", challenge.message_sha256),
        format!("signature_scheme={}", proof.signature_scheme.as_str()),
        format!("signature_sha256={}", proof.signature_sha256),
        format!("response_digest={}", proof.response_digest),
        format!("verified_at={}", proof.verified_at),
        format!("bound_at={bound_at}"),
    ]
    .join("\n");
    let receipt = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(receipt_material.as_bytes()))
    );
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "INSERT INTO esk_platform_sui_address_bindings(
               binding_id,challenge_id,user_id,address,network,subject_commitment,message_sha256,
               signature_scheme,signature_sha256,response_digest,binding_receipt_sha256,
               wallet_response_json,completed_session_id,verified_at,bound_at
             ) VALUES(?1,?2,'alice',?3,?4,?5,?6,?7,?8,?9,?10,?11,'alice',?12,?13)",
            params![
                binding_id,
                challenge.challenge_id,
                challenge.address,
                challenge.network,
                challenge.subject_commitment,
                challenge.message_sha256,
                proof.signature_scheme.as_str(),
                proof.signature_sha256,
                proof.response_digest,
                receipt,
                proof.wallet_response_json,
                proof.verified_at,
                bound_at,
            ],
        )
        .unwrap();
}
