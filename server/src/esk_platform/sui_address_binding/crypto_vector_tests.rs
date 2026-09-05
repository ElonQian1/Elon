use super::*;

const SUBJECT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const NONCE: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
const ISSUED_AT: &str = "2026-09-05T08:00:00.000Z";
const EXPIRES_AT: &str = "2026-09-05T08:10:00.000Z";

struct MystenVector {
    address: &'static str,
    message_base64: &'static str,
    signature: &'static str,
    scheme: SignatureScheme,
}

// Generated and independently verified with @mysten/sui 2.29.0 and its locked
// @noble/curves 2.4.0 dependency, using only obvious fixed test keys
// ([0x01;32], [0x02;32], [0x03;32]). These constants carry no production
// wallet authority and require no RPC or network access.
const VECTORS: &[MystenVector] = &[
    MystenVector {
        address: "0x29dfbf688abce7ab43bb8e70cae158ae961196e721440f515482f8ba1684390f",
        message_base64: "WUlMT05HX0VTS19TVUlfQUREUkVTU19CSU5ESU5HX1YxCm5ldHdvcms9dGVzdG5ldApwdXJwb3NlPXVzZXJfYXNzZXRfbWlncmF0aW9uCnN1YmplY3RfY29tbWl0bWVudD1zaGEyNTY6YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYQphZGRyZXNzPTB4MjlkZmJmNjg4YWJjZTdhYjQzYmI4ZTcwY2FlMTU4YWU5NjExOTZlNzIxNDQwZjUxNTQ4MmY4YmExNjg0MzkwZgpub25jZV9iYXNlNjQ9QndjSEJ3Y0hCd2NIQndjSEJ3Y0hCd2NIQndjSEJ3Y0hCd2NIQndjSEJ3Yz0KaXNzdWVkX2F0PTIwMjYtMDktMDVUMDg6MDA6MDAuMDAwWgpleHBpcmVzX2F0PTIwMjYtMDktMDVUMDg6MTA6MDAuMDAwWg==",
        signature: "AGdv19o56PZ3nhX04OBY8ktyxUt79RCUE/1HKjfYuyvoLnQgQvygWe4OJXJg9GvbsVdMal89mqU39NTQcKLG0wSKiOPddAnxlf1S2y08ul1yymcJvx2UEhvzdIgBtA9vXA==",
        scheme: SignatureScheme::Ed25519,
    },
    MystenVector {
        address: "0x96465ea51057d7a92bc9bae86f950cbcfd3e1ce58242be01c8c64cff7c669232",
        message_base64: "WUlMT05HX0VTS19TVUlfQUREUkVTU19CSU5ESU5HX1YxCm5ldHdvcms9dGVzdG5ldApwdXJwb3NlPXVzZXJfYXNzZXRfbWlncmF0aW9uCnN1YmplY3RfY29tbWl0bWVudD1zaGEyNTY6YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYQphZGRyZXNzPTB4OTY0NjVlYTUxMDU3ZDdhOTJiYzliYWU4NmY5NTBjYmNmZDNlMWNlNTgyNDJiZTAxYzhjNjRjZmY3YzY2OTIzMgpub25jZV9iYXNlNjQ9QndjSEJ3Y0hCd2NIQndjSEJ3Y0hCd2NIQndjSEJ3Y0hCd2NIQndjSEJ3Yz0KaXNzdWVkX2F0PTIwMjYtMDktMDVUMDg6MDA6MDAuMDAwWgpleHBpcmVzX2F0PTIwMjYtMDktMDVUMDg6MTA6MDAuMDAwWg==",
        signature: "AcaELZPPaLbJvJij8sruvNVzMmh0iKmsvekgFVg9yVBfPnkw4kmv6LuSf6cot9KrE+wljnld7BOtq6lEvlKnF7ACTUts0TYQMsqb0q652QCqTUXZ6tgKyUIzdMRRpyVNB2Y=",
        scheme: SignatureScheme::Secp256k1,
    },
    MystenVector {
        address: "0x64a32d2f8b9ce1c87c71a7868adc02e4b07a28e1318fd66651f14800279fd6fb",
        message_base64: "WUlMT05HX0VTS19TVUlfQUREUkVTU19CSU5ESU5HX1YxCm5ldHdvcms9dGVzdG5ldApwdXJwb3NlPXVzZXJfYXNzZXRfbWlncmF0aW9uCnN1YmplY3RfY29tbWl0bWVudD1zaGEyNTY6YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYQphZGRyZXNzPTB4NjRhMzJkMmY4YjljZTFjODdjNzFhNzg2OGFkYzAyZTRiMDdhMjhlMTMxOGZkNjY2NTFmMTQ4MDAyNzlmZDZmYgpub25jZV9iYXNlNjQ9QndjSEJ3Y0hCd2NIQndjSEJ3Y0hCd2NIQndjSEJ3Y0hCd2NIQndjSEJ3Yz0KaXNzdWVkX2F0PTIwMjYtMDktMDVUMDg6MDA6MDAuMDAwWgpleHBpcmVzX2F0PTIwMjYtMDktMDVUMDg6MTA6MDAuMDAwWg==",
        signature: "ArHflbqinhbya3RGkv74WqmfilGrLyTzUJnOeHKWtr5RepA4KemscZyqBPQ6+2XwDlSSfFGqnnHQYrJEaSA/OMsCWRq3ceu8/W2cuQlNEGUordGmnUTCwfYn8InsWLnGGt8=",
        scheme: SignatureScheme::Secp256r1,
    },
];

#[test]
fn rust_verifier_accepts_fixed_mysten_sdk_personal_message_vectors() {
    let verified_at = parse_timestamp("2026-09-05T08:05:00.000Z").unwrap();
    for vector in VECTORS {
        let challenge = assemble_challenge(
            SUBJECT,
            &ChallengeMaterial {
                address: vector.address.to_owned(),
                ttl_seconds: 600,
                nonce_base64: NONCE.to_owned(),
                issued_at: ISSUED_AT.to_owned(),
                expires_at: EXPIRES_AT.to_owned(),
            },
        )
        .unwrap();
        assert_eq!(challenge.message_base64, vector.message_base64);
        let response = WalletResponseBody {
            schema: WALLET_RESPONSE_SCHEMA.to_owned(),
            challenge_id: challenge.challenge_id.clone(),
            message_base64: vector.message_base64.to_owned(),
            signature: vector.signature.to_owned(),
        };
        let verified =
            verify_wallet_response(&challenge, &response, verified_at).unwrap_or_else(|error| {
                panic!("{} Mysten vector failed: {error}", vector.scheme.as_str())
            });
        assert_eq!(verified.address, vector.address);
        assert_eq!(verified.signature_scheme, vector.scheme);
    }
}
