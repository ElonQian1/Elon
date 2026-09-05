use anyhow::Result;
use rusqlite::{params, TransactionBehavior};

use crate::{
    esk_asset::platform::sui_address_binding::{
        assemble_challenge, AddressBindingChallenge, AddressBindingError, AddressBindingRecord,
        ChallengeMaterial, VerifiedWalletResponse,
    },
    store::Store,
};

use super::{
    authenticate_session_on, binding_id_for, binding_now, binding_receipt_for,
    ensure_challenge_live_at, fresh_subject_commitment, map_insert_error, parse_time,
    read::{
        binding_exists_for_address_on, binding_on, challenge_by_id_on, challenge_rate_counts_on,
        live_challenge_for_address_on, subject_for_user_on,
    },
    reverify_wallet_response,
};

const MAX_LIVE_CHALLENGES_PER_USER: i64 = 3;
const MAX_CHALLENGES_PER_ROLLING_DAY: i64 = 20;

impl Store {
    pub(crate) fn create_esk_sui_address_binding_challenge(
        &self,
        user_id: &str,
        session_token: &str,
        material: &ChallengeMaterial,
    ) -> Result<AddressBindingChallenge> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let recorded_at = binding_now()?;
        let session_id = authenticate_session_on(&tx, user_id, session_token, &recorded_at)?;

        if binding_on(&tx, user_id, None)?.is_some()
            || binding_exists_for_address_on(&tx, &material.address)?
        {
            return Err(AddressBindingError::Conflict.into());
        }

        if let Some(challenge) =
            live_challenge_for_address_on(&tx, user_id, &material.address, &recorded_at)?
        {
            recheck_before_commit(&tx, user_id, session_token, &session_id, &challenge)?;
            tx.commit().map_err(map_insert_error)?;
            return Ok(challenge);
        }
        let (live_count, rolling_day_count) = challenge_rate_counts_on(&tx, user_id, &recorded_at)?;
        if live_count >= MAX_LIVE_CHALLENGES_PER_USER
            || rolling_day_count >= MAX_CHALLENGES_PER_ROLLING_DAY
        {
            return Err(AddressBindingError::RateLimited.into());
        }

