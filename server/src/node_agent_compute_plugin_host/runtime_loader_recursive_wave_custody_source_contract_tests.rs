const HOST_MODULE: &str = include_str!("mod.rs");
const EXACT_CONTEXT: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan.rs");
const RECURSIVE_POLICY: &str = include_str!(
    "runtime_loader_load_set/launch_path_discovery/exact_context_plan/recursive_policy.rs"
);
const SYSTEM_CLOSURE: &str = include_str!("runtime_loader_load_set/resolution/system_closure.rs");
const SYSTEM_CLOSURE_DIGEST: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/digest.rs");
const ACQUISITION: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition.rs");
const ACQUISITION_CUSTODY: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition/custody.rs");
const ACQUISITION_DIGEST: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition/digest.rs");
const ACQUISITION_FAILURE: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition/failure.rs");
const ACQUISITION_VALIDATION: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/acquisition/validation.rs");
const GRANT_READY: &str = include_str!("runtime_loader_load_set/resolution/grant_ready.rs");
const GRANT_READY_VALIDATION: &str =
    include_str!("runtime_loader_load_set/resolution/grant_ready/validation.rs");

fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap_or_else(|| panic!("source start marker missing: {start}"))
        .1
        .split_once(end)
        .unwrap_or_else(|| panic!("source end marker missing: {end}"))
        .0
}

#[test]
fn recursive_policy_and_wave_acquisition_are_routed_as_private_modules() {
    assert!(
        HOST_MODULE.contains("mod runtime_loader_recursive_wave_custody_source_contract_tests;")
    );
    assert!(EXACT_CONTEXT.contains("mod recursive_policy;"));
    assert!(SYSTEM_CLOSURE.contains("mod acquisition;"));
    for load_set_seam in [
        "WindowsRecursiveWaveRequestPlan",
        "WindowsRecursiveWaveGrantAcquisitionCustody",
        "WindowsRecursiveWaveAdvanceFailureCustody",
        "TerminalWindowsRecursiveResolutionCustody",
    ] {
        assert!(
            SYSTEM_CLOSURE.contains(load_set_seam),
            "recursive seam is hidden below system_closure: {load_set_seam}"
        );
    }
    for child in [
        "mod custody;",
        "mod digest;",
        "mod failure;",
        "mod validation;",
    ] {
        assert!(
            ACQUISITION.contains(child),
            "missing acquisition child {child}"
        );
    }
}

#[test]
fn recursive_limits_are_owned_by_one_authenticated_non_cloneable_policy() {
    for contract in [
        "struct AuthenticatedWindowsRecursiveResolutionPolicy",
        "struct AuthenticatedWindowsRecursiveResolutionPolicyLimits",
        "max_wave_count",
        "max_parsed_image_count",
        "max_module_request_count",
        "max_searched_name_count",
        "max_system_image_request_count",
        "max_forwarder_hop_count",
        "policy_digest",
        "Infallible",
        "unavailable",
    ] {
        assert!(
            RECURSIVE_POLICY.contains(contract),
            "missing policy contract {contract}"
        );
    }
    assert!(EXACT_CONTEXT.contains("AuthenticatedWindowsRecursiveResolutionPolicy"));
    assert!(RECURSIVE_POLICY.contains("validate_projected_totals_before_dispatch"));
    assert!(RECURSIVE_POLICY.contains("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_LIMIT_EXCEEDED"));
    assert_eq!(
        RECURSIVE_POLICY.matches("#[derive(Clone").count(),
        1,
        "only the copyable limits value may derive Clone"
    );
    assert!(
        !RECURSIVE_POLICY.contains("impl Clone for AuthenticatedWindowsRecursiveResolutionPolicy")
    );
    assert!(!RECURSIVE_POLICY.contains("fn produce"));
    let authenticated_owner = between(
        RECURSIVE_POLICY,
        "struct AuthenticatedWindowsRecursiveResolutionPolicy\n",
        "impl AuthenticatedWindowsRecursiveResolutionPolicy",
    );
    for forbidden in ["#[derive(Clone", "Serialize", "Deserialize"] {
        assert!(
            !authenticated_owner.contains(forbidden),
            "authenticated policy must remain linear/private: {forbidden}"
        );
    }
}

