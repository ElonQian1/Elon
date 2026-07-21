use super::*;
use rsa::{
    pkcs1v15::SigningKey,
    rand_core::OsRng,
    signature::{SignatureEncoding, Signer},
    RsaPrivateKey,
};

fn temp_ledger(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "elon-review-{name}-{}-{}.json",
        std::process::id(),
        uuid::Uuid::new_v4()
    ))
}

fn test_auth(path: PathBuf, keys: &[&RsaPrivateKey], allow_v2: bool) -> DesktopReviewAuth {
    DesktopReviewAuth {
        credential: Some(Arc::from(
            b"legacy-shared-secret-at-least-32-bytes".as_slice(),
        )),
        public_keys: Arc::new(
            keys.iter()
                .enumerate()
                .map(|(index, key)| (format!("{index:016x}"), key.to_public_key()))
                .collect(),
        ),
        allow_v2,
        nonce_ledger: Some(Arc::new(NonceLedger {
            path,
            lock: Mutex::new(()),
        })),
    }
}

fn v3_ticket(
    key: &RsaPrivateKey,
    key_id: &str,
    expires: u64,
    nonce: &str,
    owner: &str,
    task: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> String {
    let hash = hex::encode(Sha256::digest(body));
    let message = ticket_message_v3(owner, task, method, path, &hash, expires, nonce, key_id);
    let signature = SigningKey::<Sha256>::new(key.clone()).sign(message.as_bytes());
    format!(
        "v3.{key_id}.{expires}.{nonce}.{}",
        BASE64.encode(signature.to_bytes())
    )
}

fn v2_ticket(
    key: &RsaPrivateKey,
    key_id: &str,
    expires: u64,
    nonce: &str,
    owner: &str,
    task: &str,
) -> String {
    let message = format!("v2\n{owner}\n{task}\n{expires}\n{nonce}");
    let signature = SigningKey::<Sha256>::new(key.clone()).sign(message.as_bytes());
    format!(
        "v2.{key_id}.{expires}.{nonce}.{}",
        BASE64.encode(signature.to_bytes())
    )
}

fn headers(ticket: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(DESKTOP_REVIEW_TICKET_HEADER, ticket.parse().unwrap());
    headers
}

#[test]
fn v3_binds_method_path_exact_body_owner_task_and_key() {
    let key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let auth = test_auth(temp_ledger("binding"), &[&key], false);
    let now = now_secs();
    let body = r#"{"verdict":"accepted","summary":"钱一龙"}"#.as_bytes();
    let path = "/api/local-tasks/local-a/supervision/desktop-review";
    let ticket = v3_ticket(
        &key,
        "0000000000000000",
        now + 120,
        "nonce-1234567890",
        "owner",
        "local-a",
        "POST",
        path,
        body,
    );
    for (owner, task, method, changed_path, changed_body) in [
        ("other", "local-a", "POST", path, body),
        ("owner", "local-b", "POST", path, body),
        ("owner", "local-a", "PUT", path, body),
        ("owner", "local-a", "POST", "/other", body),
        ("owner", "local-a", "POST", path, &b"{}"[..]),
    ] {
        assert_eq!(
            auth.verify_and_consume(
                &headers(&ticket),
                owner,
                task,
                method,
                changed_path,
                changed_body
            ),
            Err(DesktopReviewAuthError::Invalid)
        );
    }
    assert_eq!(
        auth.verify_and_consume(&headers(&ticket), "owner", "local-a", "POST", path, body),
        Ok(())
    );
    assert_eq!(
        auth.verify_and_consume(&headers(&ticket), "owner", "local-a", "POST", path, body),
        Err(DesktopReviewAuthError::Replayed)
    );
}

#[test]
fn nonce_is_atomic_bounded_and_survives_restart() {
    let key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let ledger = temp_ledger("replay");
    let auth = Arc::new(test_auth(ledger.clone(), &[&key], false));
    let now = now_secs();
    let path = "/api/local-tasks/local-a/supervision/desktop-review";
    let body = b"{}";
    let ticket = v3_ticket(
        &key,
        "0000000000000000",
        now + 120,
        "concurrent-nonce-1234",
        "owner",
        "local-a",
        "POST",
        path,
        body,
    );
    let results: Vec<_> = (0..8)
        .map(|_| {
            let auth = auth.clone();
            let ticket = ticket.clone();
            std::thread::spawn(move || {
                auth.verify_and_consume(&headers(&ticket), "owner", "local-a", "POST", path, body)
            })
        })
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let restarted = test_auth(ledger, &[&key], false);
    assert_eq!(
        restarted.verify_and_consume(&headers(&ticket), "owner", "local-a", "POST", path, body),
        Err(DesktopReviewAuthError::Replayed)
    );
}

#[test]
fn expiry_unknown_key_rotation_and_downgrade_are_fail_closed() {
    let old = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let new = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let auth = test_auth(temp_ledger("rotation"), &[&old, &new], false);
    let now = now_secs();
    let path = "/api/local-tasks/t/supervision/desktop-review";
    let body = b"{}";
    for (key, id, nonce) in [
        (&old, "0000000000000000", "old-key-nonce-1234"),
        (&new, "0000000000000001", "new-key-nonce-1234"),
    ] {
        let ticket = v3_ticket(key, id, now + 120, nonce, "o", "t", "POST", path, body);
        assert_eq!(
            auth.verify_and_consume(&headers(&ticket), "o", "t", "POST", path, body),
            Ok(())
        );
    }
    let expired = v3_ticket(
        &new,
        "0000000000000001",
        now - 30,
        "expired-nonce-1234",
        "o",
        "t",
        "POST",
        path,
        body,
    );
    assert_eq!(
        auth.verify_and_consume(&headers(&expired), "o", "t", "POST", path, body),
        Err(DesktopReviewAuthError::Expired)
    );
    let unknown = v3_ticket(
        &new,
        "ffffffffffffffff",
        now + 120,
        "unknown-key-nonce1",
        "o",
        "t",
        "POST",
        path,
        body,
    );
    assert_eq!(
        auth.verify_and_consume(&headers(&unknown), "o", "t", "POST", path, body),
        Err(DesktopReviewAuthError::Invalid)
    );
    let legacy = DesktopReviewAuth::for_test("legacy-shared-secret-at-least-32-bytes")
        .mint_for_test("o", "t", now + 120, "legacy-nonce-1234");
    assert_eq!(
        auth.verify_and_consume(&headers(&legacy), "o", "t", "POST", path, body),
        Err(DesktopReviewAuthError::Invalid)
    );
}

#[test]
fn v2_migration_requires_explicit_opt_in_and_never_enables_v1_fallback() {
    let key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let now = now_secs();
    let path = "/api/local-tasks/t/supervision/desktop-review";
    let body = b"{}";
    let v2 = v2_ticket(
        &key,
        "0000000000000000",
        now + 120,
        "migration-nonce-1234",
        "o",
        "t",
    );
    let default_closed = test_auth(temp_ledger("v2-closed"), &[&key], false);
    assert_eq!(
        default_closed.verify_and_consume(&headers(&v2), "o", "t", "POST", path, body),
        Err(DesktopReviewAuthError::Invalid)
    );

    let migration = test_auth(temp_ledger("v2-open"), &[&key], true);
    assert_eq!(
        migration.verify_and_consume(&headers(&v2), "o", "t", "POST", path, body),
        Ok(())
    );
    let v1 = DesktopReviewAuth::for_test("legacy-shared-secret-at-least-32-bytes").mint_for_test(
        "o",
        "t",
        now + 120,
        "legacy-nonce-5678",
    );
    assert_eq!(
        migration.verify_and_consume(&headers(&v1), "o", "t", "POST", path, body),
        Err(DesktopReviewAuthError::Invalid)
    );
}
