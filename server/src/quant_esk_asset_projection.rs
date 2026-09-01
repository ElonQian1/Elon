use serde::Serialize;
use uuid::Uuid;

use super::quant_paper_access::PaperGrantSigner;
use crate::esk_asset::{
    format_esk_amount, EskAccountLedger, EskAssetMode, ESK_ASSET_ID, ESK_DECIMALS, ESK_NAME,
    ESK_SYMBOL,
};

pub(crate) const ESK_PROJECTION_SCHEMA: &str = "yilong.esk.asset_projection.v1";
const TOKEN_PREFIX: &str = "yep1";
const ISSUER: &str = "yilong-main";
const AUDIENCE: &str = "yilong-quant";
const MAX_LIFETIME_SECONDS: i64 = 300;

#[derive(Debug, Serialize)]
struct EskAssetProjectionClaims<'a> {
    schema: &'static str,
    projection_id: String,
    issuer: &'static str,
    audience: &'static str,
    key_id: &'a str,
    grant_id: &'a str,
    participant_ref: &'a str,
    asset_id: &'static str,
    symbol: &'static str,
    name: &'static str,
    decimals: u32,
    mode: &'static str,
    issuance_mode: &'static str,
    chain_status: &'static str,
    total: String,
    available: String,
    reserved_for_sellback: String,
    total_base_units: String,
    available_base_units: String,
    reserved_base_units: String,
    source_revision: i64,
    source_updated_at: Option<String>,
    observed_at_unix: i64,
    expires_at_unix: i64,
    simulated: bool,
    funds_moved: bool,
}

pub(crate) fn issue_esk_projection(
    signer: &PaperGrantSigner,
    grant_id: &str,
    participant_ref: &str,
    mode: EskAssetMode,
    ledger: EskAccountLedger,
    observed_at_unix: i64,
    expires_at_unix: i64,
) -> Result<String, ()> {
    if matches!(mode, EskAssetMode::Invalid)
        || observed_at_unix <= 0
        || expires_at_unix <= observed_at_unix
        || expires_at_unix - observed_at_unix > MAX_LIFETIME_SECONDS
        || ledger.total_base_units < 0
        || ledger.reserved_base_units < 0
        || ledger.reserved_base_units > ledger.total_base_units
        || ledger.revision < 0
        || !valid_prefixed_hex(grant_id, "qpg_", 32)
        || !valid_participant_ref(participant_ref)
        || ledger.updated_at.as_ref().is_some_and(|value| {
            value.is_empty() || value.len() > 64 || value.chars().any(char::is_control)
        })
    {
        return Err(());
    }
    let available_base_units = ledger
        .total_base_units
        .checked_sub(ledger.reserved_base_units)
        .ok_or(())?;
    let claims = EskAssetProjectionClaims {
        schema: ESK_PROJECTION_SCHEMA,
        projection_id: format!("qep_{}", Uuid::new_v4().simple()),
        issuer: ISSUER,
        audience: AUDIENCE,
        key_id: signer.key_id(),
        grant_id,
        participant_ref,
        asset_id: ESK_ASSET_ID,
        symbol: ESK_SYMBOL,
        name: ESK_NAME,
        decimals: ESK_DECIMALS,
        mode: mode.label(),
        issuance_mode: "paper_recorded",
        chain_status: "not_deployed",
        total: format_esk_amount(ledger.total_base_units),
        available: format_esk_amount(available_base_units),
        reserved_for_sellback: format_esk_amount(ledger.reserved_base_units),
        total_base_units: ledger.total_base_units.to_string(),
        available_base_units: available_base_units.to_string(),
        reserved_base_units: ledger.reserved_base_units.to_string(),
        source_revision: ledger.revision,
        source_updated_at: ledger.updated_at,
        observed_at_unix,
        expires_at_unix,
        simulated: true,
        funds_moved: false,
    };
    signer.sign_token(TOKEN_PREFIX, &claims)
}

fn valid_participant_ref(value: &str) -> bool {
    valid_prefixed_hex(value, "yp1_", 40)
}

fn valid_prefixed_hex(value: &str, prefix: &str, digits: usize) -> bool {
    value.len() == prefix.len() + digits
        && value.starts_with(prefix)
        && value[prefix.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use ring::signature::{UnparsedPublicKey, ED25519};

    const SEED: [u8; 32] = [7; 32];
    const SUBJECT_SECRET: [u8; 32] = [11; 32];

    fn signer() -> PaperGrantSigner {
        PaperGrantSigner::from_material("paper-key-test".to_owned(), &SEED, &SUBJECT_SECRET)
            .unwrap()
    }

    #[test]
    fn signs_exact_esk_balances_and_paper_boundaries() {
        let token = issue_esk_projection(
            &signer(),
            "qpg_0123456789abcdef0123456789abcdef",
            "yp1_0123456789abcdef0123456789abcdef01234567",
            EskAssetMode::Paper,
            EskAccountLedger {
                total_base_units: 12_500_000,
                reserved_base_units: 4_250_000,
                revision: 3,
                updated_at: Some("2026-09-02T06:00:00Z".to_owned()),
            },
            1_788_192_000,
            1_788_192_300,
        )
        .unwrap();
        let segments = token.split('.').collect::<Vec<_>>();
        assert_eq!(segments[0], TOKEN_PREFIX);
        let payload = URL_SAFE_NO_PAD.decode(segments[1]).unwrap();
        let signature = URL_SAFE_NO_PAD.decode(segments[2]).unwrap();
        UnparsedPublicKey::new(&ED25519, signer().public_key_bytes())
            .verify(&payload, &signature)
            .unwrap();
        let claims: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(claims["schema"], ESK_PROJECTION_SCHEMA);
        assert_eq!(claims["total"], "12.500000");
        assert_eq!(claims["available"], "8.250000");
        assert_eq!(claims["reserved_for_sellback"], "4.250000");
        assert_eq!(claims["source_revision"], 3);
        assert_eq!(claims["chain_status"], "not_deployed");
        assert_eq!(claims["funds_moved"], false);
    }

    #[test]
    fn rejects_invalid_mode_balance_or_identity() {
        let ledger = EskAccountLedger {
            total_base_units: 1,
            reserved_base_units: 2,
            revision: 1,
            updated_at: None,
        };
        assert!(issue_esk_projection(
            &signer(),
            "qpg_0123456789abcdef0123456789abcdef",
            "yp1_0123456789abcdef0123456789abcdef01234567",
            EskAssetMode::Paper,
            ledger,
            100,
            400,
        )
        .is_err());
        assert!(issue_esk_projection(
            &signer(),
            "bad",
            "bad",
            EskAssetMode::Invalid,
            EskAccountLedger {
                total_base_units: 0,
                reserved_base_units: 0,
                revision: 0,
                updated_at: None,
            },
            100,
            400,
        )
        .is_err());
    }

    #[test]
    fn schema_matches_the_signed_projection_version() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../contracts/quant/esk-paper-asset-projection-v1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            schema["properties"]["schema"]["const"],
            ESK_PROJECTION_SCHEMA
        );
        assert_eq!(schema["properties"]["asset_id"]["const"], ESK_ASSET_ID);
    }
}
