const HOST_MODULE: &str = include_str!("mod.rs");
const EXACT_CONTEXT: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan.rs");
const LAUNCH_DISCOVERY: &str = include_str!("runtime_loader_load_set/launch_path_discovery.rs");
const POLICY: &str = include_str!(
    "runtime_loader_load_set/launch_path_discovery/exact_context_plan/recursive_policy.rs"
);
const POLICY_SIGNATURE: &str = include_str!(
    "runtime_loader_load_set/launch_path_discovery/exact_context_plan/recursive_policy/signature.rs"
);
const POLICY_CURRENTNESS: &str = include_str!(
    "runtime_loader_load_set/launch_path_discovery/exact_context_plan/recursive_policy/currentness.rs"
);
const POLICY_DIGEST: &str = include_str!(
    "runtime_loader_load_set/launch_path_discovery/exact_context_plan/recursive_policy/digest.rs"
);
const POLICY_VALIDATION: &str = include_str!(
    "runtime_loader_load_set/launch_path_discovery/exact_context_plan/recursive_policy/validation.rs"
);
const GRANT_READY: &str = include_str!("runtime_loader_load_set/resolution/grant_ready.rs");
const A0_CURRENTNESS: &str =
    include_str!("runtime_loader_load_set/resolution/grant_ready/policy_currentness.rs");
const LOAD_SET_FAILURE: &str = include_str!("runtime_loader_load_set/failure.rs");
const SYSTEM_CONTENT_LEASE_FAILURE: &str =
    include_str!("runtime_loader_load_set/failure/system_content_lease.rs");
const RESOLUTION: &str = include_str!("runtime_loader_load_set/resolution.rs");
const SYSTEM_CLOSURE: &str = include_str!("runtime_loader_load_set/resolution/system_closure.rs");
const ACQUISITION: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition.rs");
const ACQUISITION_CUSTODY: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition/custody.rs");
const ACQUISITION_FAILURE: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition/failure.rs");
const ACQUISITION_DIGEST: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition/digest.rs");
const ACQUISITION_VALIDATION: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition/validation.rs");

fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing start marker: {start}"))
        .1
        .split_once(end)
        .unwrap_or_else(|| panic!("missing end marker: {end}"))
        .0
}

fn without_whitespace(source: &str) -> String {
    source
        .chars()
        .filter(|value| !value.is_whitespace())
        .collect()
}

#[test]
fn recursive_policy_signature_evidence_is_typed_and_uninhabited() {
    assert!(HOST_MODULE
        .contains("mod runtime_loader_recursive_policy_currentness_source_contract_tests;"));
    for module in [
        "mod currentness;",
        "mod digest;",
        "mod signature;",
        "mod validation;",
    ] {
        assert!(POLICY.contains(module));
    }
    assert!(EXACT_CONTEXT.contains("WindowsRecursivePolicyDispatchAuthorization"));
    assert!(LAUNCH_DISCOVERY.contains("WindowsRecursivePolicyDispatchAuthorization"));
    assert!(POLICY_SIGNATURE.contains("struct SignedWindowsRecursiveResolutionPolicyEnvelope"));
    assert!(POLICY_SIGNATURE.contains("struct WindowsRecursivePolicySignatureVerificationReceipt"));
    assert!(POLICY_SIGNATURE.contains("_signature_verifier_backend_unavailable: Infallible"));
    for suite in [
        "canonicalization",
        "digest_algorithm",
        "signature_algorithm",
        "signature_domain",
        "signature_material_digest",
        "signature_bytes_digest",
        "signature_message_digest",
        "control_key_record_digest",
        "control_public_key_spki_digest",
        "signing_control_keyring_generation",
    ] {
        assert!(POLICY_SIGNATURE.contains(suite));
    }
    assert!(POLICY
        .contains("signature_verification: WindowsRecursivePolicySignatureVerificationReceipt"));
    assert!(POLICY_DIGEST.contains("ELON_WINDOWS_AUTHENTICATED_RECURSIVE_POLICY_BINDING_V2"));
    assert!(POLICY_DIGEST.contains("ELON_WINDOWS_RECURSIVE_POLICY_SIGNATURE_BYTES_V1"));
    assert!(POLICY_DIGEST.contains("ELON_WINDOWS_RECURSIVE_POLICY_SIGNATURE_MESSAGE_V1"));
    assert!(POLICY_VALIDATION.contains("POLICY_SIGNATURE_DOMAIN"));
    assert!(POLICY_VALIDATION.contains("validate_policy_source(policy)?"));

    let signature_material = between(
        POLICY_DIGEST,
        "pub(super) fn signature_material_digest(",
        "pub(super) fn signature_bytes_digest(",
    );
    assert!(!signature_material.contains("envelope.signature_bytes"));
    assert!(!signature_material.contains("envelope.signature_bytes_digest"));
    assert!(!signature_material.contains("envelope.signed_envelope_digest"));
    let envelope_digest = between(
        POLICY_DIGEST,
        "pub(super) fn signed_envelope_digest(",
        "/// Digest of the exact unsigned JCS envelope material.",
    );
    assert!(envelope_digest.contains("signature_material_digest"));
    assert!(envelope_digest.contains("signature_bytes_digest"));
    let verification_message = between(
        POLICY_DIGEST,
        "pub(super) fn signature_verification_message(",
        "pub(super) fn signature_message_digest(",
    );
    for assembly in [
        "signature_material_digest(envelope)?",
        "envelope.signature_domain.as_bytes()",
        "message.push(0)",
        "message.extend_from_slice(&material_digest)",
    ] {
        assert!(verification_message.contains(assembly));
    }
}

