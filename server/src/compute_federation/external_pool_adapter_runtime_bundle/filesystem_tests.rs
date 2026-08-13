use super::*;

#[test]
fn exact_sha256_accepts_only_the_exact_well_formed_digest() {
    let bytes = b"test-only-config";
    let expected = hex::encode(Sha256::digest(bytes));
    let expected_bytes: [u8; 32] = Sha256::digest(bytes).into();

    assert_eq!(
        exact_sha256(bytes, &expected).expect("exact digest"),
        expected_bytes
    );
    assert_eq!(
        exact_sha256(bytes, &format!("{expected}0")).unwrap_err(),
        ExternalPoolAdapterRuntimeBundleError::InvalidAuthority
    );
}

#[test]
fn exact_sha256_rejects_content_drift_without_returning_observed_hash() {
    let expected = hex::encode(Sha256::digest(b"expected-test-only-credential"));

    assert_eq!(
        exact_sha256(b"drifted-test-only-credential", &expected).unwrap_err(),
        ExternalPoolAdapterRuntimeBundleError::ContentDrift
    );
}

#[test]
fn profile_digest_requires_exact_lowercase_sha256_shape() {
    let valid = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    assert!(validate_profile_digest(valid).is_ok());
    assert_eq!(
        validate_profile_digest(&valid.to_ascii_uppercase()).unwrap_err(),
        ExternalPoolAdapterRuntimeBundleError::InvalidAuthority
    );
    assert_eq!(
        validate_profile_digest(&valid[..63]).unwrap_err(),
        ExternalPoolAdapterRuntimeBundleError::InvalidAuthority
    );
}
