CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_challenge_current_authority
BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_challenges
WHEN NEW.profile_digest<>__PROFILE_DIGEST_SQL__
 OR NEW.runner_policy_digest<>__RUNNER_POLICY_DIGEST_SQL__
 OR NEW.fixture_catalog_digest<>__FIXTURE_CATALOG_DIGEST_SQL__
 OR NEW.idempotency_scope<>('v268:runtime-compatibility-challenge:'||NEW.created_by_admin_user_id)
 OR NOT EXISTS(SELECT 1 FROM users WHERE id=NEW.created_by_admin_user_id AND role IN ('admin','owner') AND status='active')
 OR abs((julianday(NEW.issued_at)-julianday('now'))*86400.0)>60.0
 OR NOT EXISTS (
   SELECT 1 FROM compute_external_pool_adapter_registry_release_current current
   WHERE current.registry_release_id=NEW.registry_release_id
     AND current.registry_release_digest=NEW.registry_release_digest
     AND current.current_status='release_current')
 OR NOT EXISTS (
   SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_key_current current
   WHERE current.key_record_id=NEW.sandbox_verifier_key_record_id
     AND current.key_record_digest=NEW.sandbox_verifier_key_record_digest
     AND current.key_id=NEW.sandbox_verifier_key_id
     AND current.verifier_operator=NEW.sandbox_verifier_operator
     AND current.verifier_product=NEW.sandbox_verifier_product
     AND current.current_status='active')
 OR NEW.sequence<>COALESCE((SELECT MAX(sequence)+1 FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts WHERE registry_release_id=NEW.registry_release_id),1)
 OR NEW.predecessor_verification_receipt_id IS NOT (SELECT verification_receipt_id FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts WHERE registry_release_id=NEW.registry_release_id ORDER BY sequence DESC LIMIT 1)
 OR NEW.predecessor_verification_receipt_digest IS NOT (SELECT verification_receipt_digest FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts WHERE registry_release_id=NEW.registry_release_id ORDER BY sequence DESC LIMIT 1)
BEGIN SELECT RAISE(ABORT,'V268 challenge current authority or predecessor mismatch'); END;

CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_observation_current_authority
BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_run_observations
WHEN abs((julianday(NEW.run_completed_at)-julianday('now'))*86400.0)>60.0
 OR julianday(NEW.run_started_at)<julianday((SELECT issued_at FROM compute_external_pool_adapter_runtime_compatibility_verification_challenges WHERE challenge_id=NEW.challenge_id))
 OR julianday(NEW.run_completed_at)>=julianday((SELECT expires_at FROM compute_external_pool_adapter_runtime_compatibility_verification_challenges WHERE challenge_id=NEW.challenge_id))
 OR julianday('now')>=julianday((SELECT expires_at FROM compute_external_pool_adapter_runtime_compatibility_verification_challenges WHERE challenge_id=NEW.challenge_id))
 OR NOT EXISTS (
   SELECT 1 FROM compute_external_pool_adapter_registry_release_current current
   WHERE current.registry_release_id=NEW.registry_release_id
     AND current.registry_release_digest=NEW.registry_release_digest
     AND current.current_status='release_current')
 OR NOT EXISTS (
   SELECT 1 FROM compute_external_pool_adapter_runtime_compatibility_verification_challenges challenge
   JOIN compute_external_pool_adapter_sandbox_verifier_key_current key
     ON key.key_record_id=challenge.sandbox_verifier_key_record_id
    AND key.key_record_digest=challenge.sandbox_verifier_key_record_digest
    AND key.key_id=challenge.sandbox_verifier_key_id
    AND key.current_status='active'
   WHERE challenge.challenge_id=NEW.challenge_id
     AND challenge.profile_digest=__PROFILE_DIGEST_SQL__
     AND challenge.runner_policy_digest=__RUNNER_POLICY_DIGEST_SQL__
     AND challenge.fixture_catalog_digest=__FIXTURE_CATALOG_DIGEST_SQL__)
BEGIN SELECT RAISE(ABORT,'V268 observation challenge is expired or no longer current'); END;

CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_verification_current_authority
BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_receipts
WHEN abs((julianday(NEW.verified_at)-julianday('now'))*86400.0)>60.0
 OR NEW.idempotency_scope<>('v268:runtime-compatibility-verify:'||NEW.verified_by_admin_user_id)
 OR NOT EXISTS(SELECT 1 FROM users WHERE id=NEW.verified_by_admin_user_id AND role IN ('admin','owner') AND status='active')
 OR julianday('now')>=julianday((SELECT expires_at FROM compute_external_pool_adapter_runtime_compatibility_verification_challenges WHERE challenge_id=NEW.challenge_id))
 OR NOT EXISTS (
   SELECT 1 FROM compute_external_pool_adapter_registry_release_current current
   WHERE current.registry_release_id=NEW.registry_release_id
     AND current.registry_release_digest=NEW.registry_release_digest
     AND current.current_status='release_current')
 OR NOT EXISTS (
   SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_key_current current
   WHERE current.key_record_id=NEW.sandbox_verifier_key_record_id
     AND current.key_record_digest=NEW.sandbox_verifier_key_record_digest
     AND current.key_id=NEW.sandbox_verifier_key_id
     AND current.current_status='active')
 OR NEW.profile_digest<>__PROFILE_DIGEST_SQL__
 OR NEW.runner_policy_digest<>__RUNNER_POLICY_DIGEST_SQL__
 OR NEW.fixture_catalog_digest<>__FIXTURE_CATALOG_DIGEST_SQL__
 OR NEW.sequence<>COALESCE((SELECT MAX(sequence)+1 FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts WHERE registry_release_id=NEW.registry_release_id),1)
 OR NEW.predecessor_verification_receipt_id IS NOT (SELECT verification_receipt_id FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts WHERE registry_release_id=NEW.registry_release_id ORDER BY sequence DESC LIMIT 1)
 OR NEW.predecessor_verification_receipt_digest IS NOT (SELECT verification_receipt_digest FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts WHERE registry_release_id=NEW.registry_release_id ORDER BY sequence DESC LIMIT 1)
BEGIN SELECT RAISE(ABORT,'V268 verification current authority or lineage mismatch'); END;

CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_revocation_head_only
BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_revocations
WHEN abs((julianday(NEW.revoked_at)-julianday('now'))*86400.0)>60.0
 OR NEW.idempotency_scope<>('v268:runtime-compatibility-revoke:'||NEW.revoked_by_admin_user_id)
 OR NOT EXISTS(SELECT 1 FROM users WHERE id=NEW.revoked_by_admin_user_id AND role IN ('admin','owner') AND status='active')
 OR NOT EXISTS(
   SELECT 1 FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts verification
   WHERE verification.verification_receipt_id=NEW.verification_receipt_id
     AND verification.verification_receipt_digest=NEW.verification_receipt_digest
     AND verification.registry_release_id=NEW.registry_release_id
     AND verification.registry_release_digest=NEW.registry_release_digest)
 OR NEW.verification_receipt_id IS NOT (
   SELECT verification_receipt_id FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts
   WHERE registry_release_id=NEW.registry_release_id ORDER BY sequence DESC LIMIT 1)
 OR NEW.verification_receipt_digest IS NOT (
   SELECT verification_receipt_digest FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts
   WHERE registry_release_id=NEW.registry_release_id ORDER BY sequence DESC LIMIT 1)
BEGIN SELECT RAISE(ABORT,'V268 only the exact verification lineage head may be revoked'); END;
