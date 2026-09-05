use anyhow::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use ring::rand::{SecureRandom, SystemRandom};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::esk_asset::platform::sui_address_binding::{
    verify_wallet_response, AddressBindingChallenge, AddressBindingError, VerifiedWalletResponse,
    WalletResponseBody,
};
use crate::store::Store;

use super::{hash_token, now};

mod read;
mod write;

impl Store {
    /// Authenticates the private HTTP boundary without collapsing database
    /// failures into an invalid-session response. Store mutations recheck the
    /// same session again inside their SQLite transaction.
    pub(crate) fn authenticate_esk_sui_address_binding_user_id(
        &self,
        session_token: &str,
    ) -> Result<String> {
        let session_token = session_token.trim();
        if session_token.is_empty() {
            return Err(AddressBindingError::Unauthorized.into());
        }
        let checked_at = binding_now()?;
        let conn = self
            .conn()
            .map_err(|_| anyhow::Error::from(AddressBindingError::Storage))?;
        conn.query_row(
            "SELECT u.id
               FROM sessions s JOIN users u ON u.id=s.user_id
              WHERE s.token_hash=?1 AND s.revoked_at IS NULL
                AND julianday(s.expires_at) IS NOT NULL
                AND julianday(s.expires_at)>julianday(?2)
                AND u.status='active' AND u.id<>'local-owner'",
            params![hash_token(session_token), checked_at],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_read_error)?
        .ok_or_else(|| AddressBindingError::Unauthorized.into())
    }
}

fn binding_now() -> Result<String> {
    let parsed =
        DateTime::parse_from_rfc3339(&now()).map_err(|_| AddressBindingError::CorruptLedger)?;
    Ok(parsed
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn parse_time(value: &str, ledger_value: bool) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| {
        if ledger_value {
            AddressBindingError::CorruptLedger
        } else {
            AddressBindingError::InvalidResponse
        }
    })?;
    let utc = parsed.with_timezone(&Utc);
    if utc.to_rfc3339_opts(SecondsFormat::Millis, true) != value {
        return Err(if ledger_value {
            AddressBindingError::CorruptLedger.into()
        } else {
            AddressBindingError::InvalidResponse.into()
        });
    }
    Ok(utc)
}

fn authenticate_session_on(
    conn: &Connection,
    user_id: &str,
    session_token: &str,
    checked_at: &str,
) -> Result<String> {
    if user_id == "local-owner" || session_token.trim().is_empty() {
        return Err(AddressBindingError::Unauthorized.into());
    }
    conn.query_row(
        "SELECT s.id FROM sessions s JOIN users u ON u.id=s.user_id
          WHERE u.id=?1 AND u.status='active' AND u.id<>'local-owner'
            AND s.token_hash=?2 AND s.revoked_at IS NULL
            AND julianday(s.expires_at) IS NOT NULL
            AND julianday(s.expires_at)>julianday(?3)",
        params![user_id, hash_token(session_token.trim()), checked_at],
        |row| row.get(0),
    )
    .optional()
    .map_err(map_read_error)?
    .ok_or_else(|| AddressBindingError::Unauthorized.into())
}

fn fresh_subject_commitment() -> Result<String> {
    let mut seed = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut seed)
        .map_err(|_| AddressBindingError::RandomUnavailable)?;
    let commitment = crate::esk_asset::platform::sui_address_binding::subject_commitment(&seed);
    seed.fill(0);
    Ok(commitment)
}

fn ensure_challenge_live_at(challenge: &AddressBindingChallenge, checked_at: &str) -> Result<()> {
    let issued_at = parse_time(&challenge.issued_at, true)?;
    let expires_at = parse_time(&challenge.expires_at, true)?;
    let checked_at = parse_time(checked_at, true)?;
    if checked_at < issued_at {
        return Err(AddressBindingError::NotYetValid.into());
    }
    if checked_at >= expires_at {
        return Err(AddressBindingError::Expired.into());
    }
    Ok(())
}

fn reverify_wallet_response(
    challenge: &AddressBindingChallenge,
    verified: &VerifiedWalletResponse,
    ledger_value: bool,
) -> Result<()> {
    let response: WalletResponseBody = serde_json::from_str(&verified.wallet_response_json)
        .map_err(|_| ledger_or_response_error(ledger_value))?;
    let canonical =
        serde_json::to_string(&response).map_err(|_| ledger_or_response_error(ledger_value))?;
    if canonical != verified.wallet_response_json {
        return Err(ledger_or_response_error(ledger_value).into());
    }
    let verified_at = parse_time(&verified.verified_at, ledger_value)?;
    let rebuilt = verify_wallet_response(challenge, &response, verified_at)
        .map_err(|_| ledger_or_response_error(ledger_value))?;
    if rebuilt != *verified {
        return Err(ledger_or_response_error(ledger_value).into());
    }
    Ok(())
}

fn binding_id_for(challenge_id: &str, response_digest: &str) -> String {
    let material = format!(
        "YILONG_ESK_SUI_PLATFORM_BINDING_ID_V2\nchallenge_id={challenge_id}\nresponse_digest={response_digest}"
    );
    let digest = hex::encode(Sha256::digest(material.as_bytes()));
    format!("eskpsb_{}", &digest[..32])
}

