use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};

use crate::{
    esk_asset::platform::sui_address_binding::{
        assemble_challenge, AddressBindingChallenge, AddressBindingError, AddressBindingRecord,
        ChallengeMaterial, SignatureScheme, VerifiedWalletResponse,
    },
    store::Store,
};

use super::{
    authenticate_session_on, binding_id_for, binding_now, binding_receipt_for, map_read_error,
    parse_time, reverify_wallet_response,
};

impl Store {
    /// Loads private challenge material for the local verifier. HTTP handlers
    /// must never return this private store projection directly.
    pub(crate) fn load_esk_sui_address_binding_challenge(
        &self,
        user_id: &str,
        session_token: &str,
        challenge_id: &str,
    ) -> Result<AddressBindingChallenge> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
        authenticate_session_on(&tx, user_id, session_token, &binding_now()?)?;
        let challenge =
            challenge_by_id_on(&tx, user_id, challenge_id)?.ok_or(AddressBindingError::NotFound)?;
        authenticate_session_on(&tx, user_id, session_token, &binding_now()?)?;
        tx.commit()?;
        Ok(challenge)
    }

    pub(crate) fn get_esk_sui_address_binding(
        &self,
        user_id: &str,
        session_token: &str,
    ) -> Result<Option<AddressBindingRecord>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
        authenticate_session_on(&tx, user_id, session_token, &binding_now()?)?;
        let binding = binding_on(&tx, user_id, None)?;
        authenticate_session_on(&tx, user_id, session_token, &binding_now()?)?;
        tx.commit()?;
        Ok(binding)
    }
}

pub(super) fn subject_for_user_on(conn: &Connection, user_id: &str) -> Result<Option<String>> {
    let subject = conn
        .query_row(
            "SELECT subject_commitment FROM esk_platform_sui_subjects WHERE user_id=?1",
            params![user_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(map_read_error)?;
    if subject.as_deref().is_some_and(|value| !valid_digest(value)) {
        return Err(AddressBindingError::CorruptLedger.into());
    }
    Ok(subject)
}

pub(super) fn challenge_by_id_on(
    conn: &Connection,
    user_id: &str,
    challenge_id: &str,
) -> Result<Option<AddressBindingChallenge>> {
    let stored = conn
        .query_row(
            "SELECT schema,challenge_id,network,purpose,subject_commitment,address,
                    ttl_seconds,nonce_base64,issued_at,expires_at,message_base64,message_sha256
               FROM esk_platform_sui_address_binding_challenges
              WHERE user_id=?1 AND challenge_id=?2",
            params![user_id, challenge_id],
            StoredChallenge::from_row,
        )
        .optional()
        .map_err(map_read_error)?;
    stored.map(StoredChallenge::validated).transpose()
}

pub(super) fn live_challenge_for_address_on(
    conn: &Connection,
    user_id: &str,
    address: &str,
    checked_at: &str,
) -> Result<Option<AddressBindingChallenge>> {
    let stored = conn
        .query_row(
            "SELECT schema,challenge_id,network,purpose,subject_commitment,address,
                    ttl_seconds,nonce_base64,issued_at,expires_at,message_base64,message_sha256
               FROM esk_platform_sui_address_binding_challenges
              WHERE user_id=?1 AND address=?2
                AND julianday(issued_at)<=julianday(?3)
                AND julianday(expires_at)>julianday(?3)
              ORDER BY recorded_at DESC,challenge_id DESC LIMIT 1",
            params![user_id, address, checked_at],
            StoredChallenge::from_row,
        )
        .optional()
        .map_err(map_read_error)?;
    stored.map(StoredChallenge::validated).transpose()
}

pub(super) fn challenge_rate_counts_on(
    conn: &Connection,
    user_id: &str,
    checked_at: &str,
) -> Result<(i64, i64)> {
    conn.query_row(
        "SELECT
           (SELECT COUNT(*) FROM esk_platform_sui_address_binding_challenges
             WHERE user_id=?1 AND julianday(issued_at)<=julianday(?2)
               AND julianday(expires_at)>julianday(?2)),
           (SELECT COUNT(*) FROM esk_platform_sui_address_binding_challenges
             WHERE user_id=?1 AND julianday(recorded_at)>julianday(?2,'-24 hours')
               AND julianday(recorded_at)<=julianday(?2))",
        params![user_id, checked_at],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(map_read_error)
}

pub(super) fn binding_on(
    conn: &Connection,
    user_id: &str,
    challenge_id: Option<&str>,
) -> Result<Option<AddressBindingRecord>> {
    let stored = conn
        .query_row(
            "SELECT binding_id,challenge_id,user_id,address,network,subject_commitment,
                    message_sha256,signature_scheme,signature_sha256,response_digest,
                    binding_receipt_sha256,wallet_response_json,verified_at,bound_at
               FROM esk_platform_sui_address_bindings
              WHERE user_id=?1 AND (?2 IS NULL OR challenge_id=?2)",
            params![user_id, challenge_id],
            StoredBinding::from_row,
        )
        .optional()
        .map_err(map_read_error)?;
    stored.map(|stored| stored.validated(conn)).transpose()
}

pub(super) fn binding_exists_for_address_on(conn: &Connection, address: &str) -> Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM esk_platform_sui_address_bindings WHERE address=?1)",
        params![address],
        |row| row.get(0),
    )
    .map_err(map_read_error)
}