#[test]
fn point_of_use_currentness_binds_signer_scope_time_and_exact_dispatch() {
    let authorization = between(
        POLICY_CURRENTNESS,
        "struct WindowsRecursivePolicyDispatchAuthorization",
        "impl WindowsRecursivePolicyDispatchAuthorization",
    );
    for binding in [
        "authenticated_recursive_policy_digest",
        "signature_verification_receipt_digest",
        "policy_scope_digest",
        "policy_generation",
        "control_key_id",
        "control_key_record_digest",
        "control_public_key_spki_digest",
        "active_control_key_record_digest",
        "active_control_public_key_spki_digest",
        "control_key_non_revocation_receipt_digest",
        "policy_scope_current_generation",
        "trusted_now_ms",
        "trusted_time_attestation_sequence",
        "anti_rollback_receipt_digest",
        "acquisition_receipt_ordinal",
        "producer_wave_ordinal",
        "input_custody_digest",
        "pre_dispatch_plan_evidence_digest",
        "authorization_nonce_digest",
        "_policy_currentness_backend_unavailable: Infallible",
    ] {
        assert!(
            authorization.contains(binding),
            "missing currentness binding: {binding}"
        );
    }
    for forbidden in ["Clone", "Copy", "Serialize", "Deserialize", "Default"] {
        assert!(!authorization.contains(forbidden));
    }
    assert!(POLICY_VALIDATION.contains("observed_control_keyring_generation"));
    assert!(POLICY_VALIDATION.contains("policy_scope_current_generation"));
    assert!(POLICY_VALIDATION.contains("active_control_public_key_spki_digest"));
    assert!(POLICY_VALIDATION.contains("trusted_now_ms >= authorization.policy_not_after_ms"));
    assert!(POLICY_CURRENTNESS.contains("pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn canonical_material"));
}

