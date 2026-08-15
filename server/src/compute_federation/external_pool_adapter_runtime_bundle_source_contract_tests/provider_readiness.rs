use super::{
    FILESYSTEM, LINUX, LOCKED_BYTES, PROVIDER_READINESS_TYPES, STORE_RUNTIME,
    STORE_RUNTIME_CUSTODY, STORE_TYPES,
};

#[test]
fn provider_readiness_runtime_retains_roots_locked_keys_and_epoch_local_seals() {
    for required in [
        "retained_directory: File",
        "open_external_pool_adapter_runtime_bundle_root(&path)",
        "LinuxOpenedRuntimeBundle::open(root.retained_directory(), digest)",
        "pub(super) fn open_custody_root(",
        "fn duplicate_cloexec(",
        "libc::F_DUPFD_CLOEXEC",
        "SystemRandom::new()",
        ".fill(custody.as_mut_slice())",
    ] {
        let combined = [STORE_TYPES, FILESYSTEM, LINUX, LOCKED_BYTES].concat();
        assert!(
            combined.contains(required),
            "missing retained custody rule {required}"
        );
    }
    for required in [
        "ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_ENABLED",
        "ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_CGROUP_PARENT_PATH",
        "ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_BUNDLE_ROOT_PATH",
        "OnceLock<",
        "ExternalPoolAdapterSupervisorCgroupParent::from_operator_delegated_path",
        "ExternalPoolAdapterRuntimeBundleRoot::new(bundle_path)",
        "Mutex<LockedProviderRuntimeReadinessSecrets>",
        "unsafe impl Send for LockedProviderRuntimeReadinessSecrets",
        "runtime_bundle_identity_commitment(",
        "attests_custody_epoch_digest(",
        "attests_runtime_bundle_identity_commitment(",
        "post_cleanup_observation_commitment(",
        "remember_readiness_seal(",
        "commit_readiness_seal(",
        "attests_readiness_seal(",
        "MAX_READINESS_SEAL_TTL_MS: i64 = 15_000",
        "MAX_LIVE_READINESS_SEALS: usize = 4_096",
        "registry.prune(now)",
        ".retain(|_, seal| seal.expires_at_utc > now)",
        "verify_slices_are_equal",
        "receipt_matches & bundle_matches & observation_matches",
        "committed: false",
        "seal.committed",
        "profile_receipt.profile_id.as_bytes()",
        "profile_receipt.profile_digest.as_bytes()",
        "profile.provider_binding_id.as_bytes()",
        "profile.provider_binding_digest.as_bytes()",
        "credential_receipt.reattestation_receipt_id.as_bytes()",
        ".reattestation_receipt_digest",
    ] {
        let combined = [STORE_RUNTIME, STORE_RUNTIME_CUSTODY].concat();
        assert!(
            combined.contains(required),
            "missing process custody rule {required}"
        );
    }
    for domain in [
        "PROVIDER_RUNTIME_READINESS_CUSTODY_EPOCH_DIGEST_DOMAIN",
        "PROVIDER_RUNTIME_READINESS_BUNDLE_IDENTITY_COMMITMENT_DOMAIN",
        "PROVIDER_RUNTIME_READINESS_POST_CLEANUP_COMMITMENT_DOMAIN",
    ] {
        assert!(PROVIDER_READINESS_TYPES.contains(&format!("const {domain}: &[u8]")));
        assert!(STORE_RUNTIME_CUSTODY.contains(domain));
        assert!(!STORE_RUNTIME_CUSTODY.contains(&format!("const {domain}")));
    }
    for size in [
        "PROVIDER_RUNTIME_READINESS_HMAC_KEY_BYTES",
        "PROVIDER_RUNTIME_READINESS_CUSTODY_EPOCH_BYTES",
    ] {
        assert!(PROVIDER_READINESS_TYPES.contains(&format!("const {size}: usize")));
        assert!(STORE_RUNTIME_CUSTODY.contains(size));
        assert!(!STORE_RUNTIME_CUSTODY.contains(&format!("const {size}")));
    }
    assert!(!STORE_RUNTIME_CUSTODY.contains("PROCESS_SECRET_BYTES"));
    for forbidden_domain in [
        "elon.external_pool_adapter.provider_runtime_readiness.epoch.v1",
        "elon.external_pool_adapter.provider_runtime_readiness.bundle_identity.v1",
        "elon.external_pool_adapter.provider_runtime_readiness.post_cleanup_observation.v1",
    ] {
        assert!(!STORE_RUNTIME_CUSTODY.contains(forbidden_domain));
    }
    for forbidden in [
        "pub(in crate::store) fn keyed_commitment",
        "pub(in crate::store) fn with_commitment",
        "pub(in crate::store) fn update_field",
        "pub(in crate::store) fn update_socket_address",
    ] {
        assert!(
            !STORE_RUNTIME_CUSTODY.contains(forbidden),
            "generic custody primitive escaped: {forbidden}"
        );
    }
    let commit = STORE_RUNTIME_CUSTODY
        .split_once("fn commit_readiness_seal(")
        .unwrap()
        .1
        .split_once("fn attests_readiness_seal(")
        .unwrap()
        .0;
    assert!(commit.contains("get_mut(readiness_receipt_id)"));
    assert!(commit.contains("seal.committed = true"));
    assert!(!commit.contains(".insert("));
}
