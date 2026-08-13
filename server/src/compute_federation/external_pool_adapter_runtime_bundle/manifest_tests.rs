use super::*;

const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DIGEST_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const DIGEST_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const DIGEST_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

fn expected() -> ExpectedExternalPoolAdapterRuntimeBundle {
    ExpectedExternalPoolAdapterRuntimeBundle {
        profile_id: "profile-1".into(),
        profile_digest: DIGEST_A.into(),
        launch_policy_digest: DIGEST_B.into(),
        candidate_id: "candidate-1".into(),
        candidate_digest: DIGEST_C.into(),
        provider_binding_id: "binding-1".into(),
        provider_binding_digest: DIGEST_D.into(),
        provider_id: "provider-1".into(),
        provider_owner_account_id: "owner-1".into(),
        logical_adapter_id: "adapter-1".into(),
        release_version: "1.0.0".into(),
        adapter_config_revision: 7,
        adapter_config_digest: "config-revision-digest".into(),
        credential_locator_commitment: DIGEST_E.into(),
        credential_reattestation_receipt_id: "credential-receipt-1".into(),
        credential_reattestation_receipt_digest: DIGEST_F.into(),
        credential_reattestation_material_digest: DIGEST_A.into(),
        credential_report_expires_at: "2026-08-15T00:00:00Z".into(),
    }
}

fn manifest(
    expected: &ExpectedExternalPoolAdapterRuntimeBundle,
) -> ExternalPoolAdapterRuntimeBundleManifest {
    ExternalPoolAdapterRuntimeBundleManifest {
        adapter_config_digest: expected.adapter_config_digest.clone(),
        adapter_config_revision: expected.adapter_config_revision,
        bundle_generation: 1,
        candidate_digest: expected.candidate_digest.clone(),
        candidate_id: expected.candidate_id.clone(),
        config_sha256: DIGEST_B.into(),
        config_size_bytes: 4,
        credential_locator_commitment: expected.credential_locator_commitment.clone(),
        credential_reattestation_material_digest: expected
            .credential_reattestation_material_digest
            .clone(),
        credential_reattestation_receipt_digest: expected
            .credential_reattestation_receipt_digest
            .clone(),
        credential_reattestation_receipt_id: expected.credential_reattestation_receipt_id.clone(),
        credential_ref_scheme: "vault_ref".into(),
        credential_report_expires_at: expected.credential_report_expires_at.clone(),
        credential_sha256: DIGEST_C.into(),
        credential_size_bytes: 5,
        launch_policy_digest: expected.launch_policy_digest.clone(),
        logical_adapter_id: expected.logical_adapter_id.clone(),
        profile_digest: expected.profile_digest.clone(),
        profile_id: expected.profile_id.clone(),
        provider_binding_digest: expected.provider_binding_digest.clone(),
        provider_binding_id: expected.provider_binding_id.clone(),
        provider_id: expected.provider_id.clone(),
        provider_owner_account_id: expected.provider_owner_account_id.clone(),
        purpose: RUNTIME_BUNDLE_PURPOSE.into(),
        release_version: expected.release_version.clone(),
        schema: RUNTIME_BUNDLE_SCHEMA.into(),
    }
}

fn assert_invalid_authority(
    result: Result<ExternalPoolAdapterRuntimeBundleManifest, ExternalPoolAdapterRuntimeBundleError>,
) {
    assert!(matches!(
        result,
        Err(ExternalPoolAdapterRuntimeBundleError::InvalidAuthority)
    ));
}

#[test]
fn canonical_manifest_round_trips_against_exact_expected_roots() {
    let expected = expected();
    let raw = serde_json::to_vec(&manifest(&expected)).expect("serialize canonical manifest");

    let parsed = parse_and_validate_manifest(&raw, &expected).expect("validate canonical manifest");

    assert_eq!(parsed.bundle_generation, 1);
    assert_eq!(parsed.config_size_bytes, 4);
    assert_eq!(parsed.credential_size_bytes, 5);
}

#[test]
fn non_canonical_unknown_and_mismatched_manifests_fail_closed() {
    let expected = expected();
    let canonical = serde_json::to_vec(&manifest(&expected)).expect("serialize canonical manifest");

    let mut trailing_newline = canonical.clone();
    trailing_newline.push(b'\n');
    assert_invalid_authority(parse_and_validate_manifest(&trailing_newline, &expected));

    let mut unknown = canonical.clone();
    unknown.pop();
    unknown.extend_from_slice(br#","unknown":true}"#);
    assert_invalid_authority(parse_and_validate_manifest(&unknown, &expected));

    let mut wrong_expected = expected.clone();
    wrong_expected.provider_id = "provider-2".into();
    assert_invalid_authority(parse_and_validate_manifest(&canonical, &wrong_expected));
}

#[test]
fn manifest_rejects_zero_or_oversized_sensitive_material() {
    let expected = expected();
    let mut zero_config = manifest(&expected);
    zero_config.config_size_bytes = 0;
    let raw = serde_json::to_vec(&zero_config).expect("serialize zero config manifest");
    assert_invalid_authority(parse_and_validate_manifest(&raw, &expected));

    let mut oversized_credential = manifest(&expected);
    oversized_credential.credential_size_bytes = MAX_CREDENTIAL_BYTES + 1;
    let raw = serde_json::to_vec(&oversized_credential).expect("serialize oversized manifest");
    assert_invalid_authority(parse_and_validate_manifest(&raw, &expected));
}
