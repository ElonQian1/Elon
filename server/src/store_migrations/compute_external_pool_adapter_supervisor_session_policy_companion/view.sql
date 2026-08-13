DROP VIEW IF EXISTS compute_external_pool_adapter_supervisor_session_policy_companion_current;

CREATE VIEW compute_external_pool_adapter_supervisor_session_policy_companion_current AS
WITH heads AS (
  SELECT companion.*
    FROM compute_external_pool_adapter_supervisor_session_policy_companions companion
   WHERE NOT EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companions successor
      WHERE successor.predecessor_companion_id=companion.companion_id)
)
SELECT head.*,
       revocation.revocation_id,
       revocation.revocation_digest,
       revocation.revoked_at,
       'head' AS head_status,
       CASE WHEN revocation.revocation_id IS NULL
                  AND current_target.current_status='upstream_transport_target_current_inert'
                  AND current_target.head_status='head'
                  AND current_target.revocation_status='unrevoked'
                  AND current_target.profile_status='launch_profile_current_inert'
                  AND current_target.target_policy_status='server_policy_current'
                  AND head.supervisor_session_policy_digest=__POLICY_DIGEST_SQL__
                  AND head.supervisor_session_policy_json=__POLICY_JSON_SQL__
                  AND json_extract(head.supervisor_session_policy_json,'$.policy_id')=__POLICY_ID_SQL__
                  AND json_extract(head.supervisor_session_policy_json,'$.policy_revision')=__POLICY_REVISION__
            THEN 'supervisor_session_policy_companion_current_inert'
            ELSE 'historical_only' END AS current_status,
       CASE WHEN current_target.current_status='upstream_transport_target_current_inert'
                  AND current_target.head_status='head'
                  AND current_target.revocation_status='unrevoked'
            THEN 'upstream_transport_target_current_inert'
            ELSE 'historical_only' END AS target_status,
       CASE WHEN current_target.profile_status='launch_profile_current_inert'
            THEN 'launch_profile_current_inert'
            ELSE 'historical_only' END AS profile_status,
       CASE WHEN head.supervisor_session_policy_digest=__POLICY_DIGEST_SQL__
                  AND head.supervisor_session_policy_json=__POLICY_JSON_SQL__
                  AND json_extract(head.supervisor_session_policy_json,'$.policy_id')=__POLICY_ID_SQL__
                  AND json_extract(head.supervisor_session_policy_json,'$.policy_revision')=__POLICY_REVISION__
            THEN 'server_policy_current' ELSE 'historical_only' END AS policy_status,
       CASE WHEN revocation.revocation_id IS NULL THEN 'unrevoked' ELSE 'revoked' END
         AS revocation_status
  FROM heads head
  LEFT JOIN compute_external_pool_adapter_upstream_transport_target_current current_target
    ON current_target.target_id=head.target_id
   AND current_target.target_digest=head.target_digest
  LEFT JOIN compute_external_pool_adapter_supervisor_session_policy_companion_revocations revocation
    ON revocation.companion_id=head.companion_id
   AND revocation.companion_digest=head.companion_digest;
