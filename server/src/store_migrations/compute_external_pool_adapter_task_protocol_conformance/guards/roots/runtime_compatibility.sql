CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_exact_runtime_roots
BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_run_receipts
WHEN NOT EXISTS (
  SELECT 1
    FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts verification
    JOIN compute_external_pool_adapter_runtime_compatibility_verification_current current_verification
      ON current_verification.verification_receipt_id=verification.verification_receipt_id
     AND current_verification.verification_receipt_digest=verification.verification_receipt_digest
    JOIN compute_external_pool_adapter_runtime_compatibility_verification_run_observations observation
      ON observation.run_observation_id=verification.run_observation_id
     AND observation.run_observation_digest=verification.run_observation_digest
    JOIN compute_external_pool_adapter_runtime_compatibility_verification_challenges challenge
      ON challenge.challenge_id=verification.challenge_id
     AND challenge.challenge_digest=verification.challenge_digest
     AND observation.challenge_id=challenge.challenge_id
     AND observation.challenge_digest=challenge.challenge_digest
    JOIN compute_external_pool_adapter_sandbox_verifier_keys verifier
      ON verifier.key_record_id=verification.sandbox_verifier_key_record_id
     AND verifier.key_record_digest=verification.sandbox_verifier_key_record_digest
     AND verifier.key_id=verification.sandbox_verifier_key_id
    JOIN compute_external_pool_adapter_sandbox_verifier_key_current current_verifier
      ON current_verifier.key_record_id=verifier.key_record_id
     AND current_verifier.key_record_digest=verifier.key_record_digest
     AND current_verifier.key_id=verifier.key_id
   WHERE verification.verification_receipt_id=NEW.runtime_compatibility_verification_receipt_id
     AND verification.verification_receipt_digest=NEW.runtime_compatibility_verification_receipt_digest
     AND verification.verification_material_digest=NEW.runtime_compatibility_verification_material_digest
     AND current_verification.currentness_status='current_signed_verifier_assertion'
     AND verification.registry_release_id=NEW.registry_release_id
     AND verification.registry_release_digest=NEW.registry_release_digest
     AND verification.registry_release_material_digest=NEW.registry_release_material_digest
     AND verification.installation_content_digest=NEW.installation_content_digest
     AND verification.run_observation_id=NEW.runtime_compatibility_run_observation_id
     AND verification.run_observation_digest=NEW.runtime_compatibility_run_observation_digest
     AND verification.run_observation_material_digest=NEW.runtime_compatibility_run_observation_material_digest
     AND verification.runner_execution_id=NEW.runtime_compatibility_runner_execution_id
     AND verification.profile_id=NEW.runtime_compatibility_profile_id
     AND verification.profile_revision=NEW.runtime_compatibility_profile_revision
     AND verification.profile_digest=NEW.runtime_compatibility_profile_digest
     AND verification.runner_policy_digest=NEW.runtime_compatibility_runner_policy_digest
     AND verification.fixture_catalog_digest=NEW.runtime_compatibility_fixture_catalog_digest
     AND verification.public_fixture_delivery_root=NEW.runtime_compatibility_public_fixture_delivery_root
     AND verification.sandbox_verifier_key_record_id=NEW.sandbox_verifier_key_record_id
     AND verification.sandbox_verifier_key_record_digest=NEW.sandbox_verifier_key_record_digest
     AND verification.sandbox_verifier_key_id=NEW.sandbox_verifier_key_id
     AND verification.sandbox_verifier_operator=NEW.sandbox_verifier_operator
     AND verification.sandbox_verifier_product=NEW.sandbox_verifier_product
     AND verification.expires_at=NEW.runtime_compatibility_expires_at
     AND verification.verified_at<=NEW.post_cleanup_checked_at
     AND NEW.post_cleanup_checked_at<verification.expires_at
     AND observation.run_observation_material_digest=NEW.runtime_compatibility_run_observation_material_digest
     AND observation.runner_execution_id=NEW.runtime_compatibility_runner_execution_id
     AND observation.registry_release_id=NEW.registry_release_id
     AND observation.registry_release_digest=NEW.registry_release_digest
     AND observation.registry_release_material_digest=NEW.registry_release_material_digest
     AND observation.installation_content_digest=NEW.installation_content_digest
     AND observation.profile_id=NEW.runtime_compatibility_profile_id
     AND observation.profile_revision=NEW.runtime_compatibility_profile_revision
     AND observation.profile_digest=NEW.runtime_compatibility_profile_digest
     AND observation.runner_policy_digest=NEW.runtime_compatibility_runner_policy_digest
     AND observation.fixture_catalog_digest=NEW.runtime_compatibility_fixture_catalog_digest
     AND observation.source_capsule_sha256=NEW.source_capsule_sha256
     AND observation.source_capsule_size_bytes=NEW.source_capsule_size_bytes
     AND observation.launch_image_sha256=NEW.launch_image_sha256
     AND observation.launch_image_size_bytes=NEW.launch_image_size_bytes
     AND observation.public_fixture_delivery_root=NEW.runtime_compatibility_public_fixture_delivery_root
     AND challenge.registry_release_id=NEW.registry_release_id
     AND challenge.registry_release_digest=NEW.registry_release_digest
     AND challenge.registry_release_material_digest=NEW.registry_release_material_digest
     AND challenge.installation_content_digest=NEW.installation_content_digest
     AND challenge.entrypoint_path=NEW.entrypoint_path
     AND challenge.entrypoint_sha256=NEW.entrypoint_sha256
     AND challenge.entrypoint_size_bytes=NEW.entrypoint_size_bytes
     AND challenge.profile_id=NEW.runtime_compatibility_profile_id
     AND challenge.profile_revision=NEW.runtime_compatibility_profile_revision
     AND challenge.profile_digest=NEW.runtime_compatibility_profile_digest
     AND challenge.runner_policy_digest=NEW.runtime_compatibility_runner_policy_digest
     AND challenge.fixture_catalog_digest=NEW.runtime_compatibility_fixture_catalog_digest
     AND challenge.supervisor_session_policy_digest=NEW.supervisor_session_policy_digest
     AND challenge.sandbox_verifier_key_record_id=NEW.sandbox_verifier_key_record_id
     AND challenge.sandbox_verifier_key_record_digest=NEW.sandbox_verifier_key_record_digest
     AND challenge.sandbox_verifier_key_id=NEW.sandbox_verifier_key_id
     AND challenge.sandbox_verifier_operator=NEW.sandbox_verifier_operator
     AND challenge.sandbox_verifier_product=NEW.sandbox_verifier_product
     AND verifier.verifier_operator=NEW.sandbox_verifier_operator
     AND verifier.verifier_product=NEW.sandbox_verifier_product
     AND current_verifier.current_status='active'
     AND current_verifier.verifier_operator=NEW.sandbox_verifier_operator
     AND current_verifier.verifier_product=NEW.sandbox_verifier_product
     -- The V272 fresh delivery root is deliberately not compared with the randomized V268 root.
     AND NEW.expires_at=min(
           strftime('%Y-%m-%dT%H:%M:%S',NEW.post_cleanup_checked_at,'+15 seconds')||substr(NEW.post_cleanup_checked_at,20),
           (SELECT upstream.intelligence_expires_at
              FROM compute_external_pool_adapter_vulnerability_reattestation_receipts upstream
             WHERE upstream.reattestation_receipt_id=NEW.vulnerability_reattestation_receipt_id
               AND upstream.reattestation_receipt_digest=NEW.vulnerability_reattestation_receipt_digest),
           (SELECT upstream.report_expires_at
              FROM compute_external_pool_adapter_sandbox_reattestation_receipts upstream
             WHERE upstream.reattestation_receipt_id=NEW.sandbox_reattestation_receipt_id
               AND upstream.reattestation_receipt_digest=NEW.sandbox_reattestation_receipt_digest),
           verification.expires_at)
)
BEGIN SELECT RAISE(ABORT,'V272 run lacks exact current V268/V237 roots or TTL'); END;
