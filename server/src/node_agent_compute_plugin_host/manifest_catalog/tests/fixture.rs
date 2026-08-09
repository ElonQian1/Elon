use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Duration, TimeZone, Utc};
use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
};
use sha2::{Digest, Sha256};

use super::super::{
    verify_manifest_catalog_candidate, ComputePluginManifestCatalog,
    ComputePluginManifestCatalogCandidate, ComputePluginManifestCatalogEntry,
    SignedComputePluginManifestCatalog, ValidatedComputePluginManifestCatalog,
    COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA, SIGNED_COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA,
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    keyring::{
        ComputePluginControlPlaneKeyResolver, ComputePluginKeyringBinding,
        ComputePluginPublisherKeyResolver, ResolvedComputePluginVerificationKey,
        KEY_PURPOSE_CONTROL_INSTALL_PLAN, KEY_PURPOSE_PUBLISHER_MANIFEST,
    },
    plugin_manifest::{
        ComputePluginEntrypoint, ComputePluginFilesystemScope, ComputePluginHealthCheck,
        ComputePluginHostApiRange, ComputePluginManifest, ComputePluginPackage,
        ComputePluginPackageFile, ComputePluginPermissionProfile, ComputePluginResourceLimits,
        ComputePluginSignature, ComputePluginTarget, SignedComputePluginManifest,
        COMPUTE_PLUGIN_ARCHIVE_FORMAT_ZIP, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION,
        COMPUTE_PLUGIN_MANIFEST_SCHEMA, COMPUTE_PLUGIN_PACKAGE_MEDIA_TYPE_ZIP,
        COMPUTE_PLUGIN_SIGNATURE_ALGORITHM, SIGNED_COMPUTE_PLUGIN_MANIFEST_SCHEMA,
    },
    signed_artifact_verification::{jcs_sha256_hex, ComputePluginEd25519PublicKey},
};

const TARGET_ID: &str = "windows_x86_64";
const HOST_API_PROTOCOL_ID: &str = "elon_compute_plugin_host";
const HOST_API_REVISION: u32 = 1;
const KEYRING_BUNDLE_REVISION: i64 = 3;
const PUBLISHER_ID: &str = "publisher.test";
const PUBLISHER_KEY_ID: &str = "publisher_key_1";
const CONTROL_KEY_ID: &str = "control_key_1";

pub(super) struct CatalogFixture {
    candidate: ComputePluginManifestCatalogCandidate,
    keys: TestKeyResolver,
    trusted_now: DateTime<Utc>,
    publisher_keyring: ComputePluginKeyringBinding,
    control_keyring: ComputePluginKeyringBinding,
    publisher_fingerprint: String,
    control_fingerprint: String,
}

impl CatalogFixture {
    pub(super) fn new(
        reuse_key_material: bool,
        manifest_signature_domain: &str,
        catalog_signature_domain: &str,
    ) -> Self {
        let trusted_now = Utc.with_ymd_and_hms(2026, 8, 9, 10, 0, 0).single().unwrap();
        let random = SystemRandom::new();
        let publisher_document = Ed25519KeyPair::generate_pkcs8(&random).unwrap();
        let control_document = if reuse_key_material {
            publisher_document.as_ref().to_vec()
        } else {
            Ed25519KeyPair::generate_pkcs8(&random)
                .unwrap()
                .as_ref()
                .to_vec()
        };
        let publisher_pair = Ed25519KeyPair::from_pkcs8(publisher_document.as_ref()).unwrap();
        let control_pair = Ed25519KeyPair::from_pkcs8(&control_document).unwrap();
        let publisher_keyring = ComputePluginKeyringBinding {
            revision: 5,
            digest: jcs_sha256_hex(&"publisher-keyring-v5").unwrap(),
        };
        let control_keyring = ComputePluginKeyringBinding {
            revision: 7,
            digest: jcs_sha256_hex(&"control-keyring-v7").unwrap(),
        };
        let publisher_fingerprint = fingerprint(&publisher_pair);
        let control_fingerprint = fingerprint(&control_pair);
        let signed_manifest = signed_manifest(&publisher_pair, manifest_signature_domain);
        let catalog = catalog(
            &signed_manifest,
            &publisher_fingerprint,
            &publisher_keyring,
            &control_keyring,
        );
        let signed_catalog = SignedComputePluginManifestCatalog {
            schema: SIGNED_COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA.to_string(),
            catalog_digest: jcs_sha256_hex(&catalog).unwrap(),
            signature: signature(
                &control_pair,
                CONTROL_KEY_ID,
                catalog_signature_domain,
                &jcs_sha256_hex(&catalog).unwrap(),
            ),
            catalog,
            canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
            catalog_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
        };
        let candidate = ComputePluginManifestCatalogCandidate::new(
            "catalog_request_4".to_string(),
            signed_catalog,
            vec![signed_manifest],
        )
        .unwrap();
        let keys = TestKeyResolver {
            publisher: resolved_key(
                &publisher_pair,
                publisher_keyring.clone(),
                KEY_PURPOSE_PUBLISHER_MANIFEST,
                Some(PUBLISHER_ID),
                PUBLISHER_KEY_ID,
                trusted_now.clone(),
            ),
            control: resolved_key(
                &control_pair,
                control_keyring.clone(),
                KEY_PURPOSE_CONTROL_INSTALL_PLAN,
                None,
                CONTROL_KEY_ID,
                trusted_now.clone(),
            ),
        };
        Self {
            candidate,
            keys,
            trusted_now,
            publisher_keyring,
            control_keyring,
            publisher_fingerprint,
            control_fingerprint,
        }
    }

