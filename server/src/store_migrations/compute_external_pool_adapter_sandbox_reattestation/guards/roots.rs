use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_challenge_exact_roots
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_challenges
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_adapter_registry_release_current current_release
            JOIN compute_external_pool_adapter_registry_releases release
              ON release.registry_release_id=current_release.registry_release_id
             AND release.registry_release_digest=current_release.registry_release_digest
            JOIN compute_external_pool_adapter_vulnerability_reattestation_current vulnerability
              ON vulnerability.current_status='verified_current'
            JOIN compute_external_pool_adapter_vulnerability_reattestation_receipts vulnerability_receipt
              ON vulnerability_receipt.reattestation_receipt_id=vulnerability.reattestation_receipt_id
             AND vulnerability_receipt.reattestation_receipt_digest=vulnerability.reattestation_receipt_digest
            JOIN compute_external_pool_adapter_sandbox_verifier_key_current verifier
              ON verifier.current_status='active'
           WHERE current_release.current_status='release_current'
             AND release.registry_release_id=NEW.registry_release_id
             AND release.registry_release_digest=NEW.registry_release_digest
             AND release.registry_release_material_digest=NEW.registry_release_material_digest
             AND vulnerability.registry_release_id=NEW.registry_release_id
             AND vulnerability.registry_release_digest=NEW.registry_release_digest
             AND vulnerability_receipt.reattestation_receipt_id=NEW.vulnerability_reattestation_receipt_id
             AND vulnerability_receipt.reattestation_receipt_digest=NEW.vulnerability_reattestation_receipt_digest
             AND vulnerability_receipt.reattestation_material_digest=NEW.vulnerability_reattestation_material_digest
             AND verifier.key_record_id=NEW.sandbox_verifier_key_record_id
             AND verifier.key_record_digest=NEW.sandbox_verifier_key_record_digest
             AND verifier.key_id=NEW.sandbox_verifier_key_id
             AND json_extract(NEW.challenge_json,'$.binding.admission_id')=release.admission_id
             AND json_extract(NEW.challenge_json,'$.binding.admission_digest')=release.admission_digest
             AND json_extract(NEW.challenge_json,'$.binding.package_receipt_id')=release.package_receipt_id
             AND json_extract(NEW.challenge_json,'$.binding.package_receipt_digest')=release.package_receipt_digest
             AND json_extract(NEW.challenge_json,'$.binding.source_receipt_id')=release.source_receipt_id
             AND json_extract(NEW.challenge_json,'$.binding.source_receipt_digest')=release.source_receipt_digest
             AND json_extract(NEW.challenge_json,'$.binding.adapter_id')=release.adapter_id
             AND json_extract(NEW.challenge_json,'$.binding.release_version')=release.release_version
             AND json_extract(NEW.challenge_json,'$.binding.route_kind')=release.route_kind
             AND json(json_extract(NEW.challenge_json,'$.binding.supported_provider_kinds'))=json(release.supported_provider_kinds_json)
             AND json_extract(NEW.challenge_json,'$.binding.implementation_digest')=release.implementation_digest
             AND json_extract(NEW.challenge_json,'$.binding.declared_implementation_sha256')=release.declared_implementation_sha256
             AND json(json_extract(NEW.challenge_json,'$.binding.supported_capabilities'))=json(release.supported_capabilities_json)
             AND json_extract(NEW.challenge_json,'$.binding.capability_set_digest')=release.capability_set_digest
             AND json(json_extract(NEW.challenge_json,'$.binding.expected_credential_verifier'))=json(release.credential_verifier_json)
             AND json_extract(NEW.challenge_json,'$.binding.credential_verifier_digest')=release.credential_verifier_digest
             AND json_extract(NEW.challenge_json,'$.binding.archive_sha256')=release.archive_sha256
             AND json_extract(NEW.challenge_json,'$.binding.archive_size_bytes')=release.archive_size_bytes
             AND json_extract(NEW.challenge_json,'$.binding.manifest_digest')=release.manifest_digest
             AND json_extract(NEW.challenge_json,'$.binding.entry_inventory_digest')=release.entry_inventory_digest
             AND json_extract(NEW.challenge_json,'$.binding.entry_count')=release.entry_count
             AND json_extract(NEW.challenge_json,'$.binding.total_uncompressed_bytes')=release.total_uncompressed_bytes
             AND json_extract(NEW.challenge_json,'$.binding.installation_content_digest')=release.installation_content_digest
             AND json_extract(NEW.challenge_json,'$.binding.vulnerability_reattestation_sequence')=vulnerability_receipt.sequence
             AND json_extract(NEW.challenge_json,'$.binding.vulnerability_reattestation_verified_at')=vulnerability_receipt.verified_at
             AND json_extract(NEW.challenge_json,'$.binding.vulnerability_intelligence_snapshot_digest')=vulnerability_receipt.intelligence_snapshot_digest
             AND json_extract(NEW.challenge_json,'$.binding.vulnerability_intelligence_expires_at')=vulnerability_receipt.intelligence_expires_at
             AND json_extract(NEW.challenge_json,'$.binding.security_receipt_id')=vulnerability_receipt.security_receipt_id
             AND json_extract(NEW.challenge_json,'$.binding.security_receipt_digest')=vulnerability_receipt.security_receipt_digest
             AND json_extract(NEW.challenge_json,'$.binding.security_material_digest')=vulnerability_receipt.security_material_digest
             AND json_extract(NEW.challenge_json,'$.binding.sbom_digest')=vulnerability_receipt.sbom_digest
             AND json_extract(NEW.challenge_json,'$.binding.component_inventory_digest')=vulnerability_receipt.component_inventory_digest
             AND json_extract(NEW.challenge_json,'$.binding.component_count')=vulnerability_receipt.component_count
             AND json_extract(NEW.challenge_json,'$.binding.dependency_inventory_digest')=vulnerability_receipt.dependency_inventory_digest
             AND json_extract(NEW.challenge_json,'$.binding.sandbox_verifier_operator')=verifier.verifier_operator
             AND json_extract(NEW.challenge_json,'$.binding.sandbox_verifier_product')=verifier.verifier_product)
           OR NEW.issued_at>(strftime('%Y-%m-%dT%H:%M:%S','now','+5 minutes')||'.999999999Z')
          OR json_extract(NEW.challenge_json,'$.binding.run_started_at')<json_extract(NEW.challenge_json,'$.binding.vulnerability_reattestation_verified_at')
          OR json_extract(NEW.challenge_json,'$.binding.report_expires_at')>json_extract(NEW.challenge_json,'$.binding.vulnerability_intelligence_expires_at')
        BEGIN SELECT RAISE(ABORT,'V252 challenge lacks current exact V249/V250/V237 roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_receipt_exact_challenge
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_receipts
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_challenges challenge
           WHERE challenge.challenge_id=NEW.challenge_id
             AND challenge.challenge_nonce_digest=NEW.challenge_nonce_digest
             AND challenge.signature_message_digest=NEW.signature_message_digest
             AND challenge.registry_release_id=NEW.registry_release_id
             AND challenge.registry_release_digest=NEW.registry_release_digest
             AND challenge.registry_release_material_digest=NEW.registry_release_material_digest
             AND challenge.vulnerability_reattestation_receipt_id=NEW.vulnerability_reattestation_receipt_id
             AND challenge.vulnerability_reattestation_receipt_digest=NEW.vulnerability_reattestation_receipt_digest
             AND challenge.vulnerability_reattestation_material_digest=NEW.vulnerability_reattestation_material_digest
             AND challenge.sandbox_verifier_key_record_id=NEW.sandbox_verifier_key_record_id
             AND challenge.sandbox_verifier_key_record_digest=NEW.sandbox_verifier_key_record_digest
             AND challenge.sandbox_verifier_key_id=NEW.sandbox_verifier_key_id
             AND challenge.sequence=NEW.sequence
             AND challenge.predecessor_receipt_id IS NEW.predecessor_receipt_id
             AND challenge.predecessor_receipt_digest IS NEW.predecessor_receipt_digest
             AND json(json_extract(challenge.challenge_json,'$.binding'))=json(json_extract(NEW.receipt_json,'$.reattestation.binding'))
             AND challenge.issued_at<=NEW.verified_at AND NEW.verified_at<challenge.expires_at)
        BEGIN SELECT RAISE(ABORT,'V252 receipt requires exact unexpired single-use challenge'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_receipt_current_roots
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_receipts
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_adapter_registry_release_current current_release
            JOIN compute_external_pool_adapter_registry_releases release
              ON release.registry_release_id=current_release.registry_release_id
             AND release.registry_release_digest=current_release.registry_release_digest
            JOIN compute_external_pool_adapter_vulnerability_reattestation_current vulnerability
              ON vulnerability.current_status='verified_current'
            JOIN compute_external_pool_adapter_vulnerability_reattestation_receipts vulnerability_receipt
              ON vulnerability_receipt.reattestation_receipt_id=vulnerability.reattestation_receipt_id
             AND vulnerability_receipt.reattestation_receipt_digest=vulnerability.reattestation_receipt_digest
            JOIN compute_external_pool_adapter_sandbox_verifier_key_current verifier
              ON verifier.current_status='active'
           WHERE current_release.current_status='release_current'
             AND release.registry_release_id=NEW.registry_release_id
             AND release.registry_release_digest=NEW.registry_release_digest
             AND release.registry_release_material_digest=NEW.registry_release_material_digest
             AND release.admission_id=NEW.admission_id AND release.admission_digest=NEW.admission_digest
             AND release.package_receipt_id=NEW.package_receipt_id AND release.package_receipt_digest=NEW.package_receipt_digest
             AND release.source_receipt_id=NEW.source_receipt_id AND release.source_receipt_digest=NEW.source_receipt_digest
             AND release.adapter_id=NEW.adapter_id AND release.release_version=NEW.release_version
             AND release.route_kind=NEW.route_kind
             AND release.supported_provider_kinds_json=NEW.supported_provider_kinds_json
             AND release.implementation_digest=NEW.implementation_digest
             AND release.declared_implementation_sha256=NEW.declared_implementation_sha256
             AND release.supported_capabilities_json=NEW.supported_capabilities_json
             AND release.capability_set_digest=NEW.capability_set_digest
             AND release.credential_verifier_json=NEW.credential_verifier_json
             AND release.credential_verifier_digest=NEW.credential_verifier_digest
             AND release.archive_sha256=NEW.archive_sha256
             AND release.archive_size_bytes=NEW.archive_size_bytes
             AND release.manifest_digest=NEW.manifest_digest
             AND release.entry_inventory_digest=NEW.entry_inventory_digest
             AND release.entry_count=NEW.entry_count
             AND release.total_uncompressed_bytes=NEW.total_uncompressed_bytes
             AND release.installation_content_digest=NEW.installation_content_digest
             AND vulnerability.registry_release_id=NEW.registry_release_id
             AND vulnerability.registry_release_digest=NEW.registry_release_digest
             AND vulnerability_receipt.reattestation_receipt_id=NEW.vulnerability_reattestation_receipt_id
             AND vulnerability_receipt.reattestation_receipt_digest=NEW.vulnerability_reattestation_receipt_digest
             AND vulnerability_receipt.reattestation_material_digest=NEW.vulnerability_reattestation_material_digest
             AND vulnerability_receipt.sequence=NEW.vulnerability_reattestation_sequence
             AND vulnerability_receipt.verified_at=NEW.vulnerability_reattestation_verified_at
             AND vulnerability_receipt.intelligence_snapshot_digest=NEW.vulnerability_intelligence_snapshot_digest
             AND vulnerability_receipt.intelligence_expires_at=NEW.vulnerability_intelligence_expires_at
             AND vulnerability_receipt.security_receipt_id=NEW.security_receipt_id
             AND vulnerability_receipt.security_receipt_digest=NEW.security_receipt_digest
             AND vulnerability_receipt.security_material_digest=NEW.security_material_digest
             AND vulnerability_receipt.sbom_digest=NEW.sbom_digest
             AND vulnerability_receipt.component_inventory_digest=NEW.component_inventory_digest
             AND vulnerability_receipt.component_count=NEW.component_count
             AND vulnerability_receipt.dependency_inventory_digest=NEW.dependency_inventory_digest
             AND verifier.key_record_id=NEW.sandbox_verifier_key_record_id
             AND verifier.key_record_digest=NEW.sandbox_verifier_key_record_digest
             AND verifier.key_id=NEW.sandbox_verifier_key_id
             AND verifier.verifier_operator=NEW.sandbox_verifier_operator
             AND verifier.verifier_product=NEW.sandbox_verifier_product)
        BEGIN SELECT RAISE(ABORT,'V252 receipt lacks current exact V249/V250/V237 roots'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_adapter_sandbox_reattestation_revocation_exact_target
        BEFORE INSERT ON compute_external_pool_adapter_sandbox_reattestation_revocations
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_sandbox_reattestation_receipts receipt
           WHERE receipt.reattestation_receipt_id=NEW.reattestation_receipt_id
             AND receipt.reattestation_receipt_digest=NEW.reattestation_receipt_digest
             AND receipt.registry_release_id=NEW.registry_release_id
             AND receipt.registry_release_digest=NEW.registry_release_digest)
        BEGIN SELECT RAISE(ABORT,'V252 revocation requires exact receipt/release target'); END;
        "#,
    )?;
    Ok(())
}
