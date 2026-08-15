CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_challenge_no_replace
BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_challenges
WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_runtime_compatibility_verification_challenges old
 WHERE old.challenge_id=NEW.challenge_id OR old.challenge_digest=NEW.challenge_digest
    OR old.challenge_nonce_digest=NEW.challenge_nonce_digest
    OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
BEGIN SELECT RAISE(ABORT,'V268 challenge replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_observation_no_replace
BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_run_observations
WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_runtime_compatibility_verification_run_observations old
 WHERE old.run_observation_id=NEW.run_observation_id
    OR old.run_observation_digest=NEW.run_observation_digest
    OR old.runner_execution_id=NEW.runner_execution_id
    OR old.challenge_id=NEW.challenge_id)
BEGIN SELECT RAISE(ABORT,'V268 observation replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_verification_no_replace
BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_receipts
WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_runtime_compatibility_verification_receipts old
 WHERE old.verification_receipt_id=NEW.verification_receipt_id
    OR old.verification_receipt_digest=NEW.verification_receipt_digest
    OR old.runner_execution_id=NEW.runner_execution_id
    OR old.challenge_id=NEW.challenge_id
    OR old.run_observation_id=NEW.run_observation_id
    OR old.signature_digest=NEW.signature_digest
    OR (old.registry_release_id=NEW.registry_release_id AND old.sequence=NEW.sequence)
    OR (NEW.predecessor_verification_receipt_id IS NOT NULL
        AND old.registry_release_id=NEW.registry_release_id
        AND old.predecessor_verification_receipt_id=NEW.predecessor_verification_receipt_id)
    OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
BEGIN SELECT RAISE(ABORT,'V268 verification replacement is forbidden'); END;

CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_revocation_no_replace
BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_revocations
WHEN EXISTS(SELECT 1 FROM compute_external_pool_adapter_runtime_compatibility_verification_revocations old
 WHERE old.revocation_receipt_id=NEW.revocation_receipt_id
    OR old.revocation_receipt_digest=NEW.revocation_receipt_digest
    OR old.verification_receipt_id=NEW.verification_receipt_id
    OR (old.idempotency_scope=NEW.idempotency_scope AND old.idempotency_key=NEW.idempotency_key))
BEGIN SELECT RAISE(ABORT,'V268 revocation replacement is forbidden'); END;
