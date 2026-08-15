use anyhow::Result;
use rusqlite::{functions::FunctionFlags, Connection};

use crate::compute_federation::external_pool_adapter_runtime_compatibility_verification::*;

const CHALLENGE_EXACT: &str = "elon_v268_runtime_compatibility_challenge_is_exact";
const OBSERVATION_EXACT: &str = "elon_v268_runtime_compatibility_observation_is_exact";
const VERIFICATION_EXACT: &str = "elon_v268_runtime_compatibility_verification_is_exact";
const REVOCATION_EXACT: &str = "elon_v268_runtime_compatibility_revocation_is_exact";

pub(super) fn register(conn: &Connection) -> Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    conn.create_scalar_function(CHALLENGE_EXACT, 1, flags, |context| {
        Ok(i64::from(text(context, 0).is_some_and(challenge_is_exact)))
    })?;
    conn.create_scalar_function(OBSERVATION_EXACT, 2, flags, |context| {
        Ok(i64::from(
            text(context, 0)
                .zip(text(context, 1))
                .is_some_and(|(challenge, observation)| {
                    observation_is_exact(challenge, observation)
                }),
        ))
    })?;
    conn.create_scalar_function(VERIFICATION_EXACT, 4, flags, |context| {
        Ok(i64::from(
            text(context, 0)
                .zip(text(context, 1))
                .zip(text(context, 2))
                .zip(text(context, 3))
                .is_some_and(|(((challenge, observation), verification), public_key)| {
                    verification_is_exact(challenge, observation, verification, public_key)
                }),
        ))
    })?;
    conn.create_scalar_function(REVOCATION_EXACT, 1, flags, |context| {
        Ok(i64::from(text(context, 0).is_some_and(revocation_is_exact)))
    })?;
    Ok(())
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_challenge_integrity
         BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_challenges
         WHEN {CHALLENGE_EXACT}(NEW.challenge_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V268 challenge canonical/current-root integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_observation_integrity
         BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_run_observations
         WHEN {OBSERVATION_EXACT}(
           (SELECT challenge_json FROM compute_external_pool_adapter_runtime_compatibility_verification_challenges WHERE challenge_id=NEW.challenge_id),
           NEW.run_observation_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V268 observation canonical/challenge integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_verification_integrity
         BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_receipts
         WHEN {VERIFICATION_EXACT}(
           (SELECT challenge_json FROM compute_external_pool_adapter_runtime_compatibility_verification_challenges WHERE challenge_id=NEW.challenge_id),
           (SELECT run_observation_json FROM compute_external_pool_adapter_runtime_compatibility_verification_run_observations WHERE run_observation_id=NEW.run_observation_id),
           NEW.verification_receipt_json,
           (SELECT public_key_pem FROM compute_external_pool_adapter_sandbox_verifier_keys
             WHERE key_record_id=NEW.sandbox_verifier_key_record_id
               AND key_record_digest=NEW.sandbox_verifier_key_record_digest
               AND key_id=NEW.sandbox_verifier_key_id)) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V268 verification signature/canonical integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v268_runtime_compatibility_revocation_integrity
         BEFORE INSERT ON compute_external_pool_adapter_runtime_compatibility_verification_revocations
         WHEN {REVOCATION_EXACT}(NEW.revocation_receipt_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V268 revocation canonical integrity mismatch'); END;"
    ))?;
    Ok(())
}

fn text<'a>(context: &'a rusqlite::functions::Context<'a>, index: usize) -> Option<&'a str> {
    context.get_raw(index).as_str().ok()
}

fn challenge_is_exact(json: &str) -> bool {
    let Ok(receipt) =
        bounded_parse::<ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt>(json)
    else {
        return false;
    };
    validate_runtime_compatibility_challenge_receipt(&receipt).is_ok()
        && validate_runtime_compatibility_challenge_current_roots(&receipt.challenge).is_ok()
        && runtime_compatibility_challenge_json_and_digest(&receipt)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn observation_is_exact(challenge_json: &str, observation_json: &str) -> bool {
    let (Ok(challenge), Ok(observation)) = (
        bounded_parse::<ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt>(challenge_json),
        bounded_parse::<ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt>(
            observation_json,
        ),
    ) else {
        return false;
    };
    validate_runtime_compatibility_run_observation_receipt(&observation).is_ok()
        && validate_runtime_compatibility_observation_against_challenge(
            &observation.observation,
            &challenge,
        )
        .is_ok()
        && runtime_compatibility_observation_json_and_digest(&observation)
            .is_ok_and(|(canonical, _)| canonical == observation_json)
}

fn verification_is_exact(
    challenge_json: &str,
    observation_json: &str,
    verification_json: &str,
    public_key_pem: &str,
) -> bool {
    let (Ok(challenge), Ok(observation), Ok(verification)) = (
        bounded_parse::<ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt>(challenge_json),
        bounded_parse::<ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt>(
            observation_json,
        ),
        bounded_parse::<ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt>(
            verification_json,
        ),
    ) else {
        return false;
    };
    let Ok(signature_challenge) =
        runtime_compatibility_signature_challenge(&challenge, &observation)
    else {
        return false;
    };
    validate_runtime_compatibility_verification_receipt(&verification, &challenge, &observation)
        .is_ok()
        && verify_runtime_compatibility_signature(
            public_key_pem,
            &signature_challenge,
            &verification.verification.signature_base64,
        )
        .is_ok()
        && runtime_compatibility_verification_receipt_json_and_digest(&verification)
            .is_ok_and(|(canonical, _)| canonical == verification_json)
}

fn revocation_is_exact(json: &str) -> bool {
    let Ok(receipt) =
        bounded_parse::<ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt>(json)
    else {
        return false;
    };
    validate_runtime_compatibility_revocation_receipt(&receipt).is_ok()
        && runtime_compatibility_revocation_receipt_json_and_digest(&receipt)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn bounded_parse<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
    if json.len() > RUNTIME_COMPATIBILITY_VERIFICATION_MAX_RECEIPT_JSON_BYTES {
        anyhow::bail!("V268 receipt exceeds the durable bound");
    }
    Ok(serde_json::from_str(json)?)
}
