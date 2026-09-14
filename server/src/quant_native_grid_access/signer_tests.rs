use super::*;
use ring::signature::{KeyPair, UnparsedPublicKey, ED25519};

fn configured(key: &str, seed: u8, subject: u8) -> Signer {
    Signer::from_lookup(|name| {
        Ok(Some(match name {
            KEY => key.to_owned(),
            SEED => URL_SAFE_NO_PAD.encode([seed; 32]),
            SUBJECT => URL_SAFE_NO_PAD.encode([subject; 32]),
            _ => panic!("unexpected setting"),
        }))
    })
    .unwrap()
}

#[test]
fn independent_configuration_is_strict() {
    assert!(matches!(
        Signer::from_lookup(|_| Ok(None)),
        Err(Error::Disabled)
    ));
    for missing in [KEY, SEED, SUBJECT] {
        assert!(matches!(
            Signer::from_lookup(|name| Ok(if name == missing {
                None
            } else {
                Some(if name == KEY {
                    "test-key".into()
                } else {
                    URL_SAFE_NO_PAD.encode([7; 32])
                })
            })),
            Err(Error::Misconfigured)
        ));
    }
    for bad in [
        "".to_owned(),
        " padded ".into(),
        URL_SAFE_NO_PAD.encode([1; 31]),
        format!("{}=", URL_SAFE_NO_PAD.encode([1; 32])),
    ] {
        assert!(matches!(
            Signer::from_lookup(|name| Ok(Some(if name == KEY {
                "test-key".into()
            } else {
                bad.clone()
            }))),
            Err(Error::Misconfigured)
        ));
    }
    assert!(matches!(
        Signer::from_lookup(|_| Err(Error::Misconfigured)),
        Err(Error::Misconfigured)
    ));
}

#[test]
fn signature_binds_prefix_and_wire_bytes() {
    let signer = configured("test-key", 7, 11);
    let value = signer
        .issue("synthetic-alice", vec![Scope::Read], 1700000000, 1700000300)
        .unwrap();
    let parts: Vec<_> = value.access_token.split('.').collect();
    let signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
    let verifier = UnparsedPublicKey::new(&ED25519, signer.signing_key.public_key().as_ref());
    assert!(verifier
        .verify(format!("yng1.{}", parts[1]).as_bytes(), &signature)
        .is_ok());
    assert!(verifier
        .verify(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap(), &signature)
        .is_err());
    assert!(verifier
        .verify(format!("ypg1.{}", parts[1]).as_bytes(), &signature)
        .is_err());
    let claims: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
    assert_eq!(claims["audience"], "yilong-quant-native-grid");
    assert_eq!(claims["environment"], "native_paper");
    assert_eq!(claims["scopes"], serde_json::json!(["native_grid.read"]));
    assert!(!value.access_token.contains("synthetic-alice"));
}

#[test]
fn identity_survives_signing_key_rotation_and_separates_users() {
    let first = configured("test-key", 7, 11);
    let rotated = configured("next-key", 8, 11);
    let changed_subject = configured("test-key", 7, 12);
    let issue = |s: &Signer, user| {
        s.issue(user, vec![Scope::Read], 1700000000, 1700000300)
            .unwrap()
    };
    let a = issue(&first, "synthetic-alice");
    assert_eq!(
        a.subject_ref,
        issue(&rotated, "synthetic-alice").subject_ref
    );
    assert_ne!(a.subject_ref, issue(&first, "synthetic-bob").subject_ref);
    assert_ne!(
        a.subject_ref,
        issue(&changed_subject, "synthetic-alice").subject_ref
    );
    assert_ne!(a.grant_id, issue(&first, "synthetic-alice").grant_id);
}

#[test]
fn signer_rejects_invalid_lifetimes_and_scope_expansion() {
    let signer = configured("test-key", 7, 11);
    for (now, expires) in [(0, 300), (100, 100), (100, 401), (i64::MIN, i64::MAX)] {
        assert!(matches!(
            signer.issue("synthetic-alice", vec![Scope::Read], now, expires),
            Err(Error::InvalidInput)
        ));
    }
    for scopes in [vec![], vec![Scope::Read, Scope::Read]] {
        assert!(matches!(
            signer.issue("synthetic-alice", scopes, 100, 200),
            Err(Error::InvalidInput)
        ));
    }
    for user in ["", " ", "local-owner"] {
        assert!(matches!(
            signer.issue(user, vec![Scope::Read], 100, 200),
            Err(Error::InvalidInput)
        ));
    }
}

#[test]
fn shared_consumer_fixture_is_exact_production_output() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/quant-native-grid-access-harness/fixtures/native-grid-grant-v1.json"
    ))
    .unwrap();
    let signer = configured("native-grid-test-key", 7, 11);
    let response = signer
        .issue_with_id(
            "synthetic-alice",
            vec![Scope::Read, Scope::Create, Scope::Control],
            1700000000,
            1700000300,
            "ngg_0123456789abcdef0123456789abcdef".into(),
        )
        .unwrap();
    assert_eq!(response.access_token, fixture["access_token"]);
    assert_eq!(response.subject_ref, fixture["subject_ref"]);
    assert_eq!(
        URL_SAFE_NO_PAD.encode(signer.signing_key.public_key().as_ref()),
        fixture["public_key_base64url"]
    );
}
