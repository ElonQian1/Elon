use base64::{engine::general_purpose::STANDARD, Engine as _};

use super::super::super::contract::*;

pub(super) fn valid_contracts() -> (
    SignedPrivilegedComponentInstallPlan,
    SignedPrivilegedComponentManifest,
) {
    let manifest = signed_manifest();
    let plan = SignedPrivilegedComponentInstallPlan {
        schema: SIGNED_PRIVILEGED_COMPONENT_INSTALL_PLAN_SCHEMA.to_string(),
        plan: PrivilegedComponentInstallPlan {
            schema: PRIVILEGED_COMPONENT_INSTALL_PLAN_SCHEMA.to_string(),
            plan_id: "install-plan-1".to_string(),
            component_id: WINDOWS_NAMESPACE_FENCE_COMPONENT_ID.to_string(),
            action: PRIVILEGED_COMPONENT_PLAN_ACTION_INSTALL.to_string(),
            node_version: "1.5.0".to_string(),
            node_release_identity: release_identity("1.5.0", 'b'),
            target_architecture: "x86_64".to_string(),
            target_manifest_digest: manifest.manifest_digest.clone(),
            target_release_identity: manifest.manifest.release_identity.clone(),
            target_package_digest: manifest.manifest.package.package_digest.clone(),
            target_rollback_generation: manifest.manifest.rollback_generation,
            expected_installed_manifest_digest: None,
            expected_installed_release_identity: None,
            expected_installed_rollback_generation: None,
            explicit_user_consent_required: true,
            elevation_required: true,
            requires_no_active_fences: true,
            background_install_allowed: false,
            test_signing_allowed: false,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2026-01-01T01:00:00Z".to_string(),
        },
        canonicalization: PRIVILEGED_COMPONENT_CANONICALIZATION.to_string(),
        plan_digest_algorithm: PRIVILEGED_COMPONENT_DIGEST_ALGORITHM.to_string(),
        plan_digest: digest('e'),
        signature: signature(
            PRIVILEGED_COMPONENT_INSTALL_PLAN_KEY_PURPOSE,
            "privileged-install-plan-key-1",
        ),
    };
    (plan, manifest)
}

