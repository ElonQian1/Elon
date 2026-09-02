use serde::Serialize;
use uuid::Uuid;

use super::quant_paper_signer::PaperGrantSigner;
use crate::esk_asset::{
    format_esk_amount, EskAccountLedger, EskAssetMode, ESK_ASSET_ID, ESK_DECIMALS, ESK_NAME,
    ESK_SYMBOL,
};

pub(crate) const ESK_PROJECTION_SCHEMA_V1: &str = "yilong.esk.asset_projection.v1";
pub(crate) const ESK_PROJECTION_SCHEMA_V2: &str = "yilong.esk.asset_projection.v2";
const TOKEN_PREFIX_V1: &str = "yep1";
const TOKEN_PREFIX_V2: &str = "yep2";
const ISSUER: &str = "yilong-main";
const AUDIENCE: &str = "yilong-quant";
const MAX_LIFETIME_SECONDS: i64 = 300;

#[derive(Debug, Serialize)]
struct EskAssetProjectionClaimsV1<'a> {
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

#[derive(Debug, Serialize)]
struct EskAssetProjectionClaimsV2<'a> {
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
    reserved_for_quant: String,
    reserved_total: String,
    total_base_units: String,
    available_base_units: String,
    sellback_reserved_base_units: String,
    quant_reserved_base_units: String,
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
    if invalid_projection(
        mode,
        &ledger,
        grant_id,
        participant_ref,
        observed_at_unix,
        expires_at_unix,
    ) || ledger.quant_reserved_base_units != 0
    {
        return Err(());
    }
    let available_base_units = ledger
        .total_base_units
        .checked_sub(ledger.sellback_reserved_base_units)
        .ok_or(())?;
    let claims = EskAssetProjectionClaimsV1 {
        schema: ESK_PROJECTION_SCHEMA_V1,
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
        reserved_for_sellback: format_esk_amount(ledger.sellback_reserved_base_units),
        total_base_units: ledger.total_base_units.to_string(),
        available_base_units: available_base_units.to_string(),
        reserved_base_units: ledger.sellback_reserved_base_units.to_string(),
        source_revision: ledger.revision,
        source_updated_at: ledger.updated_at,
        observed_at_unix,
        expires_at_unix,
        simulated: true,
        funds_moved: false,
    };
    signer.sign_token(TOKEN_PREFIX_V1, &claims)
}

pub(crate) fn issue_esk_projection_v2(
    signer: &PaperGrantSigner,
    grant_id: &str,
    participant_ref: &str,
    mode: EskAssetMode,
    ledger: EskAccountLedger,
    observed_at_unix: i64,
    expires_at_unix: i64,
) -> Result<String, ()> {
    issue_esk_projection_v2_with_id(
        signer,
        format!("qep_{}", Uuid::new_v4().simple()),
        grant_id,
        participant_ref,
        mode,
        ledger,
        observed_at_unix,
        expires_at_unix,
    )
}

fn issue_esk_projection_v2_with_id(
    signer: &PaperGrantSigner,
    projection_id: String,
    grant_id: &str,
    participant_ref: &str,
    mode: EskAssetMode,
    ledger: EskAccountLedger,
    observed_at_unix: i64,
    expires_at_unix: i64,
) -> Result<String, ()> {
    if invalid_projection(
        mode,
        &ledger,
        grant_id,
        participant_ref,
        observed_at_unix,
        expires_at_unix,
    ) || !valid_prefixed_hex(&projection_id, "qep_", 32)
    {
        return Err(());
    }
    let available_base_units = ledger
        .total_base_units
        .checked_sub(ledger.reserved_base_units)
        .ok_or(())?;
    let claims = EskAssetProjectionClaimsV2 {
        schema: ESK_PROJECTION_SCHEMA_V2,
        projection_id,
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
        reserved_for_sellback: format_esk_amount(ledger.sellback_reserved_base_units),
        reserved_for_quant: format_esk_amount(ledger.quant_reserved_base_units),
        reserved_total: format_esk_amount(ledger.reserved_base_units),
        total_base_units: ledger.total_base_units.to_string(),
        available_base_units: available_base_units.to_string(),
        sellback_reserved_base_units: ledger.sellback_reserved_base_units.to_string(),
        quant_reserved_base_units: ledger.quant_reserved_base_units.to_string(),
        reserved_base_units: ledger.reserved_base_units.to_string(),
        source_revision: ledger.revision,
        source_updated_at: ledger.updated_at,
        observed_at_unix,
        expires_at_unix,
        simulated: true,
        funds_moved: false,
    };
    signer.sign_token(TOKEN_PREFIX_V2, &claims)
}