    pub(super) fn verify(&self) -> Result<ValidatedComputePluginManifestCatalog> {
        self.verify_for_target(TARGET_ID)
    }

    pub(super) fn verify_for_target(
        &self,
        target_id: &str,
    ) -> Result<ValidatedComputePluginManifestCatalog> {
        verify_manifest_catalog_candidate(
            &self.candidate,
            target_id,
            HOST_API_PROTOCOL_ID,
            HOST_API_REVISION,
            KEYRING_BUNDLE_REVISION,
            &self.publisher_keyring,
            &self.control_keyring,
            self.trusted_now.clone(),
            &self.keys,
            &self.keys,
        )
    }

    pub(super) fn publisher_fingerprint(&self) -> &str {
        &self.publisher_fingerprint
    }

    pub(super) fn control_fingerprint(&self) -> &str {
        &self.control_fingerprint
    }
}

#[derive(Clone)]
struct TestKeyResolver {
    publisher: ResolvedComputePluginVerificationKey,
    control: ResolvedComputePluginVerificationKey,
}

impl ComputePluginPublisherKeyResolver for TestKeyResolver {
    fn resolve_publisher_key(
        &self,
        publisher_id: &str,
        signing_key_id: &str,
        expected_keyring: &ComputePluginKeyringBinding,
        _trusted_now: DateTime<Utc>,
    ) -> Result<Option<ResolvedComputePluginVerificationKey>> {
        Ok(
            (publisher_id == self.publisher.publisher_id().unwrap_or_default()
                && signing_key_id == self.publisher.signing_key_id()
                && expected_keyring == self.publisher.keyring_binding())
            .then(|| self.publisher.clone()),
        )
    }
}

impl ComputePluginControlPlaneKeyResolver for TestKeyResolver {
    fn resolve_control_plane_key(
        &self,
        signing_key_id: &str,
        expected_keyring: &ComputePluginKeyringBinding,
        _trusted_now: DateTime<Utc>,
    ) -> Result<Option<ResolvedComputePluginVerificationKey>> {
        Ok((signing_key_id == self.control.signing_key_id()
            && expected_keyring == self.control.keyring_binding())
        .then(|| self.control.clone()))
    }
}

