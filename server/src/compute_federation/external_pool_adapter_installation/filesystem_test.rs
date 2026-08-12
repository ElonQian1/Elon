use std::io::{Cursor, Write};

use axum::body::Body;
use chrono::{Duration, SecondsFormat, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use crate::{
    compute_federation::{
        external_pool_adapter_adoption::{
            adoption_material_digest, canonical_adoption_receipt_json_and_digest,
            ExternalPoolAdapterAdoptionBinding, ExternalPoolAdapterAdoptionMaterial,
            ExternalPoolAdapterAdoptionReceipt, ADOPTION_AUTHORITY_EFFECT,
            ADOPTION_CANONICALIZATION, ADOPTION_CONFIRMATION, ADOPTION_DIGEST_ALGORITHM,
            ADOPTION_INSTALL_EFFECT, ADOPTION_NO_EFFECT, ADOPTION_RECEIPT_SCHEMA,
        },
        external_pool_adapter_artifact_package::{
            canonical_artifact_package_receipt_json_and_digest, package_material_digest,
            ExternalPoolAdapterArtifactManifest, ExternalPoolAdapterArtifactManifestFile,
            ExternalPoolAdapterArtifactPackageInspection,
            ExternalPoolAdapterArtifactPackageReceipt,
            ExternalPoolAdapterArtifactPackageReceiptMaterial, ExternalPoolAdapterArtifactRuntime,
            ARTIFACT_PACKAGE_CANONICALIZATION, ARTIFACT_PACKAGE_CONFIRMATION,
            ARTIFACT_PACKAGE_DIGEST_ALGORITHM, ARTIFACT_PACKAGE_EVIDENCE_SCOPE,
            ARTIFACT_PACKAGE_FORMAT, ARTIFACT_PACKAGE_FORMAT_EFFECT,
            ARTIFACT_PACKAGE_MANIFEST_PATH, ARTIFACT_PACKAGE_MANIFEST_SCHEMA,
            ARTIFACT_PACKAGE_NO_EFFECT, ARTIFACT_PACKAGE_RECEIPT_SCHEMA,
            ARTIFACT_PACKAGE_RUNTIME_KIND,
        },
        external_pool_adapter_artifact_source::{
            intake_quarantined_artifact_bytes, open_current_quarantined_artifact_bytes,
        },
        external_pool_adapter_release::{
            canonical_external_pool_adapter_release_capability_set_digest,
            ComputeExternalPoolAdapterReleaseCapability,
            ComputeExternalPoolAdapterReleaseVerifierIntent,
        },
    },
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::*;

#[tokio::test]
async fn filesystem_prepares_reuses_and_rejects_installed_byte_drift() {
    let data_dir = std::env::temp_dir().join(format!(
        "elon_adapter_installation_fs_{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&data_dir).unwrap();
    let fixture = package_fixture();
    let digest = hex::encode(Sha256::digest(&fixture.bytes));
    let intake =
        intake_quarantined_artifact_bytes(&data_dir, &digest, Body::from(fixture.bytes.clone()))
            .await
            .unwrap();
    assert_eq!(intake.content_address_digest(), digest);
    drop(intake);
    let artifact =
        open_current_quarantined_artifact_bytes(&data_dir, &digest, fixture.bytes.len() as u64)
            .await
            .unwrap();
    let first =
        prepare_external_pool_adapter_installation(&data_dir, artifact, target(&fixture, &digest))
            .unwrap();
    let content_digest = first.installation_content_digest().to_string();
    let binding = first.binding().clone();
    assert_eq!(first.installed_files().len(), 2);
    drop(first);

    let artifact =
        open_current_quarantined_artifact_bytes(&data_dir, &digest, fixture.bytes.len() as u64)
            .await
            .unwrap();
    let replay =
        prepare_external_pool_adapter_installation(&data_dir, artifact, target(&fixture, &digest))
            .unwrap();
    assert_eq!(replay.installation_content_digest(), content_digest);
    drop(replay);

    let root = installed_root(&data_dir, &content_digest);
    std::fs::write(root.join("bin/adapter.sh"), b"drifted").unwrap();
    assert!(matches!(
        audit_external_pool_adapter_installation(&data_dir, binding),
        Err(ExternalPoolAdapterInstallationFsError::ContentDrift)
    ));
    std::fs::remove_dir_all(data_dir).unwrap();
}

#[tokio::test]
async fn filesystem_audit_fails_closed_when_installed_tree_is_missing() {
    let data_dir = std::env::temp_dir().join(format!(
        "elon_adapter_installation_missing_{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&data_dir).unwrap();
    let fixture = package_fixture();
    let digest = hex::encode(Sha256::digest(&fixture.bytes));
    let binding = prepared_binding(&data_dir, &digest, &fixture).await;
    let root = installed_root(&data_dir, &binding.installation_content_digest);
    std::fs::remove_dir_all(root).unwrap();
    assert!(matches!(
        audit_external_pool_adapter_installation(&data_dir, binding),
        Err(ExternalPoolAdapterInstallationFsError::Missing)
    ));
    std::fs::remove_dir_all(data_dir).unwrap();
}

#[tokio::test]
async fn filesystem_audit_rejects_hardlinked_installed_files() {
    let data_dir = std::env::temp_dir().join(format!(
        "elon_adapter_installation_hardlink_{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&data_dir).unwrap();
    let fixture = package_fixture();
    let digest = hex::encode(Sha256::digest(&fixture.bytes));
    let binding = prepared_binding(&data_dir, &digest, &fixture).await;
    let installed =
        installed_root(&data_dir, &binding.installation_content_digest).join("bin/adapter.sh");
    std::fs::hard_link(&installed, data_dir.join("outside-hardlink")).unwrap();
    assert!(matches!(
        audit_external_pool_adapter_installation(&data_dir, binding),
        Err(ExternalPoolAdapterInstallationFsError::UnsafeTarget)
    ));
    std::fs::remove_dir_all(data_dir).unwrap();
}

async fn prepared_binding(
    data_dir: &std::path::Path,
    digest: &str,
    fixture: &PackageFixture,
) -> ExternalPoolAdapterInstallationBinding {
    let intake =
        intake_quarantined_artifact_bytes(data_dir, digest, Body::from(fixture.bytes.clone()))
            .await
            .unwrap();
    drop(intake);
    let artifact =
        open_current_quarantined_artifact_bytes(data_dir, digest, fixture.bytes.len() as u64)
            .await
            .unwrap();
    let prepared =
        prepare_external_pool_adapter_installation(data_dir, artifact, target(fixture, digest))
            .unwrap();
    prepared.binding().clone()
}

#[derive(Clone)]
struct PackageFixture {
    bytes: Vec<u8>,
    manifest: ExternalPoolAdapterArtifactManifest,
    manifest_json: String,
    manifest_digest: String,
    inventory_digest: String,
    total_bytes: u64,
}

#[derive(Serialize)]
struct InventoryEntry<'a> {
    path: &'a str,
    sha256: &'a str,
    size_bytes: u64,
}

fn package_fixture() -> PackageFixture {
    let entrypoint = b"#!/bin/sh\nexit 0\n".to_vec();
    let resource = b"fixture-resource".to_vec();
    let capabilities = [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ]
    .into_iter()
    .map(|id| ComputeExternalPoolAdapterReleaseCapability {
        capability_id: id.to_string(),
        capability_revision: 1,
    })
    .collect::<Vec<_>>();
    let files = vec![
        manifest_file("bin/adapter.sh", &entrypoint, "entrypoint"),
        manifest_file("resources/config.json", &resource, "resource"),
    ];
    let manifest = ExternalPoolAdapterArtifactManifest {
        schema: ARTIFACT_PACKAGE_MANIFEST_SCHEMA.to_string(),
        adapter_id: "community-external-pool".to_string(),
        release_version: "46.9.0".to_string(),
        package_format: ARTIFACT_PACKAGE_FORMAT.to_string(),
        runtime: ExternalPoolAdapterArtifactRuntime {
            kind: ARTIFACT_PACKAGE_RUNTIME_KIND.to_string(),
            entrypoint: "bin/adapter.sh".to_string(),
        },
        capability_set_digest: canonical_external_pool_adapter_release_capability_set_digest(
            &capabilities,
        )
        .unwrap(),
        supported_capabilities: capabilities,
        credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent {
            verification_kind: "signed_challenge".to_string(),
            verifier_id: "community-pool-verifier".to_string(),
            verifier_revision: 1,
            verifier_digest: "2".repeat(64),
        },
        files,
    };
    let manifest_json = canonical(&manifest);
    let manifest_digest = domain_bytes(
        b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-MANIFEST-V1",
        manifest_json.as_bytes(),
    );
    let inventory = manifest
        .files
        .iter()
        .map(|file| InventoryEntry {
            path: &file.path,
            sha256: &file.sha256,
            size_bytes: file.size_bytes,
        })
        .collect::<Vec<_>>();
    let inventory_digest = domain_json(
        b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-INVENTORY-V1",
        &inventory,
    );
    let total_bytes = manifest_json.len() as u64 + entrypoint.len() as u64 + resource.len() as u64;
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (path, bytes) in [
        (ARTIFACT_PACKAGE_MANIFEST_PATH, manifest_json.as_bytes()),
        ("bin/adapter.sh", entrypoint.as_slice()),
        ("resources/config.json", resource.as_slice()),
    ] {
        zip.start_file(path, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    PackageFixture {
        bytes: zip.finish().unwrap().into_inner(),
        manifest,
        manifest_json,
        manifest_digest,
        inventory_digest,
        total_bytes,
    }
}

fn manifest_file(path: &str, bytes: &[u8], role: &str) -> ExternalPoolAdapterArtifactManifestFile {
    ExternalPoolAdapterArtifactManifestFile {
        path: path.to_string(),
        sha256: hex::encode(Sha256::digest(bytes)),
        size_bytes: bytes.len() as u64,
        role: role.to_string(),
    }
}

fn target(fixture: &PackageFixture, archive_sha256: &str) -> ExternalPoolAdapterInstallationTarget {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
    let expires = (Utc::now() + Duration::hours(1)).to_rfc3339_opts(SecondsFormat::Nanos, true);
    let adoption_binding = ExternalPoolAdapterAdoptionBinding {
        application_id: "application-1".to_string(),
        application_digest: "3".repeat(64),
        provider_id: "provider-1".to_string(),
        provider_owner_account_id: "owner-1".to_string(),
        provider_policy_revision: 1,
        provider_digest: "4".repeat(64),
        admission_id: "admission-1".to_string(),
        admission_digest: "5".repeat(64),
        adapter_id: fixture.manifest.adapter_id.clone(),
        adapter_release_version: fixture.manifest.release_version.clone(),
        adapter_config_revision: 1,
        adapter_config_digest: "config-digest".to_string(),
        declared_implementation_sha256: archive_sha256.to_string(),
        capability_set_digest: fixture.manifest.capability_set_digest.clone(),
        sandbox_conformance_receipt_id: "sandbox-1".to_string(),
        sandbox_conformance_receipt_digest: "6".repeat(64),
        sandbox_report_expires_at: expires.clone(),
        credential_verification_receipt_id: "credential-1".to_string(),
        credential_verification_receipt_digest: "7".repeat(64),
        credential_locator_commitment: "8".repeat(64),
        credential_report_expires_at: expires,
    };
    let adoption_material = ExternalPoolAdapterAdoptionMaterial {
        binding: adoption_binding,
        adopted_by_admin_user_id: "admin-1".to_string(),
        confirmation: ADOPTION_CONFIRMATION.to_string(),
        idempotency_scope: "scope-adoption".to_string(),
        idempotency_key: "key-adoption".to_string(),
        adopted_at: now.clone(),
        recorded_at: now.clone(),
        adoption_effect: ADOPTION_AUTHORITY_EFFECT.to_string(),
        install_effect: ADOPTION_INSTALL_EFFECT.to_string(),
        provider_effect: ADOPTION_NO_EFFECT.to_string(),
        route_effect: ADOPTION_NO_EFFECT.to_string(),
        execution_effect: ADOPTION_NO_EFFECT.to_string(),
        settlement_effect: ADOPTION_NO_EFFECT.to_string(),
    };
    let mut adoption = ExternalPoolAdapterAdoptionReceipt {
        schema: ADOPTION_RECEIPT_SCHEMA.to_string(),
        adoption_receipt_id: "adoption-1".to_string(),
        adoption_receipt_digest: String::new(),
        adoption_material_digest: adoption_material_digest(&adoption_material).unwrap(),
        canonicalization: ADOPTION_CANONICALIZATION.to_string(),
        digest_algorithm: ADOPTION_DIGEST_ALGORITHM.to_string(),
        adoption: adoption_material,
    };
    adoption.adoption_receipt_digest = canonical_adoption_receipt_json_and_digest(&adoption)
        .unwrap()
        .1;
    let package_material = package_material(fixture, archive_sha256, &now);
    let mut package = ExternalPoolAdapterArtifactPackageReceipt {
        schema: ARTIFACT_PACKAGE_RECEIPT_SCHEMA.to_string(),
        package_receipt_id: "package-1".to_string(),
        package_receipt_digest: String::new(),
        package_material_digest: package_material_digest(&package_material).unwrap(),
        canonicalization: ARTIFACT_PACKAGE_CANONICALIZATION.to_string(),
        digest_algorithm: ARTIFACT_PACKAGE_DIGEST_ALGORITHM.to_string(),
        package: package_material,
    };
    package.package_receipt_digest = canonical_artifact_package_receipt_json_and_digest(&package)
        .unwrap()
        .1;
    ExternalPoolAdapterInstallationTarget {
        adoption_receipt: adoption,
        package_receipt: package,
        source_receipt_id: "source-1".to_string(),
        source_receipt_digest: "9".repeat(64),
    }
}

fn package_material(
    fixture: &PackageFixture,
    archive_sha256: &str,
    now: &str,
) -> ExternalPoolAdapterArtifactPackageReceiptMaterial {
    let inspection = ExternalPoolAdapterArtifactPackageInspection {
        archive_sha256: archive_sha256.to_string(),
        archive_size_bytes: fixture.bytes.len() as u64,
        manifest: fixture.manifest.clone(),
        manifest_canonical_json: fixture.manifest_json.clone(),
        manifest_digest: fixture.manifest_digest.clone(),
        entry_inventory_digest: fixture.inventory_digest.clone(),
        entry_count: fixture.manifest.files.len() as u64,
        total_uncompressed_bytes: fixture.total_bytes,
        inspection_digest: String::new(),
    };
    let mut inspection = inspection;
    #[derive(Serialize)]
    struct InspectionProjection<'a> {
        archive_sha256: &'a str,
        archive_size_bytes: u64,
        manifest_digest: &'a str,
        entry_inventory_digest: &'a str,
        entry_count: u64,
        total_uncompressed_bytes: u64,
    }
    inspection.inspection_digest = domain_json(
        b"ELON-EXTERNAL-POOL-ADAPTER-ARTIFACT-INSPECTION-V1",
        &InspectionProjection {
            archive_sha256: &inspection.archive_sha256,
            archive_size_bytes: inspection.archive_size_bytes,
            manifest_digest: &inspection.manifest_digest,
            entry_inventory_digest: &inspection.entry_inventory_digest,
            entry_count: inspection.entry_count,
            total_uncompressed_bytes: inspection.total_uncompressed_bytes,
        },
    );
    ExternalPoolAdapterArtifactPackageReceiptMaterial {
        admission_id: "admission-1".to_string(),
        admission_digest: "5".repeat(64),
        source_receipt_digest: "9".repeat(64),
        provenance_receipt_id: "provenance-1".to_string(),
        provenance_receipt_digest: "a".repeat(64),
        archive_sha256: inspection.archive_sha256,
        archive_size_bytes: inspection.archive_size_bytes,
        manifest: inspection.manifest,
        manifest_canonical_json: inspection.manifest_canonical_json,
        manifest_digest: inspection.manifest_digest,
        entry_inventory_digest: inspection.entry_inventory_digest,
        entry_count: inspection.entry_count,
        total_uncompressed_bytes: inspection.total_uncompressed_bytes,
        inspection_digest: inspection.inspection_digest,
        inspected_by_admin_user_id: "admin-1".to_string(),
        confirmation: ARTIFACT_PACKAGE_CONFIRMATION.to_string(),
        idempotency_scope: "scope-package".to_string(),
        idempotency_key: "key-package".to_string(),
        inspected_at: now.to_string(),
        recorded_at: now.to_string(),
        evidence_scope: ARTIFACT_PACKAGE_EVIDENCE_SCOPE.to_string(),
        artifact_format_effect: ARTIFACT_PACKAGE_FORMAT_EFFECT.to_string(),
        artifact_security_effect: ARTIFACT_PACKAGE_NO_EFFECT.to_string(),
        conformance_effect: ARTIFACT_PACKAGE_NO_EFFECT.to_string(),
        adapter_effect: ARTIFACT_PACKAGE_NO_EFFECT.to_string(),
        route_effect: ARTIFACT_PACKAGE_NO_EFFECT.to_string(),
    }
}

fn canonical<T: Serialize>(value: &T) -> String {
    canonical_compute_plugin_ijson_and_sha256(value, 512 * 1024)
        .unwrap()
        .0
}

fn domain_bytes(domain: &[u8], bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(bytes);
    hex::encode(digest.finalize())
}

fn domain_json<T: Serialize>(domain: &[u8], value: &T) -> String {
    domain_bytes(domain, canonical(value).as_bytes())
}

fn installed_root(data_dir: &std::path::Path, digest: &str) -> std::path::PathBuf {
    data_dir
        .join(INSTALLATION_STORAGE_NAMESPACE)
        .join(&digest[..2])
        .join(digest)
}