fn signed_manifest() -> SignedPrivilegedComponentManifest {
    let driver_digest = digest('1');
    let inf_digest = digest('2');
    let catalog_digest = digest('3');
    let build_git_sha = "a".repeat(40);
    SignedPrivilegedComponentManifest {
        schema: SIGNED_PRIVILEGED_COMPONENT_MANIFEST_SCHEMA.to_string(),
        manifest: PrivilegedComponentManifest {
            schema: PRIVILEGED_COMPONENT_MANIFEST_SCHEMA.to_string(),
            component_id: WINDOWS_NAMESPACE_FENCE_COMPONENT_ID.to_string(),
            component_version: "2.3.4".to_string(),
            release_identity: format!("2.3.4+{build_git_sha}"),
            build_git_sha,
            target: PrivilegedComponentTarget {
                operating_system: "windows".to_string(),
                architecture: "x86_64".to_string(),
            },
            minifilter: WindowsMinifilterIdentity {
                backend_kind: WINDOWS_NAMESPACE_FENCE_BACKEND_KIND.to_string(),
                service_name: WINDOWS_NAMESPACE_FENCE_SERVICE_NAME.to_string(),
                filter_name: WINDOWS_NAMESPACE_FENCE_FILTER_NAME.to_string(),
                instance_name: WINDOWS_NAMESPACE_FENCE_INSTANCE_NAME.to_string(),
                filter_altitude: "385000".to_string(),
                communication_port_name: WINDOWS_NAMESPACE_FENCE_PORT_NAME.to_string(),
                supported_filesystems: vec!["ntfs".to_string(), "refs".to_string()],
                single_client_connection_required: true,
                reject_unload_with_active_grants: true,
            },
            protocol: PrivilegedComponentProtocol {
                protocol_id: WINDOWS_NAMESPACE_FENCE_PROTOCOL_ID.to_string(),
                protocol_revision: WINDOWS_NAMESPACE_FENCE_PROTOCOL_REVISION,
                wire_magic_ascii: WINDOWS_NAMESPACE_FENCE_PROTOCOL_MAGIC.to_string(),
                wire_major_revision: WINDOWS_NAMESPACE_FENCE_WIRE_MAJOR_REVISION,
                wire_minor_revision: WINDOWS_NAMESPACE_FENCE_WIRE_MINOR_REVISION,
                wire_byte_order: WINDOWS_NAMESPACE_FENCE_WIRE_BYTE_ORDER.to_string(),
                wire_schema_sha256: WINDOWS_NAMESPACE_FENCE_WIRE_SCHEMA_SHA256.to_string(),
                driver_build_digest: driver_digest.clone(),
                required_feature_mask: WINDOWS_NAMESPACE_FENCE_REQUIRED_FEATURE_MASK,
                commands: vec![
                    "describe_session".to_string(),
                    "acquire_fence".to_string(),
                    "query_fence".to_string(),
                    "release_fence".to_string(),
                ],
            },
            package: PrivilegedComponentPackage {
                media_type: "application/zip".to_string(),
                archive_format: "zip".to_string(),
                digest_algorithm: PRIVILEGED_COMPONENT_DIGEST_ALGORITHM.to_string(),
                package_digest: digest('4'),
                package_size_bytes: 50,
                unpacked_size_bytes: 60,
                files: vec![
                    package_file(
                        PrivilegedComponentFileRole::DriverBinary,
                        WINDOWS_NAMESPACE_FENCE_DRIVER_FILE,
                        driver_digest,
                        10,
                    ),
                    package_file(
                        PrivilegedComponentFileRole::DriverInf,
                        WINDOWS_NAMESPACE_FENCE_INF_FILE,
                        inf_digest,
                        20,
                    ),
                    package_file(
                        PrivilegedComponentFileRole::DriverCatalog,
                        WINDOWS_NAMESPACE_FENCE_CATALOG_FILE,
                        catalog_digest.clone(),
                        30,
                    ),
                ],
            },
            windows_signing: WindowsDriverSigningPolicy {
                catalog_relative_path: WINDOWS_NAMESPACE_FENCE_CATALOG_FILE.to_string(),
                catalog_digest_algorithm: PRIVILEGED_COMPONENT_DIGEST_ALGORITHM.to_string(),
                catalog_digest,
                expected_catalog_publisher: "Yilong First Party".to_string(),
                expected_catalog_certificate_sha256: digest('5'),
                microsoft_kernel_trust_required: true,
                test_signing_allowed: false,
            },
            node_compatibility: PrivilegedComponentNodeVersionRange {
                minimum_node_version: "1.0.0".to_string(),
                maximum_node_version: "2.0.0".to_string(),
            },
            rollback_generation: 7,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: "2026-06-01T00:00:00Z".to_string(),
        },
        canonicalization: PRIVILEGED_COMPONENT_CANONICALIZATION.to_string(),
        manifest_digest_algorithm: PRIVILEGED_COMPONENT_DIGEST_ALGORITHM.to_string(),
        manifest_digest: digest('d'),
        signature: signature(
            PRIVILEGED_COMPONENT_RELEASE_KEY_PURPOSE,
            "privileged-release-key-1",
        ),
    }
}

fn signature(key_purpose: &str, signing_key_id: &str) -> PrivilegedComponentSignature {
    PrivilegedComponentSignature {
        algorithm: PRIVILEGED_COMPONENT_SIGNATURE_ALGORITHM.to_string(),
        key_purpose: key_purpose.to_string(),
        signing_key_id: signing_key_id.to_string(),
        signature_base64: STANDARD.encode([0_u8; 64]),
    }
}

fn package_file(
    role: PrivilegedComponentFileRole,
    relative_path: &str,
    digest: String,
    size_bytes: i64,
) -> PrivilegedComponentPackageFile {
    PrivilegedComponentPackageFile {
        role,
        relative_path: relative_path.to_string(),
        digest_algorithm: PRIVILEGED_COMPONENT_DIGEST_ALGORITHM.to_string(),
        digest,
        size_bytes,
    }
}

fn digest(value: char) -> String {
    value.to_string().repeat(64)
}

fn release_identity(version: &str, git_sha_digit: char) -> String {
    format!("{version}+{}", git_sha_digit.to_string().repeat(40))
}
