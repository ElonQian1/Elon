use crate::{model::*, settlement::Signed};
use anyhow::{bail, ensure, Result};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub fn canonical<T: Serialize>(domain: &str, payload: &T) -> Result<Vec<u8>> {
    let mut bytes = domain.as_bytes().to_vec();
    bytes.extend(serde_json::to_vec(payload)?);
    Ok(bytes)
}
pub const SOURCE_DOMAIN: &str = "ESK_GAME_RECONCILIATION_V1\nsource\n";
pub const CALCULATION_DOMAIN: &str = "ESK_GAME_RECONCILIATION_V1\ncalculation\n";
pub const SETTLEMENT_DOMAIN: &str = "ESK_GAME_REWARDS_V1\nsettlement\n";

pub fn fixed_hex(value: &str, bytes: usize) -> bool {
    value.len() == bytes * 2
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn nonzero_hash(value: &str) -> bool {
    fixed_hex(value, 32) && value.bytes().any(|c| c != b'0')
}
fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-:".contains(&c))
}
fn address(value: &str) -> bool {
    value.strip_prefix("0x").is_some_and(nonzero_hash)
}
pub fn integer(value: &str) -> Result<i64> {
    let n: i64 = value.parse()?;
    ensure!(n.to_string() == value, "noncanonical integer");
    Ok(n)
}
pub fn natural(value: &str) -> Result<i64> {
    let n = integer(value)?;
    ensure!(n >= 0, "negative unsigned amount");
    Ok(n)
}
pub fn key(value: &str) -> Result<VerifyingKey> {
    ensure!(fixed_hex(value, 32), "invalid public key");
    let bytes: [u8; 32] = hex::decode(value)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("key length"))?;
    let key = VerifyingKey::from_bytes(&bytes)?;
    ensure!(!key.is_weak(), "weak public key");
    Ok(key)
}
pub fn signed<T: Serialize>(public_key: &str, domain: &str, proof: &Signed<T>) -> Result<String> {
    ensure!(
        fixed_hex(&proof.signature_hex, 64),
        "invalid signature encoding"
    );
    let bytes = canonical(domain, &proof.payload)?;
    key(public_key)?.verify_strict(
        &bytes,
        &Signature::from_slice(&hex::decode(&proof.signature_hex)?)?,
    )?;
    Ok(hash(bytes))
}
pub fn config(c: &Config) -> Result<()> {
    ensure!(
        c.schema == "esk.game.reconciliation.config.v1",
        "unsupported config"
    );
    ensure!(
        nonzero_hash(&c.policy_digest) && nonzero_hash(&c.wallet_binding_digest),
        "invalid policy/binding"
    );
    ensure!(
        id(&c.source_id) && id(&c.account_scope) && id(&c.user_id),
        "invalid scope"
    );
    ensure!(address(&c.beneficiary), "invalid beneficiary");
    ensure!(natural(&c.accounting_started_at_ms)? > 0, "invalid origin");
    ensure!(c.asset_decimals <= 18, "invalid precision");
    let parts: Vec<_> = c.asset_type.split("::").collect();
    ensure!(parts.len() == 3 && address(parts[0]), "invalid currency");
    for name in &parts[1..] {
        ensure!(
            !name.is_empty()
                && name.len() <= 64
                && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'),
            "invalid currency name"
        );
    }
    key(&c.source_public_key_hex)?;
    key(&c.reconciler_public_key_hex)?;
    ensure!(
        c.source_public_key_hex != c.reconciler_public_key_hex,
        "source and reconciler must differ"
    );
    Ok(())
}
pub fn statement(c: &Config, proof: &Signed<Statement>, now_ms: i64) -> Result<String> {
    let digest = signed(&c.source_public_key_hex, SOURCE_DOMAIN, proof)?;
    let s = &proof.payload;
    ensure!(
        s.schema == "esk.game.reconciliation.source.v1",
        "unsupported source"
    );
    ensure!(
        s.source_id == c.source_id
            && s.policy_digest == c.policy_digest
            && s.account_scope == c.account_scope
            && s.user_id == c.user_id
            && s.beneficiary == c.beneficiary
            && s.wallet_binding_digest == c.wallet_binding_digest
            && s.asset_type == c.asset_type
            && s.asset_decimals == c.asset_decimals
            && s.accounting_started_at_ms == c.accounting_started_at_ms,
        "source identity mismatch"
    );
    let sequence = natural(&s.sequence)?;
    let start = natural(&s.period_start_ms)?;
    let end = natural(&s.period_end_ms)?;
    ensure!(
        sequence > 0
            && start >= natural(&c.accounting_started_at_ms)?
            && end > start
            && end <= now_ms,
        "invalid accounting period"
    );
    ensure!(
        fixed_hex(&s.previous_source_digest, 32),
        "invalid source parent"
    );
    if sequence == 1 {
        ensure!(
            start == natural(&c.accounting_started_at_ms)?
                && s.previous_source_digest == "0".repeat(64),
            "invalid first period"
        );
    } else if s.previous_source_digest == "0".repeat(64) {
        bail!("missing source parent");
    }
    for amount in [
        &s.cumulative_trading_fees_units,
        &s.cumulative_other_costs_units,
        &s.held_reserve_units,
        &s.principal_liability_units,
    ] {
        natural(amount)?;
    }
    for amount in [
        &s.cumulative_realized_trading_pnl_units,
        &s.cumulative_net_funding_units,
        &s.unrealized_pnl_units,
    ] {
        integer(amount)?;
    }
    ensure!(
        s.evidence.len() == EVIDENCE_KINDS.len(),
        "incomplete evidence"
    );
    for (evidence, expected) in s.evidence.iter().zip(EVIDENCE_KINDS) {
        ensure!(
            evidence.kind == expected && nonzero_hash(&evidence.sha256),
            "invalid evidence coverage/order"
        );
    }
    Ok(digest)
}
