use std::ffi::OsString;

use super::{
    external_pool_adapter_runtime_compatibility_signing_handoff_runtime::signing_handoff_runtime_path,
    external_pool_adapter_runtime_compatibility_signing_handoff_service::RuntimeCompatibilitySigningHandoffServiceError,
    external_pool_adapter_runtime_compatibility_signing_handoff_service_validation::{
        validate_signing_handoff, RuntimeCompatibilitySigningHandoffBody,
    },
};

fn body() -> RuntimeCompatibilitySigningHandoffBody {
    RuntimeCompatibilitySigningHandoffBody {
        expected_challenge_digest: "a".repeat(64),
        provider_binding_id: "provider-binding-1".to_string(),
        expected_provider_binding_digest: "b".repeat(64),
        expected_installation_receipt_id: "installation-receipt-1".to_string(),
        expected_installation_receipt_digest: "c".repeat(64),
        confirm_signing_handoff: true,
    }
}

fn assert_invalid(result: Result<(), RuntimeCompatibilitySigningHandoffServiceError>) {
    assert!(matches!(
        result,
        Err(RuntimeCompatibilitySigningHandoffServiceError::Invalid(_))
    ));
}

#[test]
fn signing_handoff_runtime_defaults_to_disabled() {
    assert!(signing_handoff_runtime_path(None, None).unwrap().is_none());
    assert!(
        signing_handoff_runtime_path(Some(OsString::from("false")), None)
            .unwrap()
            .is_none()
    );
}

#[test]
fn signing_handoff_runtime_rejects_non_exact_enabled_values() {
    for value in ["TRUE", "False", "1", " true", "true ", ""] {
        assert!(signing_handoff_runtime_path(Some(OsString::from(value)), None).is_err());
    }
}

#[test]
fn signing_handoff_runtime_rejects_disabled_or_incomplete_paths() {
    assert!(signing_handoff_runtime_path(
        Some(OsString::from("false")),
        Some(OsString::from("/sys/fs/cgroup/delegated")),
    )
    .is_err());
    assert!(signing_handoff_runtime_path(Some(OsString::from("true")), None).is_err());
    assert!(
        signing_handoff_runtime_path(Some(OsString::from("true")), Some(OsString::new()),).is_err()
    );
    assert!(signing_handoff_runtime_path(
        Some(OsString::from("true")),
        Some(OsString::from("relative/cgroup")),
    )
    .is_err());
}

#[test]
fn signing_handoff_runtime_accepts_an_absolute_path_without_opening_it() {
    let path = std::env::temp_dir().join("v269-delegated-cgroup");
    assert_eq!(
        signing_handoff_runtime_path(
            Some(OsString::from("true")),
            Some(path.clone().into_os_string()),
        )
        .unwrap(),
        Some(path)
    );
}

#[test]
fn signing_handoff_validation_accepts_the_exact_body() {
    validate_signing_handoff("admin-1", "release-1", "challenge-1", &body()).unwrap();
}

#[test]
fn signing_handoff_validation_requires_explicit_confirmation() {
    let mut request = body();
    request.confirm_signing_handoff = false;
    assert_invalid(validate_signing_handoff(
        "admin-1",
        "release-1",
        "challenge-1",
        &request,
    ));
}

#[test]
fn signing_handoff_validation_rejects_invalid_identifiers() {
    for value in ["", " leading", "trailing ", "control\n"] {
        assert_invalid(validate_signing_handoff(
            value,
            "release-1",
            "challenge-1",
            &body(),
        ));
        assert_invalid(validate_signing_handoff(
            "admin-1",
            value,
            "challenge-1",
            &body(),
        ));
        assert_invalid(validate_signing_handoff(
            "admin-1",
            "release-1",
            value,
            &body(),
        ));

        let mut provider = body();
        provider.provider_binding_id = value.to_string();
        assert_invalid(validate_signing_handoff(
            "admin-1",
            "release-1",
            "challenge-1",
            &provider,
        ));

        let mut receipt = body();
        receipt.expected_installation_receipt_id = value.to_string();
        assert_invalid(validate_signing_handoff(
            "admin-1",
            "release-1",
            "challenge-1",
            &receipt,
        ));
    }
    let too_long = "x".repeat(201);
    assert_invalid(validate_signing_handoff(
        &too_long,
        "release-1",
        "challenge-1",
        &body(),
    ));
}

#[test]
fn signing_handoff_validation_rejects_noncanonical_digests() {
    for invalid_digest in [
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
    ] {
        let mut challenge = body();
        challenge.expected_challenge_digest = invalid_digest.clone();
        assert_invalid(validate_signing_handoff(
            "admin-1",
            "release-1",
            "challenge-1",
            &challenge,
        ));

        let mut binding = body();
        binding.expected_provider_binding_digest = invalid_digest.clone();
        assert_invalid(validate_signing_handoff(
            "admin-1",
            "release-1",
            "challenge-1",
            &binding,
        ));

        let mut receipt = body();
        receipt.expected_installation_receipt_digest = invalid_digest;
        assert_invalid(validate_signing_handoff(
            "admin-1",
            "release-1",
            "challenge-1",
            &receipt,
        ));
    }
}

#[test]
fn signing_handoff_body_rejects_unknown_fields_and_string_confirmation() {
    let exact = serde_json::json!({
        "expected_challenge_digest": "a".repeat(64),
        "provider_binding_id": "provider-binding-1",
        "expected_provider_binding_digest": "b".repeat(64),
        "expected_installation_receipt_id": "installation-receipt-1",
        "expected_installation_receipt_digest": "c".repeat(64),
        "confirm_signing_handoff": true,
    });
    serde_json::from_value::<RuntimeCompatibilitySigningHandoffBody>(exact.clone()).unwrap();

    let mut unknown = exact.clone();
    unknown["cgroup_path"] = serde_json::Value::String("/sys/fs/cgroup".to_string());
    assert!(serde_json::from_value::<RuntimeCompatibilitySigningHandoffBody>(unknown).is_err());

    let mut string_confirmation = exact;
    string_confirmation["confirm_signing_handoff"] = serde_json::Value::String("true".to_string());
    assert!(
        serde_json::from_value::<RuntimeCompatibilitySigningHandoffBody>(string_confirmation)
            .is_err()
    );
}