fn signed_manifest(pair: &Ed25519KeyPair, domain: &str) -> SignedComputePluginManifest {
    let manifest = ComputePluginManifest {
        schema: COMPUTE_PLUGIN_MANIFEST_SCHEMA.to_string(),
        plugin_id: "plugin.test".to_string(),
        plugin_version: "1.0.0".to_string(),
        publisher_id: PUBLISHER_ID.to_string(),
        package: ComputePluginPackage {
            media_type: COMPUTE_PLUGIN_PACKAGE_MEDIA_TYPE_ZIP.to_string(),
            archive_format: COMPUTE_PLUGIN_ARCHIVE_FORMAT_ZIP.to_string(),
            digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
            package_digest: "1".repeat(64),
            package_size_bytes: 128,
            unpacked_size_bytes: 128,
            files: vec![ComputePluginPackageFile {
                relative_path: "bin/plugin.exe".to_string(),
                digest: "2".repeat(64),
                size_bytes: 128,
                executable: true,
            }],
        },
        host_api: ComputePluginHostApiRange {
            protocol_id: HOST_API_PROTOCOL_ID.to_string(),
            minimum_revision: 1,
            maximum_revision: 2,
        },
        task_kinds: vec!["ai_inference".to_string()],
        target: ComputePluginTarget {
            target_id: TARGET_ID.to_string(),
            operating_system: "windows".to_string(),
            architecture: "x86_64".to_string(),
            accelerator_kind: None,
            accelerator_abi: None,
            minimum_driver_versions: Vec::new(),
            requires_virtualization: false,
        },
        entrypoint: ComputePluginEntrypoint {
            entrypoint_kind: COMPUTE_PLUGIN_ENTRYPOINT_SIDECAR.to_string(),
            relative_path: "bin/plugin.exe".to_string(),
            arguments: Vec::new(),
            health_check: ComputePluginHealthCheck {
                protocol: "stdio".to_string(),
                timeout_ms: 1_000,
                interval_ms: 5_000,
                healthy_after_successes: 1,
                unhealthy_after_failures: 3,
            },
        },
        system_dependencies: Vec::new(),
        download_dependencies: Vec::new(),
        requested_resources: ComputePluginResourceLimits {
            max_cpu_millicores: 1_000,
            max_memory_bytes: 512 * 1024 * 1024,
            max_vram_bytes: 0,
            max_disk_bytes: 1024 * 1024 * 1024,
            max_processes: 1,
            max_sidecar_uptime_seconds: 3_600,
        },
        requested_permissions: ComputePluginPermissionProfile {
            allow_network_egress: false,
            allowed_egress_domains: Vec::new(),
            filesystem_scopes: vec![ComputePluginFilesystemScope::PluginPackageReadOnly],
            allow_child_processes: false,
            device_scopes: Vec::new(),
        },
        state_compatibility: None,
    };
    let manifest_digest = jcs_sha256_hex(&manifest).unwrap();
    SignedComputePluginManifest {
        schema: SIGNED_COMPUTE_PLUGIN_MANIFEST_SCHEMA.to_string(),
        signature: signature(pair, PUBLISHER_KEY_ID, domain, &manifest_digest),
        manifest,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        manifest_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
        manifest_digest,
    }
}

fn catalog(
    signed: &SignedComputePluginManifest,
    publisher_fingerprint: &str,
    publisher_keyring: &ComputePluginKeyringBinding,
    control_keyring: &ComputePluginKeyringBinding,
) -> ComputePluginManifestCatalog {
    ComputePluginManifestCatalog {
        schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA.to_string(),
        catalog_revision: 4,
        target_id: TARGET_ID.to_string(),
        host_api_protocol_id: HOST_API_PROTOCOL_ID.to_string(),
        host_api_revision: HOST_API_REVISION,
        keyring_bundle_revision: KEYRING_BUNDLE_REVISION,
        publisher_keyring: publisher_keyring.clone(),
        control_keyring: control_keyring.clone(),
        entries: vec![ComputePluginManifestCatalogEntry {
            release: ComputePluginReleaseRef {
                plugin_id: signed.manifest.plugin_id.clone(),
                plugin_version: signed.manifest.plugin_version.clone(),
                target_id: signed.manifest.target.target_id.clone(),
                manifest_digest: signed.manifest_digest.clone(),
                package_digest: signed.manifest.package.package_digest.clone(),
            },
            publisher_id: signed.manifest.publisher_id.clone(),
            signing_key_id: signed.signature.signing_key_id.clone(),
            signing_key_fingerprint: publisher_fingerprint.to_string(),
            signed_manifest_envelope_digest: jcs_sha256_hex(signed).unwrap(),
        }],
    }
}

fn signature(
    pair: &Ed25519KeyPair,
    signing_key_id: &str,
    domain: &str,
    digest: &str,
) -> ComputePluginSignature {
    let digest = hex::decode(digest).unwrap();
    let mut message = Vec::with_capacity(domain.len() + 1 + digest.len());
    message.extend_from_slice(domain.as_bytes());
    message.push(0);
    message.extend_from_slice(&digest);
    ComputePluginSignature {
        algorithm: COMPUTE_PLUGIN_SIGNATURE_ALGORITHM.to_string(),
        signing_key_id: signing_key_id.to_string(),
        signature_base64: STANDARD.encode(pair.sign(&message).as_ref()),
    }
}

fn resolved_key(
    pair: &Ed25519KeyPair,
    keyring: ComputePluginKeyringBinding,
    purpose: &str,
    publisher_id: Option<&str>,
    signing_key_id: &str,
    trusted_now: DateTime<Utc>,
) -> ResolvedComputePluginVerificationKey {
    let public_key = ComputePluginEd25519PublicKey::from_standard_base64(
        &STANDARD.encode(pair.public_key().as_ref()),
    )
    .unwrap();
    ResolvedComputePluginVerificationKey::new(
        public_key,
        keyring,
        purpose.to_string(),
        publisher_id.map(str::to_string),
        signing_key_id.to_string(),
        fingerprint(pair),
        trusted_now - Duration::hours(1),
        trusted_now + Duration::hours(1),
    )
}

fn fingerprint(pair: &Ed25519KeyPair) -> String {
    hex::encode(Sha256::digest(pair.public_key().as_ref()))
}
