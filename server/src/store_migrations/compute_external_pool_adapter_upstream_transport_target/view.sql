DROP VIEW IF EXISTS compute_external_pool_adapter_upstream_transport_target_current;

CREATE VIEW compute_external_pool_adapter_upstream_transport_target_current AS
WITH heads AS (
  SELECT target.*
    FROM compute_external_pool_adapter_upstream_transport_targets target
   WHERE NOT EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets successor
      WHERE successor.predecessor_target_id=target.target_id)
)
SELECT head.*,
       revocation.revocation_id,
       revocation.revocation_digest,
       revocation.revoked_at,
       'head' AS head_status,
       CASE WHEN revocation.revocation_id IS NULL
                  AND current_profile.current_status='launch_profile_current_inert'
                  AND current_profile.head_status='head'
                  AND current_profile.revocation_status='none'
                  AND current_profile.runtime_launch_ready=0
                  AND head.target_policy_digest=__POLICY_DIGEST_SQL__
                  AND json(head.target_policy_json)=json(__POLICY_JSON_SQL__)
                  AND json_extract(head.target_policy_json,'$.policy_id')=__POLICY_ID_SQL__
                  AND json_extract(head.target_policy_json,'$.policy_revision')=__POLICY_REVISION__
            THEN 'upstream_transport_target_current_inert'
            ELSE 'historical_only' END AS current_status,
       CASE WHEN current_profile.current_status='launch_profile_current_inert'
                  AND current_profile.head_status='head'
                  AND current_profile.revocation_status='none'
                  AND current_profile.runtime_launch_ready=0
            THEN 'launch_profile_current_inert'
            ELSE 'historical_only' END AS profile_status,
       CASE WHEN head.target_policy_digest=__POLICY_DIGEST_SQL__
                  AND json(head.target_policy_json)=json(__POLICY_JSON_SQL__)
                  AND json_extract(head.target_policy_json,'$.policy_id')=__POLICY_ID_SQL__
                  AND json_extract(head.target_policy_json,'$.policy_revision')=__POLICY_REVISION__
            THEN 'server_policy_current' ELSE 'historical_only' END AS target_policy_status,
       CASE WHEN revocation.revocation_id IS NULL THEN 'unrevoked' ELSE 'revoked' END
         AS revocation_status
  FROM heads head
  LEFT JOIN compute_external_pool_adapter_runtime_launch_profile_current current_profile
    ON current_profile.profile_id=head.profile_id
   AND current_profile.profile_digest=head.profile_digest
  LEFT JOIN compute_external_pool_adapter_upstream_transport_target_revocations revocation
    ON revocation.target_id=head.target_id
   AND revocation.target_digest=head.target_digest;