struct StoredChallenge {
    schema: String,
    challenge_id: String,
    network: String,
    purpose: String,
    subject_commitment: String,
    address: String,
    ttl_seconds: i64,
    nonce_base64: String,
    issued_at: String,
    expires_at: String,
    message_base64: String,
    message_sha256: String,
}

impl StoredChallenge {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            schema: row.get(0)?,
            challenge_id: row.get(1)?,
            network: row.get(2)?,
            purpose: row.get(3)?,
            subject_commitment: row.get(4)?,
            address: row.get(5)?,
            ttl_seconds: row.get(6)?,
            nonce_base64: row.get(7)?,
            issued_at: row.get(8)?,
            expires_at: row.get(9)?,
            message_base64: row.get(10)?,
            message_sha256: row.get(11)?,
        })
    }

    fn validated(self) -> Result<AddressBindingChallenge> {
        let ttl_seconds =
            u32::try_from(self.ttl_seconds).map_err(|_| AddressBindingError::CorruptLedger)?;
        let challenge = AddressBindingChallenge {
            schema: self.schema,
            challenge_id: self.challenge_id,
            network: self.network,
            purpose: self.purpose,
            subject_commitment: self.subject_commitment,
            address: self.address,
            ttl_seconds,
            nonce_base64: self.nonce_base64,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            message_base64: self.message_base64,
            message_sha256: self.message_sha256,
        };
        let rebuilt = assemble_challenge(
            &challenge.subject_commitment,
            &ChallengeMaterial {
                address: challenge.address.clone(),
                ttl_seconds: challenge.ttl_seconds,
                nonce_base64: challenge.nonce_base64.clone(),
                issued_at: challenge.issued_at.clone(),
                expires_at: challenge.expires_at.clone(),
            },
        )
        .map_err(|_| AddressBindingError::CorruptLedger)?;
        if challenge != rebuilt {
            return Err(AddressBindingError::CorruptLedger.into());
        }
        Ok(challenge)
    }
}

struct StoredBinding {
    binding_id: String,
    challenge_id: String,
    user_id: String,
    address: String,
    network: String,
    subject_commitment: String,
    message_sha256: String,
    signature_scheme: String,
    signature_sha256: String,
    response_digest: String,
    binding_receipt_sha256: String,
    wallet_response_json: String,
    verified_at: String,
    bound_at: String,
}

impl StoredBinding {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            binding_id: row.get(0)?,
            challenge_id: row.get(1)?,
            user_id: row.get(2)?,
            address: row.get(3)?,
            network: row.get(4)?,
            subject_commitment: row.get(5)?,
            message_sha256: row.get(6)?,
            signature_scheme: row.get(7)?,
            signature_sha256: row.get(8)?,
            response_digest: row.get(9)?,
            binding_receipt_sha256: row.get(10)?,
            wallet_response_json: row.get(11)?,
            verified_at: row.get(12)?,
            bound_at: row.get(13)?,
        })
    }

    fn validated(self, conn: &Connection) -> Result<AddressBindingRecord> {
        let challenge = challenge_by_id_on(conn, &self.user_id, &self.challenge_id)?
            .ok_or(AddressBindingError::CorruptLedger)?;
        if self.address != challenge.address
            || self.network != challenge.network
            || self.subject_commitment != challenge.subject_commitment
            || self.message_sha256 != challenge.message_sha256
        {
            return Err(AddressBindingError::CorruptLedger.into());
        }
        let verified_at = parse_time(&self.verified_at, true)?;
        let bound_at = parse_time(&self.bound_at, true)?;
        let issued_at = parse_time(&challenge.issued_at, true)?;
        let expires_at = parse_time(&challenge.expires_at, true)?;
        if verified_at < issued_at
            || verified_at >= expires_at
            || bound_at < verified_at
            || bound_at >= expires_at
        {
            return Err(AddressBindingError::CorruptLedger.into());
        }
        let verified = VerifiedWalletResponse {
            challenge_id: self.challenge_id.clone(),
            address: self.address.clone(),
            subject_commitment: self.subject_commitment.clone(),
            message_sha256: self.message_sha256.clone(),
            signature_scheme: SignatureScheme::parse(&self.signature_scheme)?,
            signature_sha256: self.signature_sha256.clone(),
            response_digest: self.response_digest.clone(),
            verified_at: self.verified_at.clone(),
            wallet_response_json: self.wallet_response_json.clone(),
        };
        reverify_wallet_response(&challenge, &verified, true)?;
        let expected_binding_id = binding_id_for(&self.challenge_id, &self.response_digest);
        let expected_receipt =
            binding_receipt_for(&expected_binding_id, &challenge, &verified, &self.bound_at);
        if self.binding_id != expected_binding_id || self.binding_receipt_sha256 != expected_receipt
        {
            return Err(AddressBindingError::CorruptLedger.into());
        }
        Ok(AddressBindingRecord {
            binding_id: self.binding_id,
            user_id: self.user_id,
            challenge_id: self.challenge_id,
            address: self.address,
            network: self.network,
            subject_commitment: self.subject_commitment,
            message_sha256: self.message_sha256,
            signature_scheme: verified.signature_scheme,
            signature_sha256: self.signature_sha256,
            response_digest: self.response_digest,
            binding_receipt_sha256: self.binding_receipt_sha256,
            wallet_response_json: self.wallet_response_json,
            issued_at: challenge.issued_at,
            expires_at: challenge.expires_at,
            verified_at: self.verified_at,
            bound_at: self.bound_at,
            replayed: false,
        })
    }
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|body| {
        body.len() == 64
            && body.bytes().any(|byte| byte != b'0')
            && body
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
