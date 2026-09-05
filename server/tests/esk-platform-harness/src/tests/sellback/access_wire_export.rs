//! A production Rust serializer export for independent SDK validation, using only synthetic data.
use super::*;
use crate::esk_asset::platform::access::*;
use serde_json::json;
use std::{fs::OpenOptions, io::Write, path::PathBuf};

#[test]
fn synthetic_delegated_wire_export_matches_formal_truth_without_credentials() {
    // setup() creates isolated synthetic Alice/Bob sessions and actual formal ledger records.
    let (fixture, _, config) = setup();
    submit(&fixture, "alice", "synthetic-wire-request", 3, &config);
    let master = token("alice");
    // This public RFC example is synthetic and still passes through the real S256 exchange.
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".to_owned();
    let code = fixture
        .store
        .authorize_asset_access(
            "alice",
            &master,
            &AuthorizeBody {
                schema: AUTHORIZE_SCHEMA.into(),
                client_id: "quant.android".into(),
                redirect_uri: "com.elon.quant:/asset-access/callback".into(),
                state: "synthetic-wire-state-00000000000000".into(),
                code_challenge: challenge(&verifier).unwrap(),
                code_challenge_method: "S256".into(),
                scopes: vec![AccessScope::EskSummaryRead, AccessScope::EskProgressRead],
                expires_in: 600,
                explicit_consent: true,
                confirmation: AUTHORIZE_CONFIRMATION.into(),
            },
            "https://main.example.test",
        )
        .unwrap();
    let grant = fixture
        .store
        .exchange_asset_access_code(
            &TokenBody {
                schema: TOKEN_SCHEMA.into(),
                grant_type: "authorization_code".into(),
                client_id: "quant.android".into(),
                redirect_uri: "com.elon.quant:/asset-access/callback".into(),
                state: code.state.clone(),
                code: code.code.clone(),
                code_verifier: verifier.clone(),
            },
            "https://main.example.test",
        )
        .unwrap();
    let before = std::fs::read(&fixture.path).unwrap();
    let identity = fixture
        .store
        .asset_access_me(&grant.access_token, "quant.android")
        .unwrap();
    let page = fixture
        .store
        .asset_access_esk(
            &grant.access_token,
            "quant.android",
            20,
            None,
            true,
            &config,
        )
        .unwrap();
    assert_eq!(identity.subject, grant.subject);
    assert_eq!(page.subject, identity.subject);
    assert_eq!(identity.expires_at, page.expires_at);
    assert_eq!(page.balance.total_base_units, "10000000");
    assert_eq!(page.balance.reserved_base_units, "3");
    assert_eq!(page.balance.available_base_units, "9999997");
    let progress = page.progress.as_ref().unwrap();
    assert_eq!(progress.request_count, "1");
    assert_eq!(progress.open_count, "1");
    assert_eq!(progress.requests.len(), 1);
    assert!(!progress.has_more);
    assert!(progress.next_cursor.is_none());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), before);

    // Export actual response models, never the access-token/code response or fixture sessions.
    let wire = json!({ "identity": identity, "page": page });
    assert!(wire["identity"].get("nickname").is_none());
    let encoded = serde_json::to_vec_pretty(&wire).unwrap();
    let text = std::str::from_utf8(&encoded).unwrap();
    for secret in [&master, &grant.access_token, &code.code, &verifier] {
        assert!(!text.contains(secret.as_str()));
    }
    for forbidden in [
        "\"access_token\"",
        "\"refresh_token\"",
        "\"code\"",
        "\"user_id\"",
        "\"account\"",
        "\"email\"",
        "\"role\"",
        "alice",
        "bob",
        "payment_evidence",
        "synthetic-wire-request",
    ] {
        assert!(!text.contains(forbidden));
    }

    let Some(output) = std::env::var_os("ELON_ASSET_ACCESS_WIRE_OUTPUT") else {
        return;
    };
    let output = PathBuf::from(output);
    assert!(output.is_absolute());
    assert_eq!(output.extension().and_then(|v| v.to_str()), Some("json"));
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .canonicalize()
        .unwrap();
    let scratch = repo.join(".ai-tmp").canonicalize().unwrap();
    let parent = output.parent().unwrap().canonicalize().unwrap();
    assert!(parent.starts_with(&scratch));
    // Do not overwrite an existing artifact or follow an existing file symlink.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .unwrap();
    file.write_all(&encoded).unwrap();
    file.write_all(b"\n").unwrap();
    file.sync_all().unwrap();
}
