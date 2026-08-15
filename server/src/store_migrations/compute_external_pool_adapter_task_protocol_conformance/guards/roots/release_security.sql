CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_exact_release_security_roots
BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_run_receipts
WHEN NOT EXISTS (
  SELECT 1
    FROM compute_external_pool_adapter_registry_releases release
    JOIN compute_external_pool_adapter_registry_release_current current_release
      ON current_release.registry_release_id=release.registry_release_id
     AND current_release.registry_release_digest=release.registry_release_digest
    JOIN compute_external_pool_adapter_vulnerability_reattestation_receipts vulnerability
      ON vulnerability.reattestation_receipt_id=NEW.vulnerability_reattestation_receipt_id
     AND vulnerability.reattestation_receipt_digest=NEW.vulnerability_reattestation_receipt_digest
    JOIN compute_external_pool_adapter_vulnerability_reattestation_current current_vulnerability
      ON current_vulnerability.reattestation_receipt_id=vulnerability.reattestation_receipt_id
     AND current_vulnerability.reattestation_receipt_digest=vulnerability.reattestation_receipt_digest
    JOIN compute_external_pool_adapter_sandbox_reattestation_receipts sandbox
      ON sandbox.reattestation_receipt_id=NEW.sandbox_reattestation_receipt_id
     AND sandbox.reattestation_receipt_digest=NEW.sandbox_reattestation_receipt_digest
    JOIN compute_external_pool_adapter_sandbox_reattestation_current current_sandbox
      ON current_sandbox.reattestation_receipt_id=sandbox.reattestation_receipt_id
     AND current_sandbox.reattestation_receipt_digest=sandbox.reattestation_receipt_digest
    JOIN compute_external_pool_adapter_sandbox_verifier_keys verifier
      ON verifier.key_record_id=NEW.sandbox_verifier_key_record_id
     AND verifier.key_record_digest=NEW.sandbox_verifier_key_record_digest
     AND verifier.key_id=NEW.sandbox_verifier_key_id
    JOIN compute_external_pool_adapter_sandbox_verifier_key_current current_verifier
      ON current_verifier.key_record_id=verifier.key_record_id
     AND current_verifier.key_record_digest=verifier.key_record_digest
     AND current_verifier.key_id=verifier.key_id
   WHERE release.registry_release_id=NEW.registry_release_id
     AND release.registry_release_digest=NEW.registry_release_digest
     AND release.registry_release_material_digest=NEW.registry_release_material_digest
     AND current_release.current_status='release_current'
     AND release.admission_id=NEW.admission_id
     AND release.admission_digest=NEW.admission_digest
     AND release.package_receipt_id=NEW.package_receipt_id
     AND release.package_receipt_digest=NEW.package_receipt_digest
     AND release.package_material_digest=NEW.package_material_digest
     AND release.source_receipt_id=NEW.source_receipt_id
     AND release.source_receipt_digest=NEW.source_receipt_digest
     AND release.adapter_id=NEW.adapter_id
     AND release.release_version=NEW.release_version
     AND release.route_kind=NEW.route_kind
     AND release.implementation_digest=NEW.implementation_digest
     AND release.declared_implementation_sha256=NEW.declared_implementation_sha256
     AND release.installation_content_digest=NEW.installation_content_digest
     AND release.capability_set_digest=NEW.capability_set_digest
     AND json(release.supported_capabilities_json)=json(json_extract(
           NEW.run_receipt_json,'$.run.registry_release.supported_capabilities'))
     AND json_extract(release.manifest_canonical_json,'$.runtime.entrypoint')=NEW.entrypoint_path
     AND EXISTS (
       SELECT 1 FROM json_each(release.manifest_canonical_json,'$.files') entrypoint
        WHERE json_extract(entrypoint.value,'$.role')='entrypoint'
          AND json_extract(entrypoint.value,'$.path')=NEW.entrypoint_path
          AND json_extract(entrypoint.value,'$.sha256')=NEW.entrypoint_sha256
          AND json_extract(entrypoint.value,'$.size_bytes')=NEW.entrypoint_size_bytes)
     AND current_vulnerability.current_status='verified_current'
     AND vulnerability.reattestation_material_digest=NEW.vulnerability_reattestation_material_digest
     AND vulnerability.registry_release_id=NEW.registry_release_id
     AND vulnerability.registry_release_digest=NEW.registry_release_digest
     AND vulnerability.registry_release_material_digest=NEW.registry_release_material_digest
     AND vulnerability.admission_id=NEW.admission_id
     AND vulnerability.admission_digest=NEW.admission_digest
     AND vulnerability.package_receipt_id=NEW.package_receipt_id
     AND vulnerability.package_receipt_digest=NEW.package_receipt_digest
     AND vulnerability.implementation_digest=NEW.implementation_digest
     AND vulnerability.installation_content_digest=NEW.installation_content_digest
     AND vulnerability.intelligence_snapshot_digest=NEW.vulnerability_intelligence_snapshot_digest
     AND vulnerability.intelligence_expires_at=NEW.vulnerability_intelligence_expires_at
     AND vulnerability.blocking_finding_count=0
     AND vulnerability.verified_at<=NEW.post_cleanup_checked_at
     AND NEW.post_cleanup_checked_at<vulnerability.intelligence_expires_at
     AND current_sandbox.current_status='verified_current'
     AND sandbox.reattestation_material_digest=NEW.sandbox_reattestation_material_digest
     AND sandbox.registry_release_id=NEW.registry_release_id
     AND sandbox.registry_release_digest=NEW.registry_release_digest
     AND sandbox.registry_release_material_digest=NEW.registry_release_material_digest
     AND sandbox.admission_id=NEW.admission_id
     AND sandbox.admission_digest=NEW.admission_digest
     AND sandbox.package_receipt_id=NEW.package_receipt_id
     AND sandbox.package_receipt_digest=NEW.package_receipt_digest
     AND sandbox.source_receipt_id=NEW.source_receipt_id
     AND sandbox.source_receipt_digest=NEW.source_receipt_digest
     AND sandbox.adapter_id=NEW.adapter_id
     AND sandbox.release_version=NEW.release_version
     AND sandbox.route_kind=NEW.route_kind
     AND sandbox.implementation_digest=NEW.implementation_digest
     AND sandbox.declared_implementation_sha256=NEW.declared_implementation_sha256
     AND sandbox.installation_content_digest=NEW.installation_content_digest
     AND sandbox.capability_set_digest=NEW.capability_set_digest
     AND sandbox.vulnerability_reattestation_receipt_id=NEW.vulnerability_reattestation_receipt_id
     AND sandbox.vulnerability_reattestation_receipt_digest=NEW.vulnerability_reattestation_receipt_digest
     AND sandbox.vulnerability_reattestation_material_digest=NEW.vulnerability_reattestation_material_digest
     AND sandbox.vulnerability_intelligence_snapshot_digest=NEW.vulnerability_intelligence_snapshot_digest
     AND sandbox.vulnerability_intelligence_expires_at=NEW.vulnerability_intelligence_expires_at
     AND json_extract(sandbox.receipt_json,'$.reattestation.binding.sandbox_policy_id')=NEW.sandbox_policy_id
     AND sandbox.test_plan_digest=NEW.sandbox_test_plan_digest
     AND sandbox.observation_inventory_digest=NEW.sandbox_observation_inventory_digest
     AND sandbox.report_expires_at=NEW.sandbox_report_expires_at
     AND sandbox.passed_capability_count=6
     AND sandbox.policy_violation_count=0
     AND sandbox.verified_at<=NEW.post_cleanup_checked_at
     AND NEW.post_cleanup_checked_at<sandbox.report_expires_at
     AND sandbox.sandbox_verifier_key_record_id=NEW.sandbox_verifier_key_record_id
     AND sandbox.sandbox_verifier_key_record_digest=NEW.sandbox_verifier_key_record_digest
     AND sandbox.sandbox_verifier_key_id=NEW.sandbox_verifier_key_id
     AND sandbox.sandbox_verifier_operator=NEW.sandbox_verifier_operator
     AND sandbox.sandbox_verifier_product=NEW.sandbox_verifier_product
     AND verifier.verifier_operator=NEW.sandbox_verifier_operator
     AND verifier.verifier_product=NEW.sandbox_verifier_product
     AND current_verifier.verifier_operator=NEW.sandbox_verifier_operator
     AND current_verifier.verifier_product=NEW.sandbox_verifier_product
     AND current_verifier.current_status='active'
)
BEGIN SELECT RAISE(ABORT,'V272 run lacks exact current V249/V250/V252/V237 roots'); END;