#[test]
fn a0_and_ak_require_currentness_after_whole_plan_validation() {
    assert!(GRANT_READY.contains("mod policy_currentness;"));
    assert!(A0_CURRENTNESS
        .contains("struct PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite"));
    assert!(A0_CURRENTNESS.contains("self.grant_ready.validate_whole()?"));
    assert!(A0_CURRENTNESS
        .contains(".validate_against(&self.grant_ready.borrow_preliminary_requests())?"));
    assert!(A0_CURRENTNESS.contains("base_pre_dispatch_plan_evidence_digest"));
    let a0_compact = without_whitespace(A0_CURRENTNESS);
    let a0_whole = a0_compact
        .find("self.grant_ready.validate_whole()?")
        .unwrap();
    let a0_policy = a0_compact
        .find(".validate_against(&self.grant_ready.borrow_preliminary_requests())?")
        .unwrap();
    let a0_evidence = a0_compact
        .find("base_pre_dispatch_plan_evidence_digest(input_custody_digest)?")
        .unwrap();
    let a0_authorization = a0_compact
        .find("self.policy_dispatch_authorization.validate_against(&self.authenticated_recursive_policy,0,0,input_custody_digest,&pre_dispatch_plan_evidence_digest,")
        .unwrap();
    assert!(a0_whole < a0_policy && a0_policy < a0_evidence && a0_evidence < a0_authorization);
    assert!(LOAD_SET_FAILURE.contains("PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite"));
    assert!(
        !LOAD_SET_FAILURE.contains("_grant_ready: GrantReadyWindowsRunnerResolutionPrerequisite")
    );

    assert!(SYSTEM_CLOSURE.contains("PolicyCurrentnessPendingWindowsRecursiveWaveGrantCustody"));
    let pending = between(
        ACQUISITION_CUSTODY,
        "struct PolicyCurrentnessPendingWindowsRecursiveWaveGrantCustody",
        "/// The only Ak state",
    );
    assert!(pending.contains("validated_plan_evidence: WindowsRecursiveWaveDispatchPlanEvidence"));
    assert!(pending.contains("_policy_currentness_authorization_producer_unavailable: Infallible"));
    assert!(pending.contains("validate_authorization_before_first_dispatch"));
    assert!(pending.contains("recursive_pre_dispatch_plan_evidence_digest"));
    let pending_compact = without_whitespace(pending);
    assert!(pending_compact.contains("authorization.validate_against(&self.accumulated.authenticated_policy,producer_wave_ordinal,producer_wave_ordinal,&self.validated_plan_evidence.input_custody_digest,&pre_dispatch_plan_evidence_digest,"));
    let dispatch_ready = between(
        ACQUISITION_CUSTODY,
        "struct DispatchReadyWindowsRecursiveWaveGrantCustody",
        "pub(super) type WindowsRecursiveWaveGrantAcquisitionCustody",
    );
    assert!(dispatch_ready
        .contains("policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization"));
}

