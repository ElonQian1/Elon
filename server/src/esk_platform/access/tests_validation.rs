use super::{model::*, validation::*};

#[test]
fn pkce_s256_uses_rfc_vector_and_rejects_weak_verifiers() {
    assert_eq!(
        challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap(),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    assert!(challenge("short").is_err());
    assert!(challenge(&"?".repeat(43)).is_err());
    assert!(challenge(&"x".repeat(129)).is_err());
}

#[test]
fn scopes_require_summary_and_never_accept_wildcards_or_duplicates() {
    assert!(valid_scopes(&[AccessScope::EskSummaryRead]));
    assert!(!valid_scopes(&[AccessScope::ProfileRead]));
    assert!(!valid_scopes(&[
        AccessScope::EskSummaryRead,
        AccessScope::EskSummaryRead
    ]));
    assert!(serde_json::from_str::<Vec<AccessScope>>("[\"*\"]").is_err());
    assert!(serde_json::from_str::<Vec<AccessScope>>("[\"esk.sellback.write\"]").is_err());
}

#[test]
fn callbacks_are_exact_and_public_http_cannot_become_web_callback() {
    let public = "https://main.example.test";
    assert!(validate_redirect(
        "quant.android",
        "com.elon.quant:/asset-access/callback",
        public
    )
    .is_ok());
    assert!(validate_redirect(
        "quant.web",
        "https://main.example.test/quant/asset-access/callback",
        public
    )
    .is_ok());
    assert!(validate_redirect(
        "quant.ai",
        "http://127.0.0.1:48151/asset-access/callback",
        public
    )
    .is_ok());
    for uri in [
        "http://localhost:48151/asset-access/callback",
        "http://127.0.0.1:80/asset-access/callback",
        "http://127.0.0.1:48151/asset-access/callback?secret=x",
        "http://127.0.0.2:48151/asset-access/callback",
    ] {
        assert!(validate_redirect("quant.ai", uri, public).is_err());
    }
    assert!(validate_redirect(
        "quant.web",
        "http://main.example.test/quant/asset-access/callback",
        "http://main.example.test"
    )
    .is_err());
    assert!(validate_redirect(
        "quant.web",
        "https://other.example.test/quant/asset-access/callback",
        public
    )
    .is_err());
}
