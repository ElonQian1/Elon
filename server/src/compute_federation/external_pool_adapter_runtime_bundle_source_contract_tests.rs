use sha2::{Digest, Sha256};

const COMPUTE_FEDERATION_MOD: &str = include_str!("mod.rs");
const FILESYSTEM: &str = include_str!("external_pool_adapter_runtime_bundle/filesystem.rs");
const LINUX: &str = include_str!("external_pool_adapter_runtime_bundle/filesystem/linux.rs");
const WINDOWS: &str = include_str!("external_pool_adapter_runtime_bundle/filesystem/windows.rs");
const LOCKED_BYTES: &str = include_str!("external_pool_adapter_runtime_bundle/locked_bytes.rs");
const MANIFEST: &str = include_str!("external_pool_adapter_runtime_bundle/manifest.rs");
const STORE_FACADE: &str = include_str!("../store/compute_external_pool_adapter_runtime_bundle.rs");
const STORE_CURRENT: &str =
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/current.rs");
const STORE_TYPES: &str =
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/types.rs");
const V253_CURRENT: &str =
    include_str!("../store/compute_external_pool_adapter_credential_reattestation/current.rs");
const V254_FENCES: &str = include_str!(
    "../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);

#[test]
fn runtime_bundle_manifest_is_strict_private_and_exactly_rooted() {
    for required in [
        "#[serde(deny_unknown_fields)]",
        "compute_federation.external_pool_adapter_runtime_bundle.v1",
        "external_pool_adapter_runtime_v1",
        "raw.starts_with(&[0xef, 0xbb, 0xbf])",
        "std::str::from_utf8(raw)",
        "let canonical_matches = canonical.as_bytes() == raw",
        "(1..=9_007_199_254_740_991).contains(&self.bundle_generation)",
        "profile_id",
        "profile_digest",
        "launch_policy_digest",
        "candidate_id",
        "candidate_digest",
        "provider_binding_id",
        "provider_binding_digest",
        "provider_owner_account_id",
        "adapter_config_revision",
        "adapter_config_digest",
        "credential_ref_scheme",
        "credential_locator_commitment",
        "credential_reattestation_receipt_id",
        "credential_reattestation_receipt_digest",
        "credential_reattestation_material_digest",
        "credential_report_expires_at",
        "config_size_bytes",
        "config_sha256",
        "credential_size_bytes",
        "credential_sha256",
    ] {
        assert!(
            MANIFEST.contains(required),
            "missing strict manifest root {required}"
        );
    }
    assert!(MANIFEST.contains("MAX_CONFIG_BYTES: u64 = 1_048_576"));
    assert!(MANIFEST.contains("MAX_CREDENTIAL_BYTES: u64 = 65_536"));
    assert!(MANIFEST.contains("(1..=MAX_CONFIG_BYTES)"));
    assert!(MANIFEST.contains("(1..=MAX_CREDENTIAL_BYTES)"));
    assert!(MANIFEST.contains("bounded_opaque(&self.adapter_config_digest, 512)"));
    assert!(
        MANIFEST.contains("(1..=9_007_199_254_740_991).contains(&self.adapter_config_revision)")
    );
    assert!(!MANIFEST.contains("is_sha256(&self.adapter_config_digest)"));
}

#[test]
fn runtime_bundle_path_and_filesystem_custody_are_fail_closed() {
    for required in [
        "const MANIFEST_FILE: &str = \"manifest.jcs\"",
        "const CONFIG_FILE: &str = \"config.bin\"",
        "const CREDENTIAL_FILE: &str = \"credential.bin\"",
        "validate_profile_digest(&expected.profile_digest)",
        "platform_open_bundle(root, &expected.profile_digest)",
        "opened.revalidate()?",
        "retained_handles: opened.into_handles()",
    ] {
        assert!(
            FILESYSTEM.contains(required),
            "missing fixed custody rule {required}"
        );
    }
    for required in [
        "open_directory_at(&root_fd, \"v1\")",
        "open_directory_at(&v1, \"sha256\")",
        "open_directory_at(&sha256, &digest[..2])",
        "open_directory_at(&shard, digest)",
        "O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC",
        "libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC",
        "metadata.uid() != current_uid",
        "metadata.nlink() != 1",
        "libc::flistxattr",
        "for component in path.components()",
        "require_local_filesystem(custody_root)",
        "require_local_filesystem(&file)",
        "for directory in self.directories.iter().skip(self.custody_root_index)",
        "for file in [&self.manifest, &self.config, &self.credential]",
        "libc::fstatfs",
        "EXT_SUPER_MAGIC | XFS_SUPER_MAGIC | BTRFS_SUPER_MAGIC | TMPFS_SUPER_MAGIC => Ok(())",
        "libc::lseek(duplicate, 0, libc::SEEK_SET)",
        "fn __errno_location() -> *mut libc::c_int",
        "*__errno_location() = 0",
        "const DIRECTORY_MODE: u32 = 0o500",
        "const FILE_MODE: u32 = 0o400",
        "require_exact_entries",
        "identity(handle)? != *expected",
    ] {
        assert!(
            LINUX.contains(required),
            "missing Linux fail-closed rule {required}"
        );
    }
    for required in [
        "Prefix::Disk(letter)",
        "GetDriveTypeW(wide.as_ptr()) } != DRIVE_FIXED",
        "FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS",
        ".share_mode(FILE_SHARE_READ)",
        "observed.links != 1",
        "GetHandleInformation",
        "HANDLE_FLAG_INHERIT",
        "FILE_ATTRIBUTE_REPARSE_POINT",
        "GetFileInformationByHandleEx",
        "FileIdInfo",
        "identity(handle)? != *expected",
        "validate_protected_dacl(file)",
        "Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)",
    ] {
        assert!(
            WINDOWS.contains(required),
            "missing Windows fail-closed rule {required}"
        );
    }
    assert!(WINDOWS.contains("Windows custody remains deliberately unavailable"));
    assert!(WINDOWS.contains(
        "fn validate_protected_dacl(_file: &File) -> Result<(), ExternalPoolAdapterRuntimeBundleError> {\n    Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)"
    ));
    assert!(FILESYSTEM.contains("not(any(target_os = \"linux\", windows))"));
    assert!(FILESYSTEM.contains("Err(ExternalPoolAdapterRuntimeBundleError::UnsafeCustody)"));
    assert!(!FILESYSTEM.contains("pub(crate) fn resolve_external_pool_adapter_runtime_bundle"));
}

#[test]
fn runtime_bundle_sensitive_bytes_are_locked_borrowed_and_zeroized() {
    for required in [
        "file.read_exact(custody.as_mut_slice())",
        "libc::mmap",
        "libc::mlock",
        "libc::MADV_DONTDUMP",
        "VirtualAlloc",
        "VirtualLock",
        "self.as_mut_slice().zeroize()",
        "atomic::compiler_fence",
        "VirtualUnlock",
        "libc::munlock",
    ] {
        assert!(
            LOCKED_BYTES.contains(required),
            "missing memory custody rule {required}"
        );
    }
    assert!(!LOCKED_BYTES.contains("derive(Clone"));
    assert!(!LOCKED_BYTES.contains("Serialize"));
    assert!(!LOCKED_BYTES.contains("Debug for LockedSensitiveBytes"));
    assert!(STORE_TYPES.contains("retained_handles: Vec<std::fs::File>"));
    assert!(STORE_TYPES.contains("RUNTIME_LAUNCH_READY: bool = false"));
    assert!(STORE_TYPES.contains("RUNTIME_BUNDLE_EFFECT: &str = \"resolved_ephemeral\""));
    assert!(STORE_TYPES.contains("CONFIG_ACCESS_EFFECT: &str = \"memory_only\""));
    assert!(STORE_TYPES.contains("SECRET_ACCESS_EFFECT: &str = \"memory_only\""));
    assert!(!STORE_TYPES.contains("derive(Clone)]\npub(super) struct Prepared"));
    assert!(STORE_TYPES.contains("impl FnOnce(&[u8], &[u8]) -> Result<()>"));
    assert!(!STORE_TYPES.contains("impl FnOnce(&[u8], &[u8]) -> T"));
    assert!(MANIFEST.contains("let mut canonical = serde_json::to_string(&manifest)"));
    assert!(MANIFEST.contains("canonical.zeroize()"));
    assert!(MANIFEST.contains("&mut self.config_sha256"));
    assert!(MANIFEST.contains("&mut self.credential_sha256"));
    assert!(MANIFEST.contains("value.zeroize()"));
}

#[test]
fn store_selects_current_v253_and_v255_at_one_private_checked_at() {
    for required in [
        "&'tx Transaction<'conn>",
        "current_external_pool_adapter_runtime_launch_profile_authority_on",
        "current_external_pool_adapter_credential_reattestation_head_authority_on",
        "historical_external_pool_onboarding_application_authority_on",
        "transaction_with_behavior(TransactionBehavior::Immediate)",
        "Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)",
        "credential_ref_scheme(locator)",
        "credential_locator_commitment(locator)",
        "credential_checked_at != checked_at",
        "resolve_external_pool_adapter_runtime_bundle(bundle_root, &expected)",
        "PhantomData<&'tx Transaction<'conn>>",
        "CurrentExternalPoolAdapterRuntimeBundleAuthority",
    ] {
        assert!(
            STORE_CURRENT.contains(required) || STORE_TYPES.contains(required),
            "missing sealed Store composition {required}"
        );
    }
    assert!(V253_CURRENT
        .contains("current_external_pool_adapter_credential_reattestation_head_authority_on"));
    assert!(STORE_CURRENT.matches("checked_at,").count() >= 3);
    assert!(!STORE_FACADE.contains("checked_at: &str"));
    for forbidden in [
        "PathBuf::from(locator)",
        ".join(locator)",
        "std::env",
        "var(locator)",
        "resolve_external_pool_adapter_runtime_bundle(bundle_root, locator",
        "INSERT INTO",
        "UPDATE compute_",
        "DELETE FROM",
    ] {
        assert!(
            !STORE_CURRENT.contains(forbidden),
            "Store crosses private boundary {forbidden}"
        );
    }
    assert!(STORE_FACADE.contains("pub(in crate::store) use"));
    assert!(STORE_FACADE.contains("ExternalPoolAdapterRuntimeBundleRoot"));
    assert!(STORE_CURRENT.contains("TransactionBehavior::Immediate"));
    assert!(!STORE_FACADE.contains("pub(crate) use"));
    assert!(!COMPUTE_FEDERATION_MOD.contains("mod external_pool_adapter_runtime_bundle;"));
    assert!(!STORE_FACADE.contains("checked_at: &str"));
    assert!(
        !STORE_CURRENT.contains("with_current_external_pool_adapter_runtime_bundle_authority<T>")
    );
    assert!(STORE_CURRENT.contains(") -> Result<bool>"));
}

#[test]
fn runtime_bundle_has_no_public_route_migration_or_downstream_effect() {
    let main = include_str!("../main.rs");
    let router = include_str!("../router.rs");
    let migrations = include_str!("../store_migrations.rs");
    let domain_and_store = [
        FILESYSTEM,
        LINUX,
        WINDOWS,
        LOCKED_BYTES,
        MANIFEST,
        STORE_FACADE,
        STORE_CURRENT,
        STORE_TYPES,
    ]
    .concat();

    assert!(!STORE_FACADE.contains("pub fn "));
    assert!(!COMPUTE_FEDERATION_MOD.contains("mod external_pool_adapter_runtime_bundle;"));
    assert!(!main.contains("external_pool_adapter_runtime_bundle_api"));
    assert!(!router.contains("runtime-bundles"));
    assert!(!router.contains("runtime_bundle_resolve"));
    assert!(!migrations.contains("migration_v256"));
    assert!(!migrations.contains("(256,"));
    for forbidden in [
        "std::process::Command",
        "tokio::process",
        "TcpStream",
        "TcpListener",
        "reqwest::",
        "activate_external_pool",
        "UPDATE compute_providers",
        "INSERT INTO compute_route_",
        "INSERT INTO compute_service_actor_authorizations",
        "INSERT INTO compute_capacity_pools",
        "INSERT INTO compute_offers",
        "INSERT INTO compute_jobs",
        "INSERT INTO compute_attempt",
        "INSERT INTO compute_usage",
        "INSERT INTO compute_settlement",
    ] {
        assert!(
            !domain_and_store.contains(forbidden),
            "V256 crosses no-effect fence {forbidden}"
        );
    }
}

#[test]
fn runtime_bundle_preserves_all_v254_absolute_denies_byte_exact() {
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );
    assert_eq!(V254_TRIGGER_NAMES.len(), 18);
    for name in V254_TRIGGER_NAMES {
        assert!(
            V254_FENCES.contains(name),
            "missing V254 absolute deny {name}"
        );
    }
}

const V254_TRIGGER_NAMES: &[&str] = &[
    "v254_external_pool_provider_activation_fence",
    "v254_external_pool_provider_insert_active_fence",
    "v254_external_pool_provider_identity_update_fence",
    "v254_external_pool_provider_kind_update_fence",
    "v254_external_pool_provider_version_active_fence",
    "v254_external_pool_candidate_projection_adapter_fence",
    "v254_external_pool_candidate_projection_adapter_version_fence",
    "v254_external_pool_candidate_service_actor_fence",
    "v254_external_pool_route_credential_fence",
    "v254_external_pool_route_authorization_fence",
    "v254_external_pool_route_capability_fence",
    "v254_external_pool_route_seal_fence",
    "v254_external_pool_capacity_pool_insert_active_fence",
    "v254_external_pool_capacity_pool_update_active_fence",
    "v254_external_pool_capacity_pool_version_active_fence",
    "v254_external_pool_offer_insert_market_fence",
    "v254_external_pool_offer_update_market_fence",
    "v254_external_pool_offer_version_market_fence",
];