#[test]
fn every_acquisition_receipt_retains_full_v3_currentness_evidence() {
    let receipt = between(
        ACQUISITION,
        "struct WindowsRecursiveWaveAcquisitionReceipt",
        "/// Retained, canonical pre-dispatch evidence",
    );
    assert!(receipt
        .contains("policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization"));
    let grant_acquired = between(
        GRANT_READY,
        "struct GrantAcquiredWindowsRunnerResolutionLeaseCustody",
        "impl GrantReadyWindowsRunnerResolutionPrerequisite",
    );
    assert!(!grant_acquired.contains("AuthenticatedWindowsRecursiveResolutionPolicy"));
    assert!(!grant_acquired.contains("WindowsRecursivePolicyDispatchAuthorization"));
    let policy_current_namespace = between(
        RESOLUTION,
        "struct PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet",
        "/// Final aggregate namespace grant/session lineage",
    );
    assert!(policy_current_namespace
        .contains("namespace: PreFinalWindowsLoaderNamespaceGrantSet<'root>"));
    assert!(policy_current_namespace
        .contains("authenticated_recursive_policy: AuthenticatedWindowsRecursiveResolutionPolicy"));
    assert!(policy_current_namespace
        .contains("policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization"));
    assert!(policy_current_namespace
        .contains("_a0_policy_current_namespace_transition_unavailable: Infallible"));
    for forbidden in ["Clone", "Copy", "Serialize", "Deserialize", "Option<"] {
        assert!(!policy_current_namespace.contains(forbidden));
    }
    let accumulated = between(
        ACQUISITION_CUSTODY,
        "struct WindowsRecursiveResolutionAccumulatedCustody",
        "struct WindowsRecursivePendingSearchedNameGrantRef",
    );
    assert!(accumulated.contains("root_namespace: PreFinalWindowsLoaderNamespaceGrantSet<'root>"));
    assert!(
        accumulated.contains("authenticated_policy: AuthenticatedWindowsRecursiveResolutionPolicy")
    );
    assert!(!accumulated.contains("WindowsRecursivePolicyDispatchAuthorization"));
    for (start, end) in [
        (
            "struct DispatchReadyWindowsRecursiveWaveGrantCustody",
            "pub(super) type WindowsRecursiveWaveGrantAcquisitionCustody",
        ),
        (
            "struct WindowsRecursiveWaveCandidateAcquisitionCustody",
            "/// Lease acquisition owns",
        ),
        (
            "struct WindowsRecursiveWaveLeaseAcquisitionCustody",
            "/// Same-owner parse stage.",
        ),
        (
            "struct WindowsRecursiveWaveSameOwnerParseCustody",
            "/// One completed producer acquisition",
        ),
    ] {
        assert!(between(ACQUISITION_CUSTODY, start, end).contains(
            "policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization"
        ));
    }
    let completed = between(
        ACQUISITION_CUSTODY,
        "struct WindowsRecursiveWaveCompletedCustody",
        "/// A base-only closure",
    );
    assert!(completed.contains("acquisition_receipt: WindowsRecursiveWaveAcquisitionReceipt"));
    for retained_stage in [
        "custody: WindowsRecursiveWaveGrantAcquisitionCustody",
        "custody: WindowsRecursiveWaveCandidateAcquisitionCustody",
        "custody: WindowsRecursiveWaveLeaseAcquisitionCustody",
        "custody: WindowsRecursiveWaveSameOwnerParseCustody",
        "custody: WindowsRecursiveWaveCompletedCustody",
    ] {
        assert!(ACQUISITION_FAILURE.contains(retained_stage));
    }
    let base_failure = between(
        LOAD_SET_FAILURE,
        "struct WindowsRunnerNameGrantAcquisitionUnusableCustody",
        "impl<'root> WindowsRunnerNameGrantAcquisitionUnusableCustody",
    );
    assert!(base_failure.contains("PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite"));
    assert!(LOAD_SET_FAILURE.contains("PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet"));
    assert!(LOAD_SET_FAILURE.contains("_policy_current_namespace:"));
    assert_eq!(
        SYSTEM_CONTENT_LEASE_FAILURE
            .matches(
                "policy_current_namespace: PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet<'root>"
            )
            .count(),
        3
    );
    assert!(!SYSTEM_CONTENT_LEASE_FAILURE
        .contains("namespace_grants: PreFinalWindowsLoaderNamespaceGrantSet<'root>"));
    assert!(ACQUISITION_DIGEST.contains("windows_recursive_wave_output_custody.v3"));
    assert!(ACQUISITION_DIGEST.contains("windows_recursive_wave_acquisition_receipt.v3"));
    assert_eq!(
        ACQUISITION_DIGEST
            .matches("policy_dispatch_authorization.canonical_material()")
            .count(),
        2
    );
    let validation_compact = without_whitespace(ACQUISITION_VALIDATION);
    assert!(validation_compact.contains("receipt.policy_dispatch_authorization.validate_against("));
    assert!(validation_compact.contains(
        "authorization_nonces.insert(receipt.policy_dispatch_authorization.nonce_digest())"
    ));
    assert!(validation_compact.contains("observed_generation<previous"));
    assert!(validation_compact.contains("trusted_now_ms<previous"));
    assert!(validation_compact.contains("trusted_time_attestation_sequence<previous"));
    assert!(ACQUISITION_VALIDATION.contains("POLICY_AUTHORIZATION_NONCE_REUSED"));
    assert!(ACQUISITION_VALIDATION.contains("CONTROL_KEYRING_GENERATION_REGRESSED"));
    assert!(ACQUISITION_VALIDATION.contains("TRUSTED_TIME_REGRESSED"));
    assert!(ACQUISITION_VALIDATION.contains("TRUSTED_TIME_SEQUENCE_REGRESSED"));
}

#[test]
fn currentness_authority_exposes_no_success_constructor_or_retry_scalar() {
    for source in [
        POLICY,
        POLICY_SIGNATURE,
        POLICY_CURRENTNESS,
        A0_CURRENTNESS,
        ACQUISITION,
        ACQUISITION_CUSTODY,
        ACQUISITION_FAILURE,
        SYSTEM_CONTENT_LEASE_FAILURE,
    ] {
        let compact = without_whitespace(source);
        assert!(!compact.contains("fnnew("));
        assert!(!compact.contains("fnproduce("));
        assert!(!compact.contains("fninto_parts("));
        assert!(!compact.contains("implCloneforWindowsRecursivePolicyDispatchAuthorization"));
        assert!(!compact.contains("->DispatchReadyWindowsRecursiveWaveGrantCustody"));
    }
}
