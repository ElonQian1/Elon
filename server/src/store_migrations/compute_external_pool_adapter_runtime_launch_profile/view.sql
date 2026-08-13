DROP VIEW IF EXISTS compute_external_pool_adapter_runtime_launch_profile_current;

CREATE VIEW compute_external_pool_adapter_runtime_launch_profile_current AS
WITH heads AS (
  SELECT profile.*
    FROM compute_external_pool_adapter_runtime_launch_profiles profile
   WHERE NOT EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_runtime_launch_profiles successor
      WHERE successor.predecessor_profile_id=profile.profile_id)
), display AS (
  SELECT head.*,
         current_binding.current_status AS binding_current_status,
         current_binding.projection_status AS projection_status,
         current_release.current_status AS release_current_status,
         current_installation.current_status AS installation_current_status,
         provider.status AS live_provider_status,
         provider.current_policy_revision AS live_provider_policy_revision,
         provider.current_provider_digest AS live_provider_digest,
         candidate.candidate_id AS exact_candidate_id,
         CASE WHEN candidate.candidate_id IS NOT NULL
                    AND delegation.delegation_id IS NOT NULL
                    AND NOT EXISTS (
                      SELECT 1 FROM compute_external_pool_provider_activation_candidates later
                       WHERE later.provider_binding_id=candidate.provider_binding_id
                         AND later.sequence>candidate.sequence)
                    AND NOT EXISTS (
                      SELECT 1 FROM compute_external_pool_provider_activation_delegations later_delegation
                       WHERE later_delegation.provider_binding_id=candidate.provider_binding_id
                         AND later_delegation.sequence>delegation.sequence)
                    AND NOT EXISTS (
                      SELECT 1 FROM compute_external_pool_provider_activation_delegation_revocations revoked
                       WHERE revoked.delegation_id=candidate.delegation_id
                         AND revoked.delegation_digest=candidate.delegation_digest)
              THEN 'candidate_current_not_activation_ready'
              ELSE 'historical_only' END AS live_candidate_status
    FROM heads head
    LEFT JOIN compute_external_pool_adapter_registry_provider_binding_current current_binding
      ON current_binding.provider_binding_id=head.provider_binding_id
     AND current_binding.provider_binding_digest=head.provider_binding_digest
    LEFT JOIN compute_external_pool_adapter_registry_release_current current_release
      ON current_release.registry_release_id=head.registry_release_id
     AND current_release.registry_release_digest=head.registry_release_digest
    LEFT JOIN compute_external_pool_adapter_installation_current current_installation
      ON current_installation.installation_receipt_id=head.installation_receipt_id
     AND current_installation.installation_receipt_digest=head.installation_receipt_digest
    LEFT JOIN compute_providers provider
      ON provider.provider_id=head.provider_id
     AND provider.provider_kind='external_pool'
     AND provider.owner_account_id=head.provider_owner_account_id
    LEFT JOIN compute_external_pool_provider_activation_candidates candidate
      ON candidate.candidate_id=head.candidate_id
     AND candidate.candidate_digest=head.candidate_digest
     AND candidate.delegation_id=head.delegation_id
     AND candidate.delegation_digest=head.delegation_digest
     AND candidate.provider_binding_id=head.provider_binding_id
     AND candidate.provider_binding_digest=head.provider_binding_digest
     AND candidate.candidate_status='candidate_current_not_activation_ready'
     AND candidate.activation_closure_status='activation_closure_not_implemented'
    LEFT JOIN compute_external_pool_provider_activation_delegations delegation
      ON delegation.delegation_id=candidate.delegation_id
     AND delegation.delegation_digest=candidate.delegation_digest
     AND delegation.provider_binding_id=candidate.provider_binding_id
)
SELECT 'compute_federation.external_pool_adapter_runtime_launch_profile_currentness.v1'
         AS currentness_schema,
       display.profile_id,
       display.profile_digest,
       display.profile_material_digest,
       display.provider_binding_id,
       display.provider_binding_digest,
       display.candidate_id,
       display.candidate_digest,
       display.provider_id,
       display.provider_status AS observed_provider_status,
       display.provider_policy_revision AS observed_provider_policy_revision,
       display.provider_digest AS observed_provider_digest,
       display.entrypoint_path_digest,
       display.entrypoint_sha256,
       display.entrypoint_size_bytes,
       display.entry_inventory_digest,
       display.installed_file_count,
       display.installed_total_bytes,
       json_extract(display.launch_policy_json,'$.policy_id') AS launch_policy_id,
       json_extract(display.launch_policy_json,'$.policy_revision') AS launch_policy_revision,
       display.launch_policy_digest,
       display.sequence,
       display.recorded_at,
       CASE WHEN display.binding_current_status='binding_current'
                  AND display.projection_status='reserved'
                  AND display.release_current_status='release_current'
                  AND display.installation_current_status='installed_upstreams_current'
                  AND display.live_candidate_status='candidate_current_not_activation_ready'
                  AND display.live_provider_status='registering'
                  AND display.live_provider_policy_revision=display.provider_policy_revision
                  AND display.live_provider_digest=display.provider_digest
                  AND revocation.revocation_id IS NULL
            THEN 'launch_profile_current_inert' ELSE 'historical_only' END AS current_status,
       'head' AS head_status,
       COALESCE(display.binding_current_status,'historical_only') AS provider_binding_status,
       COALESCE(display.release_current_status,'historical_only') AS registry_release_status,
       COALESCE(display.installation_current_status,'historical_only') AS installation_status,
       display.live_candidate_status AS candidate_status,
       CASE WHEN display.live_provider_status='registering'
                  AND display.live_provider_policy_revision=display.provider_policy_revision
                  AND display.live_provider_digest=display.provider_digest
            THEN 'exact_registering' ELSE 'historical_only' END AS provider_revision_status,
       CASE WHEN revocation.revocation_id IS NULL THEN 'none' ELSE 'revoked' END AS revocation_status,
       0 AS runtime_launch_ready
  FROM display
  LEFT JOIN compute_external_pool_adapter_runtime_launch_profile_revocations revocation
    ON revocation.profile_id=display.profile_id
   AND revocation.profile_digest=display.profile_digest;
