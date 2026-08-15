use sha2::{Digest, Sha256};

const COMPUTE_FEDERATION_MOD: &str = include_str!("mod.rs");
const CAPSULE_FACADE: &str = include_str!("external_pool_adapter_entrypoint_capsule.rs");
const CAPSULE_TYPES: &str = include_str!("external_pool_adapter_entrypoint_capsule/types.rs");
const CAPSULE_POLICY: &str = include_str!("external_pool_adapter_entrypoint_capsule/policy.rs");
const CAPSULE_ELF: &str = include_str!("external_pool_adapter_entrypoint_capsule/elf.rs");
const CAPSULE_LINUX: &str = include_str!("external_pool_adapter_entrypoint_capsule/linux.rs");
const LAUNCH_IMAGE: &str = include_str!("external_pool_adapter_entrypoint_capsule/launch_image.rs");
const LAUNCH_IMAGE_IO: &str =
    include_str!("external_pool_adapter_entrypoint_capsule/launch_image_io.rs");
const INSTALLATION_TYPES: &str = include_str!("external_pool_adapter_installation/types.rs");
const INSTALLATION_AUDIT: &str =
    include_str!("external_pool_adapter_installation/filesystem/audit.rs");
const STORE_FACADE: &str = include_str!("../store/compute_external_pool_adapter_runtime_bundle.rs");
const STORE_CURRENT: &str =
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/current.rs");
const STORE_PROBE: &str =
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/probe_preparation.rs");
const STORE_TYPES: &str =
    include_str!("../store/compute_external_pool_adapter_runtime_bundle/types.rs");
const V250_CURRENT: &str =
    include_str!("../store/compute_external_pool_adapter_vulnerability_reattestation/current.rs");
const V252_CURRENT: &str =
    include_str!("../store/compute_external_pool_adapter_sandbox_reattestation/current.rs");
