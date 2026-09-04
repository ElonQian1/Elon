//! A bounded, read-only projection of payment keys held by the formal ledger.
use anyhow::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use serde_json::Value;

use super::{payment_identity::fingerprint, PlatformError};

pub(crate) const PLATFORM_PAYMENT_SNAPSHOT_MAX_KEYS: usize = 10_000;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PlatformReconciliationSnapshot {
    pub schema: &'static str,
    pub scope: &'static str,
    pub source_fingerprint: String,
    pub policy_digest: String,
    pub observed_at: String,
    pub used_payment_keys: Vec<String>,
    pub prepared_count: String,
    pub recorded_count: String,
    pub key_count: String,
    pub platform_history_complete: bool,
    pub external_history_complete: bool,
    pub funds_moved: bool,
    pub balances_written: bool,
    pub external_payment_verified: bool,
    pub snapshot_digest: String,
}

impl PlatformReconciliationSnapshot {
    pub(crate) fn new(
        source_fingerprint: String,
        policy_digest: String,
        observed_at: String,
        used_payment_keys: Vec<String>,
        prepared_count: usize,
        recorded_count: usize,
    ) -> Result<Self> {
        if used_payment_keys.len() > PLATFORM_PAYMENT_SNAPSHOT_MAX_KEYS {
            return Err(PlatformError::LimitExceeded.into());
        }
        let valid_digest = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        let time =
            DateTime::parse_from_rfc3339(&observed_at).map_err(|_| PlatformError::CorruptLedger)?;
        if !valid_digest(&source_fingerprint)
            || !valid_digest(&policy_digest)
            || time
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Millis, true)
                != observed_at
            || prepared_count.checked_add(recorded_count) != Some(used_payment_keys.len())
            || used_payment_keys.iter().any(|key| !valid_digest(key))
            || used_payment_keys.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(PlatformError::CorruptLedger.into());
        }
        let mut result = Self {
            schema: "yilong.esk.platform_payment_snapshot.v1",
            scope: "platform_recorded_allocations_only",
            source_fingerprint,
            policy_digest,
            observed_at,
            key_count: used_payment_keys.len().to_string(),
            used_payment_keys,
            prepared_count: prepared_count.to_string(),
            recorded_count: recorded_count.to_string(),
            platform_history_complete: true,
            external_history_complete: false,
            funds_moved: false,
            balances_written: false,
            external_payment_verified: false,
            snapshot_digest: String::new(),
        };
        let mut value = serde_json::to_value(&result)?;
        value["snapshot_digest"] = Value::Null;
        result.snapshot_digest = fingerprint(&value)?;
        Ok(result)
    }
}