fn invalid_projection(
    mode: EskAssetMode,
    ledger: &EskAccountLedger,
    grant_id: &str,
    participant_ref: &str,
    observed_at_unix: i64,
    expires_at_unix: i64,
) -> bool {
    matches!(mode, EskAssetMode::Invalid)
        || observed_at_unix <= 0
        || expires_at_unix <= observed_at_unix
        || expires_at_unix - observed_at_unix > MAX_LIFETIME_SECONDS
        || ledger.total_base_units < 0
        || ledger.sellback_reserved_base_units < 0
        || ledger.quant_reserved_base_units < 0
        || ledger.reserved_base_units < 0
        || ledger
            .sellback_reserved_base_units
            .checked_add(ledger.quant_reserved_base_units)
            != Some(ledger.reserved_base_units)
        || ledger.reserved_base_units > ledger.total_base_units
        || ledger.revision < 0
        || !valid_prefixed_hex(grant_id, "qpg_", 32)
        || !valid_participant_ref(participant_ref)
        || ledger.updated_at.as_ref().is_some_and(|value| {
            value.is_empty() || value.len() > 64 || value.chars().any(char::is_control)
        })
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

    fn ledger(sellback: i64, quant: i64) -> EskAccountLedger {
        EskAccountLedger {
            total_base_units: 20_000_000,
            sellback_reserved_base_units: sellback,
            quant_reserved_base_units: quant,
            reserved_base_units: sellback + quant,
            revision: 7,
            updated_at: Some("2026-09-02T08:00:00Z".to_owned()),
        }
    }

    #[test]
    fn v1_remains_verifiable_when_no_quant_reservation_exists() {
        let token = issue_esk_projection(
            &signer(),
            "qpg_0123456789abcdef0123456789abcdef",
            "yp1_0123456789abcdef0123456789abcdef01234567",
            EskAssetMode::Paper,
            ledger(4_250_000, 0),
            1_788_192_000,
            1_788_192_300,
        )
        .unwrap();
        let segments = token.split('.').collect::<Vec<_>>();
        assert_eq!(segments[0], TOKEN_PREFIX_V1);
        let payload = URL_SAFE_NO_PAD.decode(segments[1]).unwrap();
        let signature = URL_SAFE_NO_PAD.decode(segments[2]).unwrap();
        UnparsedPublicKey::new(&ED25519, signer().public_key_bytes())
            .verify(&payload, &signature)
            .unwrap();
        let claims: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(claims["schema"], ESK_PROJECTION_SCHEMA_V1);
        assert_eq!(claims["available"], "15.750000");
        assert_eq!(claims["reserved_for_sellback"], "4.250000");
        assert_eq!(claims["funds_moved"], false);
    }

    #[test]
    fn v2_signs_split_reservations_and_v1_fails_closed_for_quant_reservation() {
        assert!(issue_esk_projection(
            &signer(),
            "qpg_0123456789abcdef0123456789abcdef",
            "yp1_0123456789abcdef0123456789abcdef01234567",
            EskAssetMode::Paper,
            ledger(3_000_000, 5_000_000),
            1_788_199_200,
            1_788_199_500,
        )
        .is_err());
        let token = issue_esk_projection_v2(
            &signer(),
            "qpg_0123456789abcdef0123456789abcdef",
            "yp1_0123456789abcdef0123456789abcdef01234567",
            EskAssetMode::Paper,
            ledger(3_000_000, 5_000_000),
            1_788_199_200,
            1_788_199_500,
        )
        .unwrap();
        let segments = token.split('.').collect::<Vec<_>>();
        assert_eq!(segments[0], TOKEN_PREFIX_V2);
        let claims: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(segments[1]).unwrap()).unwrap();
        assert_eq!(claims["schema"], ESK_PROJECTION_SCHEMA_V2);
        assert_eq!(claims["available"], "12.000000");
        assert_eq!(claims["reserved_for_sellback"], "3.000000");
        assert_eq!(claims["reserved_for_quant"], "5.000000");
        assert_eq!(claims["reserved_total"], "8.000000");
    }

    #[test]
    fn cross_repository_asset_view_fixture_uses_the_main_projection_serializer() {
        // Public deterministic test material only; never deploy this seed.
        const INTEROP_TEST_SEED: [u8; 32] = [61; 32];
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../contracts/quant/esk-paper-cross-repo-asset-view-v1.fixture.json"
        ))
        .unwrap();
        let expected = &fixture["expected"];
        let view = &expected["view"];
        let balance = &view["balance"];
        let signer = PaperGrantSigner::from_material(
            fixture["main"]["key_id"].as_str().unwrap().to_owned(),
            &INTEROP_TEST_SEED,
            &[63; 32],
        )
        .unwrap();
        let ledger = EskAccountLedger {
            total_base_units: balance["total_base_units"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            sellback_reserved_base_units: balance["sellback_reserved_base_units"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            quant_reserved_base_units: balance["quant_reserved_base_units"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            reserved_base_units: balance["reserved_base_units"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            revision: view["source_revision"].as_i64().unwrap(),
            updated_at: Some(view["source_updated_at"].as_str().unwrap().to_owned()),
        };
        let token = issue_esk_projection_v2_with_id(
            &signer,
            expected["projection_id"].as_str().unwrap().to_owned(),
            expected["grant_id"].as_str().unwrap(),
            expected["participant_ref"].as_str().unwrap(),
            EskAssetMode::Paper,
            ledger,
            expected["issued_at_unix"].as_i64().unwrap(),
            expected["expires_at_unix"].as_i64().unwrap(),
        )
        .unwrap();

        assert_eq!(
            URL_SAFE_NO_PAD.encode(signer.public_key_bytes()),
            fixture["main"]["public_key_base64url"].as_str().unwrap()
        );
        assert!(
            token == fixture["main"]["asset_projection_token"].as_str().unwrap(),
            "signed projection bytes differ from the public fixture"
        );
        assert_eq!(view["simulated"], true);
        assert_eq!(view["funds_moved"], false);
        assert_eq!(view["position_created"], false);
        assert_eq!(fixture["safety"]["chain_token_issued"], false);
        assert_eq!(fixture["safety"]["trading_started"], false);
        assert_eq!(fixture["safety"]["yield_started"], false);
    }

    #[test]
    fn rejects_invalid_balance_or_identity() {
        let mut invalid = ledger(2, 0);
        invalid.total_base_units = 1;
        assert!(issue_esk_projection_v2(
            &signer(),
            "qpg_0123456789abcdef0123456789abcdef",
            "yp1_0123456789abcdef0123456789abcdef01234567",
            EskAssetMode::Paper,
            invalid,
            100,
            400,
        )
        .is_err());
        assert!(issue_esk_projection_v2(
            &signer(),
            "bad",
            "bad",
            EskAssetMode::Invalid,
            ledger(0, 0),
            100,
            400,
        )
        .is_err());
    }

    #[test]
    fn schemas_match_both_signed_projection_versions() {
        let v1: serde_json::Value = serde_json::from_str(include_str!(
            "../../contracts/quant/esk-paper-asset-projection-v1.schema.json"
        ))
        .unwrap();
        let v2: serde_json::Value = serde_json::from_str(include_str!(
            "../../contracts/quant/esk-paper-asset-projection-v2.schema.json"
        ))
        .unwrap();
        assert_eq!(
            v1["properties"]["schema"]["const"],
            ESK_PROJECTION_SCHEMA_V1
        );
        assert_eq!(
            v2["properties"]["schema"]["const"],
            ESK_PROJECTION_SCHEMA_V2
        );
        assert_eq!(v2["properties"]["asset_id"]["const"], ESK_ASSET_ID);
    }
}
