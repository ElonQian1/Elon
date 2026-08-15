use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_exact_current_roots
        BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_receipts
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_adapter_supervisor_session_policy_companions companion
            JOIN compute_external_pool_adapter_supervisor_session_policy_companion_current current_companion
              ON current_companion.companion_id=companion.companion_id
             AND current_companion.companion_digest=companion.companion_digest
            JOIN compute_external_pool_adapter_upstream_transport_targets target
              ON target.target_id=companion.target_id
             AND target.target_digest=companion.target_digest
            JOIN compute_external_pool_adapter_runtime_launch_profiles profile
              ON profile.profile_id=companion.profile_id
             AND profile.profile_digest=companion.profile_digest
            JOIN compute_external_pool_provider_activation_candidates candidate
              ON candidate.candidate_id=companion.candidate_id
             AND candidate.candidate_digest=companion.candidate_digest
             AND candidate.delegation_id=companion.delegation_id
             AND candidate.delegation_digest=companion.delegation_digest
            JOIN compute_external_pool_adapter_registry_provider_bindings binding
              ON binding.provider_binding_id=companion.provider_binding_id
             AND binding.provider_binding_digest=companion.provider_binding_digest
            JOIN compute_external_pool_adapter_registry_releases release
              ON release.registry_release_id=companion.registry_release_id
             AND release.registry_release_digest=companion.registry_release_digest
            JOIN compute_providers provider
              ON provider.provider_id=companion.provider_id
             AND provider.provider_kind='external_pool'
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
            JOIN compute_external_pool_adapter_credential_reattestation_receipts credential
              ON credential.reattestation_receipt_id=NEW.credential_reattestation_receipt_id
             AND credential.reattestation_receipt_digest=NEW.credential_reattestation_receipt_digest
            JOIN compute_external_pool_adapter_credential_reattestation_current current_credential
              ON current_credential.reattestation_receipt_id=credential.reattestation_receipt_id
             AND current_credential.reattestation_receipt_digest=credential.reattestation_receipt_digest
            JOIN compute_external_pool_adapter_runtime_compatibility_verification_receipts compatibility
              ON compatibility.verification_receipt_id=NEW.runtime_compatibility_verification_receipt_id
             AND compatibility.verification_receipt_digest=NEW.runtime_compatibility_verification_receipt_digest
            JOIN compute_external_pool_adapter_runtime_compatibility_verification_current current_compatibility
              ON current_compatibility.verification_receipt_id=compatibility.verification_receipt_id
             AND current_compatibility.verification_receipt_digest=compatibility.verification_receipt_digest
            JOIN compute_external_pool_adapter_runtime_compatibility_verification_run_observations observation
              ON observation.run_observation_id=compatibility.run_observation_id
             AND observation.run_observation_digest=compatibility.run_observation_digest
           WHERE companion.companion_id=NEW.companion_id
             AND companion.companion_digest=NEW.companion_digest
             AND current_companion.current_status='supervisor_session_policy_companion_current_inert'
             AND current_companion.head_status='head'
             AND current_companion.revocation_status='unrevoked'
             AND current_companion.target_status='upstream_transport_target_current_inert'
             AND current_companion.profile_status='launch_profile_current_inert'
             AND current_companion.policy_status='server_policy_current'
             AND companion.target_id=NEW.target_id
             AND companion.target_digest=NEW.target_digest
             AND companion.profile_id=NEW.profile_id
             AND companion.profile_digest=NEW.profile_digest
             AND companion.candidate_id=NEW.candidate_id
             AND companion.candidate_digest=NEW.candidate_digest
             AND companion.delegation_id=NEW.delegation_id
             AND companion.delegation_digest=NEW.delegation_digest
             AND companion.provider_binding_id=NEW.provider_binding_id
             AND companion.provider_binding_digest=NEW.provider_binding_digest
             AND companion.registry_release_id=NEW.registry_release_id
             AND companion.registry_release_digest=NEW.registry_release_digest
             AND companion.installation_receipt_id=NEW.installation_receipt_id
             AND companion.installation_receipt_digest=NEW.installation_receipt_digest
             AND companion.installation_content_digest=NEW.installation_content_digest
             AND companion.provider_id=NEW.provider_id
             AND companion.provider_policy_revision=NEW.provider_policy_revision
             AND companion.provider_digest=NEW.provider_digest
             AND companion.provider_status=NEW.provider_status
             AND companion.launch_policy_digest=NEW.launch_policy_digest
             AND companion.entrypoint_capsule_policy_digest=NEW.entrypoint_capsule_policy_digest
             AND companion.target_policy_digest=NEW.target_policy_digest
             AND companion.supervisor_session_policy_digest=NEW.supervisor_session_policy_digest
             AND companion.recorded_at<=NEW.checked_at
             AND target.provider_binding_id=NEW.provider_binding_id
             AND target.provider_binding_digest=NEW.provider_binding_digest
             AND target.profile_id=NEW.profile_id
             AND target.profile_digest=NEW.profile_digest
             AND target.candidate_id=NEW.candidate_id
             AND target.candidate_digest=NEW.candidate_digest
             AND target.target_policy_digest=NEW.target_policy_digest
             AND target.recorded_at<=NEW.checked_at
             AND profile.provider_binding_id=NEW.provider_binding_id
             AND profile.provider_binding_digest=NEW.provider_binding_digest
             AND profile.candidate_id=NEW.candidate_id
             AND profile.candidate_digest=NEW.candidate_digest
             AND profile.delegation_id=NEW.delegation_id
             AND profile.delegation_digest=NEW.delegation_digest
             AND profile.registry_release_id=NEW.registry_release_id
             AND profile.registry_release_digest=NEW.registry_release_digest
             AND profile.installation_receipt_id=NEW.installation_receipt_id
             AND profile.installation_receipt_digest=NEW.installation_receipt_digest
             AND profile.installation_content_digest=NEW.installation_content_digest
             AND profile.provider_id=NEW.provider_id
             AND profile.provider_policy_revision=NEW.provider_policy_revision
             AND profile.provider_digest=NEW.provider_digest
             AND profile.provider_status=NEW.provider_status
             AND profile.launch_policy_digest=NEW.launch_policy_digest
             AND profile.entrypoint_sha256=NEW.source_capsule_sha256
             AND profile.entrypoint_size_bytes=NEW.source_capsule_size_bytes
             AND profile.recorded_at<=NEW.checked_at
             AND json_extract(profile.launch_policy_json,'$.probe_timeout_ms')=15000
             AND candidate.provider_binding_id=NEW.provider_binding_id
             AND candidate.provider_binding_digest=NEW.provider_binding_digest
             AND candidate.registry_release_id=NEW.registry_release_id
             AND candidate.registry_release_digest=NEW.registry_release_digest
             AND candidate.installation_receipt_id=NEW.installation_receipt_id
             AND candidate.installation_receipt_digest=NEW.installation_receipt_digest
             AND candidate.installation_content_digest=NEW.installation_content_digest
             AND candidate.provider_id=NEW.provider_id
             AND candidate.provider_policy_revision=NEW.provider_policy_revision
             AND candidate.provider_digest=NEW.provider_digest
             AND candidate.provider_status='registering'
             AND candidate.candidate_status='candidate_current_not_activation_ready'
             AND candidate.activation_closure_status='activation_closure_not_implemented'
             AND candidate.recorded_at<=NEW.checked_at
             AND binding.registry_release_id=NEW.registry_release_id
             AND binding.registry_release_digest=NEW.registry_release_digest
             AND binding.installation_receipt_id=NEW.installation_receipt_id
             AND binding.installation_receipt_digest=NEW.installation_receipt_digest
             AND binding.installation_content_digest=NEW.installation_content_digest
             AND binding.provider_id=NEW.provider_id
             AND binding.provider_policy_revision=NEW.provider_policy_revision
             AND binding.provider_digest=NEW.provider_digest
             AND binding.recorded_at<=NEW.checked_at
             AND release.registry_release_material_digest=NEW.registry_release_material_digest
             AND release.installation_content_digest=NEW.installation_content_digest
             AND release.recorded_at<=NEW.checked_at
             AND provider.status='registering'
             AND provider.current_policy_revision=NEW.provider_policy_revision
             AND provider.current_provider_digest=NEW.provider_digest
             AND current_vulnerability.current_status='verified_current'
             AND vulnerability.registry_release_id=NEW.registry_release_id
             AND vulnerability.registry_release_digest=NEW.registry_release_digest
             AND vulnerability.registry_release_material_digest=NEW.registry_release_material_digest
             AND vulnerability.installation_content_digest=NEW.installation_content_digest
             AND vulnerability.verified_at<=NEW.checked_at
             AND NEW.checked_at<vulnerability.intelligence_expires_at
             AND current_sandbox.current_status='verified_current'
             AND sandbox.registry_release_id=NEW.registry_release_id
             AND sandbox.registry_release_digest=NEW.registry_release_digest
             AND sandbox.registry_release_material_digest=NEW.registry_release_material_digest
             AND sandbox.installation_content_digest=NEW.installation_content_digest
             AND sandbox.vulnerability_reattestation_receipt_id=vulnerability.reattestation_receipt_id
             AND sandbox.vulnerability_reattestation_receipt_digest=vulnerability.reattestation_receipt_digest
             AND sandbox.verified_at<=NEW.checked_at
             AND NEW.checked_at<sandbox.report_expires_at
             AND current_credential.current_status='verified_current'
             AND credential.provider_binding_id=NEW.provider_binding_id
             AND credential.provider_binding_digest=NEW.provider_binding_digest
             AND credential.registry_release_id=NEW.registry_release_id
             AND credential.registry_release_digest=NEW.registry_release_digest
             AND credential.registry_release_material_digest=NEW.registry_release_material_digest
             AND credential.installation_receipt_id=NEW.installation_receipt_id
             AND credential.installation_receipt_digest=NEW.installation_receipt_digest
             AND credential.installation_content_digest=NEW.installation_content_digest
             AND credential.provider_id=NEW.provider_id
             AND credential.observed_provider_policy_revision=NEW.provider_policy_revision
             AND credential.observed_provider_digest=NEW.provider_digest
             AND credential.observed_provider_status='registering'
             AND credential.verified_at<=NEW.checked_at
             AND NEW.checked_at<credential.report_expires_at
             AND current_compatibility.currentness_status='current_signed_verifier_assertion'
             AND compatibility.registry_release_id=NEW.registry_release_id
             AND compatibility.registry_release_digest=NEW.registry_release_digest
             AND compatibility.registry_release_material_digest=NEW.registry_release_material_digest
             AND compatibility.installation_content_digest=NEW.installation_content_digest
             AND compatibility.verified_at<=NEW.checked_at
             AND NEW.checked_at<compatibility.expires_at
             AND observation.source_capsule_sha256=NEW.source_capsule_sha256
             AND observation.source_capsule_size_bytes=NEW.source_capsule_size_bytes
             AND observation.source_capsule_policy_digest=NEW.entrypoint_capsule_policy_digest
             AND observation.launch_image_sha256=NEW.launch_image_sha256
             AND observation.launch_image_size_bytes=NEW.launch_image_size_bytes
             AND NEW.entrypoint_capsule_policy_digest='710decef25b4d19b33f086239f55f809a513508eb5ba431967971ff89249604f'
             AND json_extract(companion.supervisor_session_policy_json,'$.state.probe_timeout_ms')=15000
             AND NEW.expires_at=min(
                   strftime('%Y-%m-%dT%H:%M:%S',NEW.checked_at,'+15 seconds')||substr(NEW.checked_at,20),
                   vulnerability.intelligence_expires_at,
                   sandbox.report_expires_at,
                   credential.report_expires_at,
                   compatibility.expires_at)
        )
        BEGIN SELECT RAISE(ABORT,'V270 readiness lacks exact current V249-V259/V268 roots or TTL'); END;

        CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_revocation_exact_roots
        BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_revocations
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_provider_runtime_readiness_receipts receipt
           WHERE receipt.readiness_receipt_id=NEW.readiness_receipt_id
             AND receipt.readiness_receipt_digest=NEW.readiness_receipt_digest
             AND receipt.provider_binding_id=NEW.provider_binding_id
             AND receipt.provider_binding_digest=NEW.provider_binding_digest
             AND receipt.candidate_id=NEW.candidate_id
             AND receipt.candidate_digest=NEW.candidate_digest
             AND receipt.profile_id=NEW.profile_id
             AND receipt.profile_digest=NEW.profile_digest
             AND receipt.target_id=NEW.target_id
             AND receipt.target_digest=NEW.target_digest
             AND receipt.companion_id=NEW.companion_id
             AND receipt.companion_digest=NEW.companion_digest
             AND receipt.provider_id=NEW.provider_id)
        BEGIN SELECT RAISE(ABORT,'V270 revocation lacks the exact durable readiness receipt'); END;
        "#,
    )?;
    Ok(())
}