        let subject = match subject_for_user_on(&tx, user_id)? {
            Some(subject) => subject,
            None => {
                let subject = fresh_subject_commitment()?;
                tx.execute(
                    "INSERT INTO esk_platform_sui_subjects(
                       user_id,subject_commitment,created_session_id,created_at
                     ) VALUES(?1,?2,?3,?4)",
                    params![user_id, subject, session_id, recorded_at],
                )
                .map_err(map_insert_error)?;
                let stored =
                    subject_for_user_on(&tx, user_id)?.ok_or(AddressBindingError::CorruptLedger)?;
                if stored != subject {
                    return Err(AddressBindingError::CorruptLedger.into());
                }
                subject
            }
        };

        let challenge = assemble_challenge(&subject, material)?;
        ensure_challenge_live_at(&challenge, &recorded_at)?;
        if let Some(stored) = challenge_by_id_on(&tx, user_id, &challenge.challenge_id)? {
            if stored != challenge {
                return Err(AddressBindingError::Conflict.into());
            }
            recheck_before_commit(&tx, user_id, session_token, &session_id, &challenge)?;
            tx.commit().map_err(map_insert_error)?;
            return Ok(stored);
        }

        tx.execute(
            "INSERT INTO esk_platform_sui_address_binding_challenges(
               challenge_id,user_id,subject_commitment,created_session_id,schema,network,purpose,
               address,ttl_seconds,nonce_base64,issued_at,expires_at,message_base64,
               message_sha256,recorded_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                challenge.challenge_id,
                user_id,
                challenge.subject_commitment,
                session_id,
                challenge.schema,
                challenge.network,
                challenge.purpose,
                challenge.address,
                i64::from(challenge.ttl_seconds),
                challenge.nonce_base64,
                challenge.issued_at,
                challenge.expires_at,
                challenge.message_base64,
                challenge.message_sha256,
                recorded_at,
            ],
        )
        .map_err(map_insert_error)?;

        let stored = challenge_by_id_on(&tx, user_id, &challenge.challenge_id)?
            .ok_or(AddressBindingError::CorruptLedger)?;
        if stored != challenge {
            return Err(AddressBindingError::CorruptLedger.into());
        }
        recheck_before_commit(&tx, user_id, session_token, &session_id, &challenge)?;
        tx.commit().map_err(map_insert_error)?;
        Ok(stored)
    }

    pub(crate) fn complete_esk_sui_address_binding(
        &self,
        user_id: &str,
        session_token: &str,
        challenge_id: &str,
        verified: &VerifiedWalletResponse,
    ) -> Result<AddressBindingRecord> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let bound_at = binding_now()?;
        let session_id = authenticate_session_on(&tx, user_id, session_token, &bound_at)?;
        let challenge =
            challenge_by_id_on(&tx, user_id, challenge_id)?.ok_or(AddressBindingError::NotFound)?;

        reverify_wallet_response(&challenge, verified, false)?;
        if let Some(mut stored) = binding_on(&tx, user_id, Some(challenge_id))? {
            if !is_exact_response_replay(&stored, verified) {
                return Err(AddressBindingError::Conflict.into());
            }
            recheck_before_commit(&tx, user_id, session_token, &session_id, &challenge)?;
            stored.replayed = true;
            tx.commit().map_err(map_insert_error)?;
            return Ok(stored);
        }

        if verified.challenge_id != challenge.challenge_id
            || verified.address != challenge.address
            || verified.subject_commitment != challenge.subject_commitment
            || verified.message_sha256 != challenge.message_sha256
        {
            return Err(AddressBindingError::Conflict.into());
        }
        ensure_challenge_live_at(&challenge, &bound_at)?;
        let verified_at = parse_time(&verified.verified_at, false)?;
        let transaction_time = parse_time(&bound_at, true)?;
        if verified_at > transaction_time {
            return Err(AddressBindingError::InvalidResponse.into());
        }
        if binding_on(&tx, user_id, None)?.is_some()
            || binding_exists_for_address_on(&tx, &challenge.address)?
        {
            return Err(AddressBindingError::Conflict.into());
        }

        let binding_id = binding_id_for(&challenge.challenge_id, &verified.response_digest);
        let binding_receipt_sha256 =
            binding_receipt_for(&binding_id, &challenge, verified, &bound_at);
        tx.execute(
            "INSERT INTO esk_platform_sui_address_bindings(
               binding_id,challenge_id,user_id,address,network,subject_commitment,message_sha256,
               signature_scheme,signature_sha256,response_digest,binding_receipt_sha256,
               wallet_response_json,completed_session_id,verified_at,bound_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                binding_id,
                challenge.challenge_id,
                user_id,
                challenge.address,
                challenge.network,
                challenge.subject_commitment,
                challenge.message_sha256,
                verified.signature_scheme.as_str(),
                verified.signature_sha256,
                verified.response_digest,
                binding_receipt_sha256,
                verified.wallet_response_json,
                session_id,
                verified.verified_at,
                bound_at,
            ],
        )
        .map_err(map_insert_error)?;

        let stored = binding_on(&tx, user_id, Some(challenge_id))?
            .ok_or(AddressBindingError::CorruptLedger)?;
        if stored.binding_id != binding_id || !is_exact_response_replay(&stored, verified) {
            return Err(AddressBindingError::CorruptLedger.into());
        }
        recheck_before_commit(&tx, user_id, session_token, &session_id, &challenge)?;
        tx.commit().map_err(map_insert_error)?;
        Ok(stored)
    }
}

fn recheck_before_commit(
    conn: &rusqlite::Connection,
    user_id: &str,
    session_token: &str,
    expected_session_id: &str,
    challenge: &AddressBindingChallenge,
) -> Result<()> {
    let commit_at = binding_now()?;
    let current_session = authenticate_session_on(conn, user_id, session_token, &commit_at)?;
    if current_session != expected_session_id {
        return Err(AddressBindingError::Unauthorized.into());
    }
    ensure_challenge_live_at(challenge, &commit_at)
}

fn is_exact_response_replay(
    stored: &AddressBindingRecord,
    verified: &VerifiedWalletResponse,
) -> bool {
    stored.challenge_id == verified.challenge_id
        && stored.address == verified.address
        && stored.subject_commitment == verified.subject_commitment
        && stored.message_sha256 == verified.message_sha256
        && stored.signature_scheme == verified.signature_scheme
        && stored.signature_sha256 == verified.signature_sha256
        && stored.response_digest == verified.response_digest
        && stored.wallet_response_json == verified.wallet_response_json
}