fn binding_receipt_for(
    binding_id: &str,
    challenge: &AddressBindingChallenge,
    verified: &VerifiedWalletResponse,
    bound_at: &str,
) -> String {
    let material = [
        "YILONG_ESK_SUI_PLATFORM_BINDING_RECEIPT_V2".to_owned(),
        format!("binding_id={binding_id}"),
        format!("challenge_id={}", challenge.challenge_id),
        format!("subject_commitment={}", challenge.subject_commitment),
        format!("address={}", challenge.address),
        format!("network={}", challenge.network),
        format!("message_sha256={}", challenge.message_sha256),
        format!("signature_scheme={}", verified.signature_scheme.as_str()),
        format!("signature_sha256={}", verified.signature_sha256),
        format!("response_digest={}", verified.response_digest),
        format!("verified_at={}", verified.verified_at),
        format!("bound_at={bound_at}"),
    ]
    .join("\n");
    format!(
        "sha256:{}",
        hex::encode(Sha256::digest(material.as_bytes()))
    )
}

fn ledger_or_response_error(ledger_value: bool) -> AddressBindingError {
    if ledger_value {
        AddressBindingError::CorruptLedger
    } else {
        AddressBindingError::InvalidResponse
    }
}

fn map_read_error(error: rusqlite::Error) -> anyhow::Error {
    use rusqlite::ErrorCode;

    match &error {
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.code,
                ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase
            ) =>
        {
            AddressBindingError::CorruptLedger.into()
        }
        rusqlite::Error::FromSqlConversionFailure(..)
        | rusqlite::Error::IntegralValueOutOfRange(..)
        | rusqlite::Error::InvalidColumnType(..) => AddressBindingError::CorruptLedger.into(),
        _ => AddressBindingError::Storage.into(),
    }
}

fn map_insert_error(error: rusqlite::Error) -> anyhow::Error {
    use rusqlite::ErrorCode;

    match &error {
        rusqlite::Error::SqliteFailure(failure, message)
            if failure.code == ErrorCode::ConstraintViolation =>
        {
            if message
                .as_deref()
                .is_some_and(|value| value.contains("ESK Sui challenge rate limited"))
            {
                AddressBindingError::RateLimited.into()
            } else {
                AddressBindingError::Conflict.into()
            }
        }
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.code,
                ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase
            ) =>
        {
            AddressBindingError::CorruptLedger.into()
        }
        _ => AddressBindingError::Storage.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::esk_asset::platform::sui_address_binding::{ChallengeMaterial, SignatureScheme};

    #[test]
    fn deterministic_private_id_and_public_receipt_match_fixed_vector() {
        let challenge_id = "eab1_0123456789abcdef0123456789abcdef";
        let response_digest = format!("sha256:{}", "1".repeat(64));
        let binding_id = binding_id_for(challenge_id, &response_digest);
        assert_eq!(binding_id, "eskpsb_8cb7197ebb5c96e3ad9a3499890dfebe");

        let challenge = crate::esk_asset::platform::sui_address_binding::assemble_challenge(
            &format!("sha256:{}", "2".repeat(64)),
            &ChallengeMaterial {
                address: format!("0x{}", "3".repeat(64)),
                ttl_seconds: 600,
                nonce_base64: "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=".into(),
                issued_at: "2026-09-05T08:00:00.000Z".into(),
                expires_at: "2026-09-05T08:10:00.000Z".into(),
            },
        )
        .expect("fixed challenge material");
        let verified = VerifiedWalletResponse {
            challenge_id: challenge_id.into(),
            address: challenge.address.clone(),
            subject_commitment: challenge.subject_commitment.clone(),
            message_sha256: format!("sha256:{}", "4".repeat(64)),
            signature_scheme: SignatureScheme::Ed25519,
            signature_sha256: format!("sha256:{}", "5".repeat(64)),
            response_digest,
            verified_at: "2026-09-05T08:05:00.000Z".into(),
            wallet_response_json: "{}".into(),
        };
        assert_eq!(
            binding_receipt_for(
                &binding_id,
                &AddressBindingChallenge {
                    challenge_id: challenge_id.into(),
                    message_sha256: verified.message_sha256.clone(),
                    ..challenge
                },
                &verified,
                "2026-09-05T08:06:00.000Z",
            ),
            "sha256:c80881a767bc361f8342809afb410e599e19761879276d8c03ba83085dc87161"
        );
    }

    #[test]
    fn sqlite_failures_keep_conflicts_separate_from_storage_and_corruption() {
        let sqlite = |code| rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None);
        assert_eq!(
            map_insert_error(sqlite(rusqlite::ffi::SQLITE_BUSY))
                .downcast_ref::<AddressBindingError>(),
            Some(&AddressBindingError::Storage)
        );
        assert_eq!(
            map_read_error(sqlite(rusqlite::ffi::SQLITE_CORRUPT))
                .downcast_ref::<AddressBindingError>(),
            Some(&AddressBindingError::CorruptLedger)
        );
        let rate_limited = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some("ESK Sui challenge rate limited".to_owned()),
        );
        assert_eq!(
            map_insert_error(rate_limited).downcast_ref::<AddressBindingError>(),
            Some(&AddressBindingError::RateLimited)
        );
        assert_eq!(
            map_insert_error(sqlite(rusqlite::ffi::SQLITE_CONSTRAINT))
                .downcast_ref::<AddressBindingError>(),
            Some(&AddressBindingError::Conflict)
        );
    }
}