const V254_FENCES: &str = include_str!(
    "../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);

const POLICY_MATERIAL: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ENTRYPOINT-CAPSULE-POLICY-V1\0revision=1\0linux\0x86_64\0elf64-le\0et_exec\0static-no-interp-no-dynamic\0no-wx\0sealed-memfd-v1";
const POLICY_DIGEST: &str = "710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f";

#[test]
fn capsule_companion_policy_is_exact_ephemeral_and_not_a_v255_reinterpretation() {
    assert_eq!(POLICY_MATERIAL.len(), 146);
    assert_eq!(hex::encode(Sha256::digest(POLICY_MATERIAL)), POLICY_DIGEST);
    assert!(CAPSULE_POLICY.contains(
        r#"b"ELON-EXTERNAL-POOL-ADAPTER-ENTRYPOINT-CAPSULE-POLICY-V1\0revision=1\0linux\0x86_64\0elf64-le\0et_exec\0static-no-interp-no-dynamic\0no-wx\0sealed-memfd-v1""#
    ));
    for required in [
        "external_pool_adapter_entrypoint_capsule_policy_v1",
        "ENTRYPOINT_CAPSULE_POLICY_REVISION: u64 = 1",
        "host_os: \"linux\"",
        "host_arch: \"x86_64\"",
        "binary_format: \"elf64-le\"",
        "executable_type: \"et_exec\"",
        "linking_policy: \"static-no-interp-no-dynamic\"",
        "segment_policy: \"no-wx\"",
        "materialization: \"sealed-memfd-v1\"",
        "Sha256::digest(ENTRYPOINT_CAPSULE_POLICY_DOMAIN)",
    ] {
        assert!(
            CAPSULE_POLICY.contains(required),
            "missing exact capsule policy field {required}"
        );
    }
    assert!(STORE_PROBE.contains("external_pool_adapter_entrypoint_capsule_policy_root"));
    assert!(STORE_PROBE.contains("policy.policy_digest != capsule.policy_digest()"));
    assert!(!STORE_PROBE.contains("executable_verification_status ="));
    assert!(!STORE_PROBE.contains("launch_policy_digest ="));
}

#[test]
fn capsule_is_store_private_linux_x86_64_and_handle_sourced() {
    for required in [
        "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]",
        "#[cfg(not(all(target_os = \"linux\", target_arch = \"x86_64\")))]",
        "ExternalPoolAdapterEntrypointCapsuleError::Unavailable",
        "fn retained_entrypoint(&self) -> Result<(&File, &str, u64)>",
        "with_external_pool_adapter_entrypoint_capsule",
    ] {
        assert!(
            CAPSULE_FACADE.contains(required),
            "missing platform/private capsule boundary {required}"
        );
    }
    assert!(STORE_FACADE.contains(
        "#[path = \"../compute_federation/external_pool_adapter_entrypoint_capsule.rs\"]"
    ));
    assert!(!COMPUTE_FEDERATION_MOD
        .contains("pub(crate) mod external_pool_adapter_entrypoint_capsule;"));
    assert!(!COMPUTE_FEDERATION_MOD.contains("mod external_pool_adapter_entrypoint_capsule;"));
    assert!(COMPUTE_FEDERATION_MOD
        .contains("mod external_pool_adapter_entrypoint_capsule_source_contract_tests;"));

    for required in [
        "pub(super) entrypoint_index: usize",
        "pub(crate) fn retained_entrypoint(&self) -> anyhow::Result<(&File, &str, u64)>",
        ".get(self.entrypoint_index)",
        "expected.role != ARTIFACT_PACKAGE_ENTRYPOINT_ROLE",
        "expected.path != self.binding.entrypoint_path",
        "expected.sha256 != self.binding.entrypoint_sha256",
        "expected.size_bytes != self.binding.entrypoint_size_bytes",
    ] {
        assert!(
            INSTALLATION_TYPES.contains(required),
            "missing retained V249 entrypoint audit {required}"
        );
    }
    assert!(INSTALLATION_AUDIT.contains("let entrypoint_index = binding"));
    assert!(INSTALLATION_AUDIT.contains("entrypoint_index,"));
    assert!(CAPSULE_LINUX.contains("let (source_file, expected_sha256, expected_size) = source"));
    assert!(CAPSULE_LINUX.contains(".retained_entrypoint()"));
    assert!(!CAPSULE_LINUX.contains("File::open"));
    assert!(!CAPSULE_LINUX.contains("OpenOptions"));
    assert!(!CAPSULE_LINUX.contains("tempfile"));
}

#[test]
fn elf_and_memfd_gate_are_fail_closed_and_byte_exact() {
    for required in [
        "ELFCLASS64",
        "ELFDATA2LSB",
        "matches!(header[7], ELFOSABI_SYSV | ELFOSABI_LINUX)",
        "header[8] != 0",
        "header[9..16].iter().any(|byte| *byte != 0)",
        "u16_at(&header, 16) != ET_EXEC",
        "u16_at(&header, 18) != EM_X86_64",
        "program_count == 0 || program_count > MAX_PROGRAM_HEADERS",
        "const X86_64_PAGE_BYTES: u64 = 4096",
        "matches!(kind, PT_INTERP | PT_DYNAMIC)",
        "flags & PF_X != 0 && flags & PF_W != 0",
        "kind == PT_GNU_STACK && flags & PF_X != 0",
        "segment_alignment > 1 && !segment_alignment.is_power_of_two()",
        "memory_size == 0",
        "file_size == 0",
        "memory_size < file_size",
        "virtual_address % X86_64_PAGE_BYTES != file_offset % X86_64_PAGE_BYTES",
        "!alignment.is_power_of_two()",
        "file_offset < end && start < file_end",
        "let mapped_start = virtual_address & !(X86_64_PAGE_BYTES - 1)",
        "if virtual_address <= previous",
        "previous_load_virtual_address = Some(virtual_address)",
        "let mapped_end = memory_end",
        "mapped_end <= mapped_start",
        "mapped_start < end && start < mapped_end",
        "memory_ranges.push((mapped_start, mapped_end))",
        "!executable_load || !executable_entry",
    ] {
        assert!(
            CAPSULE_ELF.contains(required),
            "missing ELF gate {required}"
        );
    }

    for required in [
        "libc::memfd_create",
        "libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING",
        "libc::fcntl(file.as_raw_fd(), libc::F_GETFD)",
        "descriptor_flags & libc::FD_CLOEXEC == 0",
        "byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')",
        "hex::decode_to_slice(expected_sha256, &mut expected_digest)",
        "const SOURCE_MODE: u32 = 0o600",
        "const CAPSULE_MODE: u32 = 0o500",
        "libc::fchmod(capsule.as_raw_fd(), CAPSULE_MODE)",
        "libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL",
        "libc::fcntl(capsule.as_raw_fd(), libc::F_GET_SEALS)",
        "observed != REQUIRED_SEALS",
        "identity.links != 1",
        "identity.links != 0",
        "source_before == source_after",
        "capsule_before == capsule_after",
        "copied_digest == expected_digest",
        "capsule_digest == expected_digest",
        "source_after_digest == expected_digest",
        "reject_extra_byte(source, expected_size)?",
        "reject_extra_byte(file, expected_size)?",
    ] {
        assert!(
            CAPSULE_LINUX.contains(required),
            "missing exact memfd custody rule {required}"
        );
    }
    assert!(!CAPSULE_LINUX.contains("Vec<"));
    assert!(!CAPSULE_LINUX.contains("std::process"));
    assert!(!CAPSULE_LINUX.contains("Command::"));
}

#[test]
fn v267_derives_a_second_sealed_launch_image_without_rewriting_v257_policy() {
    for required in [
        "mod launch_image;",
        "mod launch_image_io;",
        "let launch = derive_launch_image(&source_capsule)?;",
        "sealed_image: source_capsule",
        "launch_image: launch.file",
        "launch_sha256: launch.sha256",
        "launch_size_bytes: launch.size_bytes",
        "&self.launch_image",
    ] {
        assert!(
            format!("{CAPSULE_FACADE}\n{CAPSULE_LINUX}\n{CAPSULE_TYPES}").contains(required),
            "missing V267 launch custody rule {required}"
        );
    }
    for required in [
        "parse_static_elf64_x86_64",
        ".checked_add(1)",
        "stub_header(stub_vaddr, stub_file_size, stub_memory_size)",
        "rewrite_elf_header(&mut elf, stub_entry, headers.len())",
        "rewrite_phdr_header(&mut headers, stub_vaddr)",
        "copy_relocated_ranges",
        "let delta = if target_residue >= cursor_residue",
        ".checked_add(delta)",
        "libc::PR_SET_DUMPABLE",
        "libc::PR_GET_DUMPABLE",
        "0xe7, 0x00, 0x00, 0x00",
        "0x0f, 0x05, 0x0f, 0x0b",
        "require_source_custody",
        "require_launch_custody",
        "F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL",
    ] {
        assert!(
            format!("{LAUNCH_IMAGE}\n{LAUNCH_IMAGE_IO}").contains(required),
            "missing V267 launch image rule {required}"
        );
    }
    assert!(CAPSULE_POLICY.contains("ENTRYPOINT_CAPSULE_POLICY_REVISION: u64 = 1"));
    assert!(!CAPSULE_POLICY.contains("entrypoint_capsule_policy_v2"));
    assert!(CAPSULE_TYPES.contains("pub(super) sealed_image: File"));
    assert!(CAPSULE_TYPES.contains("pub(super) launch_image: File"));
    assert!(CAPSULE_TYPES.contains("fn launch_sha256(&self) -> &str"));
    assert!(CAPSULE_TYPES.contains("fn launch_size_bytes(&self) -> u64"));
    assert!(!LAUNCH_IMAGE.contains("std::process"));
    assert!(!LAUNCH_IMAGE.contains("TcpStream"));
    assert!(!LAUNCH_IMAGE.contains("execve"));
}

#[test]
fn store_owns_one_checked_at_snapshot_and_current_head_selection() {
    let wrapper_header = source_header(
        STORE_PROBE,
        "with_current_external_pool_adapter_probe_preparation_authority",
    );
    for required in [
        "&self",
        "profile_id: &str",
        "prepared: PreparedExternalPoolAdapterInstallation",
        "bundle_root: &ExternalPoolAdapterRuntimeBundleRoot",
        "CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>",
        ") -> Result<bool>",
    ] {
        assert!(
            wrapper_header.contains(required),
            "wrapper drifted: {required}"
        );
    }
    assert!(!wrapper_header.contains("checked_at"));
    assert!(!wrapper_header.contains("Transaction"));
    assert!(!wrapper_header.contains("receipt"));

    for required in [
        "transaction_with_behavior(TransactionBehavior::Immediate)",
        "Utc::now().to_rfc3339_opts",
        "drop(transaction);",
        "return Ok(false)",
        "current_external_pool_adapter_runtime_bundle_authority_on",
        "current_external_pool_adapter_vulnerability_reattestation_head_authority_on",
        "current_external_pool_adapter_sandbox_reattestation_head_authority_on",
        "select_current_probe_preparation_roots_on(&transaction, &bundle, &checked_at)",
        "audit_same_checked_at(",
        "runtime_launch_entrypoint_path_digest(&installed.entrypoint_path)",
        "recheck_callback_freshness(bundle, selected)?",
        "transaction.commit()?",
    ] {
        assert!(
            STORE_PROBE.contains(required),
            "missing Store-selected preparation rule {required}"
        );
    }
    assert_eq!(STORE_PROBE.matches("bundle.revalidate()?").count(), 3);
    assert_eq!(STORE_PROBE.matches("checked_at,").count(), 4);
    assert!(STORE_CURRENT
        .contains("current_external_pool_adapter_credential_reattestation_head_authority_on"));
    assert!(V250_CURRENT
        .contains("current_external_pool_adapter_vulnerability_reattestation_head_authority_on"));
    assert!(V252_CURRENT
        .contains("current_external_pool_adapter_sandbox_reattestation_head_authority_on"));
    assert!(
        V252_CURRENT.contains("external_pool_adapter_vulnerability_reattestation_currentness_on")
    );
    assert!(V252_CURRENT.contains("vulnerability_is_exact_current"));
}

#[test]
fn preparation_authority_exposes_only_fixed_non_readiness_effects() {
    let authority = STORE_TYPES
        .split_once(
            "pub(in crate::store) struct CurrentExternalPoolAdapterProbePreparationAuthority",
        )
        .expect("probe preparation authority remains defined")
        .1;
    for required in [
        "fn preparation_effect(&self) -> &'static str",
        "fn probe_observed(&self) -> bool",
        "fn runtime_launch_ready(&self) -> bool",
        "fn activation_ready(&self) -> bool",
    ] {
        assert!(
            authority.contains(required),
            "missing narrow getter {required}"
        );
    }
    assert_eq!(authority.matches("pub(in crate::store) fn ").count(), 4);
    for forbidden in [
        "fn sealed_entrypoint",
        "fn sealed_image",
        "fn with_sensitive_bytes",
        "fn checked_at",
        "fn receipt",
        "fn transaction",
        "fn config_sha256",
        "fn credential_sha256",
        "AsRawFd",
        "RawFd",
    ] {
        assert!(
            !authority.contains(forbidden),
            "authority leaks forbidden surface {forbidden}"
        );
    }
    for required in [
        "ENTRYPOINT_CAPSULE_EFFECT: &str = \"materialized_ephemeral\"",
        "PROBE_OBSERVED: bool = false",
        "RUNTIME_LAUNCH_READY: bool = false",
        "ACTIVATION_READY: bool = false",
    ] {
        assert!(
            CAPSULE_TYPES.contains(required),
            "effect drifted: {required}"
        );
    }
    assert!(!CAPSULE_TYPES.contains("derive(Clone)]\npub(in super::super) struct Prepared"));
    assert!(!CAPSULE_TYPES.contains("Serialize"));
    assert!(!CAPSULE_TYPES.contains("Debug for Prepared"));
}

#[test]
fn v257_has_no_process_persistence_route_or_activation_effect() {
    let main = include_str!("../main.rs");
    let router = include_str!("../router.rs");
    let migrations = include_str!("../store_migrations.rs");
    let production = [
        CAPSULE_FACADE,
        CAPSULE_TYPES,
        CAPSULE_POLICY,
        CAPSULE_ELF,
        CAPSULE_LINUX,
        LAUNCH_IMAGE,
        LAUNCH_IMAGE_IO,
        STORE_FACADE,
        STORE_PROBE,
        STORE_TYPES,
    ]
    .concat();
    for forbidden in [
        "std::process",
        "tokio::process",
        "Command::",
        "TcpStream",
        "TcpListener",
        "reqwest::",
        "INSERT INTO",
        "UPDATE compute_",
        "DELETE FROM",
        "execve",
        "posix_spawn",
        "fork(",
        "clone(",
        "activate_external_pool",
    ] {
        assert!(
            !production.contains(forbidden),
            "V257 crosses its no-effect fence {forbidden}"
        );
    }
    assert!(!main.contains("external_pool_adapter_entrypoint_capsule_api"));
    assert!(!router.contains("entrypoint-capsule"));
    assert!(!router.contains("probe-preparation"));
    assert!(!migrations.contains("migration_v257"));
    assert!(!migrations.contains("(257,"));
    assert!(!STORE_FACADE.contains("pub(crate) use"));
    assert!(!STORE_FACADE.contains("pub fn "));
    assert!(!STORE_PROBE.contains("with_sensitive_bytes("));
    assert!(!CAPSULE_FACADE.contains("with_sensitive_bytes("));
    assert!(!CAPSULE_LINUX.contains("with_sensitive_bytes("));
}

#[test]
fn v254_absolute_denies_remain_byte_exact() {
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );
    assert_eq!(V254_TRIGGER_NAMES.len(), 18);
    for name in V254_TRIGGER_NAMES {
        assert!(V254_FENCES.contains(name), "missing V254 deny {name}");
    }
}

fn source_header<'a>(source: &'a str, function_name: &str) -> &'a str {
    let start = source
        .find(function_name)
        .expect("function remains present");
    let tail = &source[start..];
    let end = tail.find('{').expect("function has a body");
    &tail[..end]
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
