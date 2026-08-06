use uuid::Uuid;

use super::{AccountSecurityError, Store, VerifiedIdentity};

fn temporary_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon_account_security_{}.db",
        Uuid::new_v4().simple()
    ));
    (Store::open(&path).expect("store opens"), path)
}

fn remove_database(path: &std::path::Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{}", path.display(), suffix));
    }
}

#[test]
fn federated_only_account_can_set_password_and_keep_current_session() {
    let (store, path) = temporary_store();
    let challenge = store
        .create_identity_challenge("google", "login", None, "web")
        .unwrap();
    let completion = store
        .complete_identity_challenge(
            &challenge.id,
            &VerifiedIdentity {
                provider: "google".to_string(),
                issuer: "https://accounts.google.com".to_string(),
                subject: "security-subject".to_string(),
                email: "federated-security@example.com".to_string(),
                display_name: Some("安全测试".to_string()),
                avatar_url: None,
                nonce: challenge.nonce,
            },
        )
        .unwrap();
    let (current_token, _) = store
        .create_session(&completion.user.id, Some("Current PC"), None)
        .unwrap();

    let changed = store
        .change_account_password(
            &completion.user.id,
            &current_token,
            None,
            "strong-password-1",
            "request-set-password",
        )
        .unwrap();
    assert!(!changed.replayed);
    assert_eq!(changed.revoked_session_count, 0);
    assert_eq!(
        store
            .authenticate_password("federated-security@example.com", "strong-password-1")
            .unwrap()
            .id,
        completion.user.id
    );
    assert!(store.authenticate_token(&current_token).is_ok());
    drop(store);
    remove_database(&path);
}

#[test]
fn password_change_is_idempotent_and_revokes_other_sessions() {
    let (store, path) = temporary_store();
    let user = store
        .create_user("password-security@example.com", "secret1", None, None)
        .unwrap();
    let (current_token, _) = store
        .create_session(&user.id, Some("Current"), None)
        .unwrap();
    let (other_token, _) = store.create_session(&user.id, Some("Other"), None).unwrap();

    let changed = store
        .change_account_password(
            &user.id,
            &current_token,
            Some("secret1"),
            "secret2",
            "password-change-1",
        )
        .unwrap();
    assert_eq!(changed.revoked_session_count, 1);
    assert!(store.authenticate_token(&current_token).is_ok());
    assert!(store.authenticate_token(&other_token).is_err());
    let replay = store
        .change_account_password(
            &user.id,
            &current_token,
            Some("secret1"),
            "secret2",
            "password-change-1",
        )
        .unwrap();
    assert!(replay.replayed);
    drop(store);
    remove_database(&path);
}

#[test]
fn recovery_code_resets_password_once_and_revokes_all_sessions() {
    let (store, path) = temporary_store();
    let user = store
        .create_user("recover-security@example.com", "secret1", None, None)
        .unwrap();
    let (token, _) = store.create_session(&user.id, Some("Phone"), None).unwrap();
    let rotation = store
        .rotate_account_recovery_codes(&user.id, Some("secret1"), "recovery-rotate-1")
        .unwrap();
    assert_eq!(rotation.codes.len(), 8);
    let code = rotation.codes[0].clone();

    let recovered = store
        .recover_account_password(
            "recover-security@example.com",
            &code,
            "secret3",
            "password-recover-1",
        )
        .unwrap();
    assert_eq!(recovered.revoked_session_count, 1);
    assert!(store.authenticate_token(&token).is_err());
    assert!(store
        .authenticate_password("recover-security@example.com", "secret3")
        .is_ok());
    assert!(matches!(
        store.recover_account_password(
            "recover-security@example.com",
            &code,
            "secret4",
            "password-recover-2"
        ),
        Err(AccountSecurityError::InvalidRecoveryCode)
    ));
    drop(store);
    remove_database(&path);
}
