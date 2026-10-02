//! Only deliberately public grid facts cross the social boundary. Never exchange session handles.
use super::{fail, model, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GridShare {
    pub schema: String,
    pub observed_at_ms: i64,
    pub show_amounts: bool,
    pub fields: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_snapshot_id: Option<String>,
}

pub(crate) const AMOUNTS: &[&str] = &[
    "investment",
    "initialNotional",
    "perGridQty",
    "perGridQuoteQty",
    "profit",
    "matchedPnl",
    "fundingFee",
    "fee",
    "positionQty",
    "positionNotional",
    "totalPnl",
    "unrealizedPnl",
];
const NUMBERS: &[&str] = &[
    "lower",
    "upper",
    "leverage",
    "count",
    "markPrice",
    "roi",
    "entryPrice",
    "liquidationPrice",
    "stopUpper",
    "stopLower",
    "matchedCount",
];

fn decimal(value: &str) -> bool {
    let raw = value.strip_prefix('-').unwrap_or(value);
    let mut parts = raw.split('.');
    let whole = parts.next().unwrap_or("");
    if !(1..=30).contains(&whole.len())
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || (whole.len() > 1 && whole.starts_with('0'))
    {
        return false;
    }
    if let Some(fraction) = parts.next() {
        if !(1..=20).contains(&fraction.len()) || !fraction.bytes().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    parts.next().is_none()
}

impl GridShare {
    pub(super) fn validate(&self) -> Result<()> {
        let invalid = || fail(400, "Invalid public grid snapshot");
        let now = chrono::Utc::now().timestamp_millis();
        if self.schema != "yilong.grid_share.v1"
            || self.observed_at_ms < 0
            || self.observed_at_ms > now + 5000
            || self.fields.len() > 45
        {
            return Err(invalid());
        }
        let symbol = self.fields.get("symbol").ok_or_else(invalid)?;
        let base = symbol.strip_suffix("USDT").unwrap_or("");
        if !(1..=24).contains(&base.chars().count())
            || !base.chars().all(|c| {
                c.is_ascii_uppercase()
                    || c.is_ascii_digit()
                    || ('\u{3400}'..='\u{4DBF}').contains(&c)
                    || ('\u{4E00}'..='\u{9FFF}').contains(&c)
            })
        {
            return Err(invalid());
        }
        for (key, value) in &self.fields {
            if value.is_empty() || value.len() > 96 {
                return Err(invalid());
            }
            let valid = match key.as_str() {
                "symbol" => true,
                "recordKind" => value == "HISTORY",
                "settlement" => matches!(value.as_str(), "UNKNOWN" | "CONFIRMED"),
                "positionState" => matches!(value.as_str(), "UNKNOWN" | "OPEN" | "CLOSED"),
                "endReason" => matches!(
                    value.as_str(),
                    "UNKNOWN" | "MANUAL" | "TAKE_PROFIT" | "STOP_LOSS" | "LIQUIDATION"
                ),
                "feeBasis" => matches!(value.as_str(), "UNKNOWN" | "INCLUDED" | "EXCLUDED"),
                "roiBasis" => matches!(value.as_str(), "UNKNOWN" | "INITIAL" | "SOURCE"),
                "created" | "end" => {
                    value
                        .parse::<i64>()
                        .is_ok_and(|n| n > 0 && n <= self.observed_at_ms + 5000)
                        && value.bytes().all(|v| v.is_ascii_digit())
                }
                "direction" => matches!(value.as_str(), "LONG" | "SHORT" | "NEUTRAL"),
                "spacing" => matches!(value.as_str(), "ARITH" | "GEO"),
                "status" => value
                    .bytes()
                    .all(|v| v.is_ascii_uppercase() || v.is_ascii_digit() || v == b'_'),
                "marginType" => matches!(value.as_str(), "CROSSED" | "ISOLATED"),
                "orderCurrency" => matches!(value.as_str(), "BASE" | "QUOTE"),
                key if NUMBERS.contains(&key) || AMOUNTS.contains(&key) => {
                    if AMOUNTS.contains(&key) && !self.show_amounts {
                        return Err(invalid());
                    }
                    decimal(value)
                }
                _ => false,
            };
            if !valid {
                return Err(invalid());
            }
        }
        if let (Some(start), Some(end)) = (self.fields.get("created"), self.fields.get("end")) {
            if start.parse::<i64>().unwrap_or(i64::MAX) > end.parse::<i64>().unwrap_or(0) {
                return Err(invalid());
            }
        }
        if self
            .fields
            .get("recordKind")
            .is_some_and(|v| v == "HISTORY")
        {
            if self
                .fields
                .get("status")
                .is_some_and(|v| matches!(v.as_str(), "NEW" | "WORKING" | "RUNNING"))
            {
                return Err(invalid());
            }
            if self
                .fields
                .get("settlement")
                .is_some_and(|v| v == "CONFIRMED")
                && (!self.fields.contains_key("totalPnl")
                    || self
                        .fields
                        .get("positionState")
                        .is_none_or(|v| v != "CLOSED"))
            {
                return Err(invalid());
            }
        }
        for key in ["leverage", "count"] {
            if self
                .fields
                .get(key)
                .is_some_and(|v| v.parse::<u32>().map_or(true, |n| n == 0 || n > 100_000))
            {
                return Err(invalid());
            }
        }
        for key in [
            "lower",
            "upper",
            "markPrice",
            "entryPrice",
            "liquidationPrice",
        ] {
            if self
                .fields
                .get(key)
                .is_some_and(|v| v.parse::<f64>().map_or(true, |n| n <= 0.0))
            {
                return Err(invalid());
            }
        }
        if let (Some(low), Some(high)) = (self.fields.get("lower"), self.fields.get("upper")) {
            if low.parse::<f64>().unwrap_or(0.0) >= high.parse::<f64>().unwrap_or(0.0) {
                return Err(invalid());
            }
        }
        if let Some(note) = &self.note {
            model::text(note, 200)?;
        }
        if self
            .previous_snapshot_id
            .as_deref()
            .is_some_and(|v| !model::opaque(v) || !v.starts_with("ai_snapshot_"))
        {
            return Err(invalid());
        }
        Ok(())
    }

    pub(super) fn title(&self) -> String {
        format!(
            "{} {}",
            self.fields
                .get("symbol")
                .map(String::as_str)
                .unwrap_or("币安"),
            if self
                .fields
                .get("recordKind")
                .is_some_and(|v| v == "HISTORY")
            {
                "历史网格"
            } else {
                "网格快照"
            }
        )
    }
    pub(super) fn summary(&self) -> String {
        let get = |k| self.fields.get(k).map(String::as_str).unwrap_or("未读取");
        let direction = match get("direction") {
            "LONG" => "做多",
            "SHORT" => "做空",
            "NEUTRAL" => "中性",
            _ => "方向未读取",
        };
        format!(
            "{} · {}× · {}–{} · {} 格 · 历史快照",
            direction,
            get("leverage"),
            get("lower"),
            get("upper"),
            get("count")
        )
    }
}
