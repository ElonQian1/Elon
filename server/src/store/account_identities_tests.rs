use uuid::Uuid;

use super::{IdentityError, Store, VerifiedIdentity};

fn temporary_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!("elon_identity_{}.db", Uuid::new_v4().simple()));
    (Store::open(&path).expect("store opens"), path)
}

fn google_identity(nonce: &str, subject: &str, email: &str) -> VerifiedIdentity {
    VerifiedIdentity {
        provider: "google".to_string(),
        issuer: "https://accounts.google.com".to_string(),
        subject: subject.to_string(),
        email: email.to_string(),
        display_name: Some("测试用户".to_string()),
        avatar_url: None,
        nonce: nonce.to_string(),
    }
}

#[test]
fn first_google_login_creates_federated_only_user() {
    let (store, path) = temporary_store();
    let challenge = store
        .create_identity_challenge("google", "login", None, "web")
        .unwrap();
    let completion = store
        .complete_identity_challenge(
            &challenge.id,
            &google_identity(&challenge.nonce, "google-sub-1", "new@example.com"),
        )
        .unwrap();
    assert!(completion.created_user);
    assert_eq!(completion.user.account, "new@example.com");
    assert!(matches!(
        store.unlink_identity(&completion.user.id, &completion.identity.id),
        Err(IdentityError::CannotUnlinkLastLogin)
    ));
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn matching_email_requires_explicit_bind() {
    let (store, path) = temporary_store();
    store
        .create_user("existing@example.com", "secret1", None, None)
        .unwrap();
    let challenge = store
        .create_identity_challenge("google", "login", None, "android")
        .unwrap();
    let result = store.complete_identity_challenge(
        &challenge.id,
        &google_identity(&challenge.nonce, "google-sub-2", "existing@example.com"),
    );
    assert!(matches!(
        result,
        Err(IdentityError::ExistingAccountRequiresBind)
    ));
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn password_user_can_bind_and_unlink_google() {
    let (store, path) = temporary_store();
    let user = store
        .create_user("owner@example.com", "secret1", None, None)
        .unwrap();
    let challenge = store
        .create_identity_challenge("google", "bind", Some(&user.id), "windows")
        .unwrap();
    let completion = store
        .complete_identity_challenge(
            &challenge.id,
            &google_identity(&challenge.nonce, "google-sub-3", "owner@gmail.com"),
        )
        .unwrap();
    assert!(!completion.created_user);
    store
        .unlink_identity(&user.id, &completion.identity.id)
        .unwrap();
    assert!(store.list_linked_identities(&user.id).unwrap().is_empty());
    drop(store);
    let _ = std::fs::remove_file(path);
}
