//! Allowlisted business snapshots; no website credentials or execution commands.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const MAX_BYTES: usize = 256 * 1024;
pub(super) const MAX_TOTAL: usize = 1024 * 1024;
pub(super) const MAX_SOURCES: usize = 16;
pub(super) const RETENTION_MS: u64 = 86_400_000;
#[path = "../private_read_projection/strict_json.rs"]
mod strict_json;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    pub schema: String,
    pub platform: String,
    pub device_id: String,
    pub sequence: u64,
    pub account: Option<String>,
    pub account_kind: String,
    pub observed_at_ms: u64,
    pub fresh_until_ms: u64,
    pub status: String,
    pub rows: Vec<Row>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Row {
    pub id: String,
    pub symbol: String,
    pub status: String,
    pub direction: Option<String>,
    pub spacing: Option<String>,
    pub lower: Option<String>,
    pub upper: Option<String>,
    pub count: Option<String>,
    pub leverage: Option<String>,
    pub profit: Option<String>,
    pub created: Option<String>,
    pub detail: bool,
    pub metrics: BTreeMap<String, Value>,
}
pub(super) fn parse(bytes: &[u8], now: u64) -> Result<Snapshot, &'static str> {
    if bytes.len() > MAX_BYTES {
        return Err("source_too_large");
    }
    let value = strict_json::parse(bytes).map_err(|_| "source_invalid_json")?;
    let snapshot: Snapshot = serde_json::from_value(value).map_err(|_| "source_invalid")?;
    snapshot.validate(now)?;
    Ok(snapshot)
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn digits(value: &str, max: usize, zero: bool) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|b| b.is_ascii_digit())
        && ((zero && value == "0") || !value.starts_with('0'))
}
fn decimal(value: &str) -> bool {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let mut parts = unsigned.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    digits(whole, 30, true)
        && parts.next().is_none()
        && fraction
            .is_none_or(|v| !v.is_empty() && v.len() <= 20 && v.bytes().all(|b| b.is_ascii_digit()))
}
fn optional(value: &Option<String>, check: impl Fn(&str) -> bool) -> bool {
    value.as_deref().is_none_or(check)
}
fn price(value: &str) -> bool {
    decimal(value) && !value.starts_with('-') && value.bytes().any(|b| (b'1'..=b'9').contains(&b))
}
fn price_less(a: &str, b: &str) -> bool {
    let (aw, af) = a.split_once('.').unwrap_or((a, ""));
    let (bw, bf) = b.split_once('.').unwrap_or((b, ""));
    aw.len()
        .cmp(&bw.len())
        .then_with(|| aw.cmp(bw))
        .then_with(|| format!("{af:0<20}").cmp(&format!("{bf:0<20}")))
        .is_lt()
}
impl Snapshot {
    pub(super) fn validate(&self, now: u64) -> Result<(), &'static str> {
        if self.schema != "yilong.grid_device_snapshot.v1"
            || !matches!(self.platform.as_str(), "android" | "windows")
            || self.device_id.len() != 36
            || !self.device_id.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
                }
            })
            || self.sequence == 0
            || self.sequence > 9_007_199_254_740_991
            || !matches!(self.account_kind.as_str(), "primary" | "sub" | "unknown")
            || !matches!(self.status.as_str(), "fresh" | "unavailable")
            || self.observed_at_ms == 0
            || self.observed_at_ms > now.saturating_add(30_000)
            || self.fresh_until_ms < self.observed_at_ms
            || self.fresh_until_ms - self.observed_at_ms > 300_000
            || self.rows.len() > 500
            || !optional(&self.account, hash)
        {
            return Err("source_invalid");
        }
        if self.status == "fresh" && (self.account.is_none() || self.fresh_until_ms <= now) {
            return Err("source_expired");
        }
        if self.status == "unavailable" && (!self.rows.is_empty() || self.account.is_some()) {
            return Err("source_unavailable_not_empty");
        }
        let mut ids = BTreeSet::new();
        for row in &self.rows {
            row.validate()?;
            if row
                .created
                .as_ref()
                .is_some_and(|v| v.parse::<u64>().unwrap_or(u64::MAX) > self.observed_at_ms)
            {
                return Err("source_created_in_future");
            }
            if !ids.insert(&row.id) {
                return Err("source_duplicate_grid");
            }
        }
        Ok(())
    }
}
impl Row {
    fn validate(&self) -> Result<(), &'static str> {
        let base = self.symbol.strip_suffix("USDT").unwrap_or("");
        let symbol_ok = !base.is_empty()
            && base.chars().count() <= 24
            && base.chars().all(|c| {
                c.is_ascii_uppercase()
                    || c.is_ascii_digit()
                    || ('\u{3400}'..='\u{4dbf}').contains(&c)
                    || ('\u{4e00}'..='\u{9fff}').contains(&c)
            });
        if !digits(&self.id, 20, false)
            || !symbol_ok
            || self.status.is_empty()
            || self.status.len() > 64
            || !self.status.as_bytes()[0].is_ascii_uppercase()
            || !self
                .status
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            || !optional(&self.direction, |v| {
                matches!(v, "LONG" | "SHORT" | "NEUTRAL")
            })
            || !optional(&self.spacing, |v| matches!(v, "ARITH" | "GEO"))
            || !optional(&self.count, |v| {
                digits(v, 6, false) && v.parse::<u32>().unwrap_or(u32::MAX) <= 100_000
            })
            || !optional(&self.leverage, |v| digits(v, 4, false))
            || !optional(&self.created, |v| digits(v, 16, false))
            || !optional(&self.lower, price)
            || !optional(&self.upper, price)
            || !optional(&self.profit, decimal)
            || self
                .lower
                .as_ref()
                .zip(self.upper.as_ref())
                .is_some_and(|(a, b)| !price_less(a, b))
        {
            return Err("source_row_invalid");
        }
        for (key, value) in &self.metrics {
            let allowed = match key.as_str() {
                "initialNotional" | "investment" | "matchedPnl" | "fundingFee" | "fee"
                | "adjustmentAmount" | "perGridQty" | "perGridQuoteQty" | "triggerPrice"
                | "stopUpper" | "stopLower" | "stopTpPnl" | "stopSlPnl" | "trailingUpPrice"
                | "trailingDownPrice" => value.is_null() || value.as_str().is_some_and(decimal),
                "closeOnStop" | "autoAddMargin" | "trailingUp" | "trailingDown" => {
                    value.is_null() || value.is_boolean()
                }
                "matchedCount" | "ended" => {
                    value.is_null() || value.as_str().is_some_and(|v| digits(v, 16, true))
                }
                "marginType" => {
                    value.is_null()
                        || value
                            .as_str()
                            .is_some_and(|v| matches!(v, "CROSSED" | "ISOLATED"))
                }
                "orderCurrency" => {
                    value.is_null()
                        || value
                            .as_str()
                            .is_some_and(|v| matches!(v, "BASE" | "QUOTE"))
                }
                _ => false,
            };
            if !allowed {
                return Err("source_metric_invalid");
            }
        }
        Ok(())
    }
}