#[test]
fn recursive_wave_chain_binds_a0_to_an_custody_and_parse_provenance() {
    for contract in [
        "struct WindowsRecursiveWaveAcquisitionReceipt",
        "struct SealedWindowsRecursiveResolutionAcquisitionChain",
        "A0..AN",
        "Infallible",
        "unavailable",
    ] {
        assert!(
            ACQUISITION.contains(contract),
            "missing acquisition contract {contract}"
        );
    }
    let acquisition_receipt = between(
        ACQUISITION,
        "struct WindowsRecursiveWaveAcquisitionReceipt\n",
        "/// Independently authenticated policy plus all `A0..AN` acquisition receipts.",
    );
    for coordinate in ["producer_wave_ordinal", "target_parse_wave_ordinal"] {
        assert!(
            acquisition_receipt.contains(coordinate),
            "acquisition receipt missing wave coordinate {coordinate}"
        );
    }
    for owner in [
        "struct WindowsRecursiveResolutionAccumulatedCustody",
        "struct WindowsRecursiveWaveRequestCustody",
        "struct WindowsRecursiveWaveGrantAcquisitionCustody",
        "struct WindowsRecursiveWaveCandidateAcquisitionCustody",
        "struct WindowsRecursiveWaveLeaseAcquisitionCustody",
        "struct WindowsRecursiveWaveSameOwnerParseCustody",
        "struct WindowsRecursiveWaveCompletedCustody",
        "struct TerminalWindowsRecursiveResolutionCustody",
    ] {
        assert!(
            ACQUISITION_CUSTODY.contains(owner),
            "missing recursive owner {owner}"
        );
    }
    assert!(
        ACQUISITION_CUSTODY.contains("pub(super) fn validate_policy_limits_before_first_dispatch")
    );
    assert!(ACQUISITION_CUSTODY.contains("projected_next_frontier_parse_receipt_count"));
    assert!(ACQUISITION_FAILURE.contains("struct WindowsRecursiveWaveAdvanceFailureCustody"));
    for failure_contract in [
        "DefinitiveRejected",
        "OutcomeUncertain",
        "returned_positive",
        "returned_negative",
        "returned_transport_bytes",
    ] {
        assert!(
            ACQUISITION_FAILURE.contains(failure_contract),
            "failure custody missing retained outcome {failure_contract}"
        );
    }
    assert!(ACQUISITION_DIGEST.contains("WindowsRecursiveWaveAcquisitionReceipt"));
    assert!(ACQUISITION_DIGEST
        .contains("elon.compute_plugin.windows_recursive_wave_acquisition_receipt.v1"));
    assert!(ACQUISITION_DIGEST
        .contains("elon.compute_plugin.windows_recursive_resolution_acquisition_chain.v1"));
    assert!(SYSTEM_CLOSURE_DIGEST
        .contains("elon.compute_plugin.windows_recursive_image_parse_receipt.v2"));
    assert!(SYSTEM_CLOSURE_DIGEST
        .contains("elon.compute_plugin.windows_recursive_resolution_closure.v2"));
    assert!(ACQUISITION_VALIDATION.contains("WindowsRecursiveWaveAcquisitionReceipt"));

    let parse_receipt = between(
        SYSTEM_CLOSURE,
        "struct WindowsPostLeaseSystemImageParseReceipt",
        "pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveResolutionWavePlan",
    );
    assert!(parse_receipt.contains("producer_acquisition_receipt_ordinal"));

    for source in [ACQUISITION, ACQUISITION_CUSTODY, ACQUISITION_FAILURE] {
        assert!(source.contains("Infallible"));
        assert!(source.contains("unavailable"));
        for forbidden in ["Clone", "Serialize", "Deserialize", "fn produce"] {
            assert!(
                !source.contains(forbidden),
                "recursive linear custody exposes forbidden authority: {forbidden}"
            );
        }
    }
}

#[test]
fn real_system_candidates_enter_only_post_grant_custody() {
    let grant_ready_owners = between(
        GRANT_READY,
        "struct GrantReadyWindowsRunnerMovableOwnerSet",
        "/// Exact terminal/disposition plan plus all linear external owners",
    );
    assert!(grant_ready_owners.contains("external_search_directories"));
    assert!(!grant_ready_owners.contains("pending_system_image_candidates"));

    let post_grant = between(
        GRANT_READY,
        "struct GrantAcquiredWindowsRunnerResolutionLeaseCustody<'root>",
        "/// Lineage left after all movable external directories/candidates",
    );
    assert!(post_grant.contains("pending_system_image_candidates"));
    assert!(post_grant.contains("WindowsGrantReadyResolvedSystemImageCandidateCustody"));
    assert!(GRANT_READY.contains("candidate_binding_digest: String"));

    let grant_ready_validation = between(
        GRANT_READY_VALIDATION,
        "impl GrantReadyWindowsRunnerMovableOwnerSet",
        "impl GrantAcquiredWindowsRunnerResolutionLeaseCustody<'_>",
    );
    assert!(grant_ready_validation.contains("external_search_directories"));
    assert!(!grant_ready_validation.contains("pending_system_image_candidates"));
    assert!(GRANT_READY_VALIDATION
        .contains("pub(super) fn validate_pending_system_image_candidates_after_grants(&self)"));
}
