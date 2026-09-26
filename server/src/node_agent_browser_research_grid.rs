//! Fixed Binance read receipts. Neither command nor result can carry arbitrary page requests.
use super::{ResearchCommand, ResearchResult};
use serde_json::Value;

const LIST_FIELDS: &[&str] = &[
    "id",
    "symbol",
    "status",
    "direction",
    "leverage",
    "lower",
    "upper",
    "count",
    "spacing",
];
const DETAIL_FIELDS: &[&str] = &[
    "id",
    "symbol",
    "status",
    "direction",
    "leverage",
    "lower",
    "upper",
    "count",
    "spacing",
    "created",
    "profit",
    "investment",
    "initialNotional",
    "perGridQty",
    "perGridQuoteQty",
    "matchedPnl",
    "fundingFee",
    "fee",
    "matchedCount",
    "marginType",
    "orderCurrency",
    "stopUpper",
    "stopLower",
];
const ERRORS: &[&str] = &[
    "invalid_request",
    "request_conflict",
    "request_expired",
    "busy",
    "session_unavailable",
    "identity_unavailable",
    "context_changed",
    "read_failed",
    "strategy_not_found",
];

fn handles(command: &ResearchCommand) -> bool {
    matches!(
        command.kind.as_str(),
        "binance_grid_list" | "binance_grid_detail"
    )
}
fn positive(value: &str) -> bool {
    (1..=20).contains(&value.len())
        && !value.starts_with('0')
        && value.bytes().all(|b| b.is_ascii_digit())
}
pub(crate) fn validate_command(command: &ResearchCommand) -> ResearchResult<()> {
    if !handles(command) {
        return Ok(());
    }
    if !command
        .request_id
        .as_deref()
        .is_some_and(|id| (8..=80).contains(&id.len()) && super::identifier(id))
        || command.query.as_deref().is_some_and(|s| s != "start")
        || command.kind == "binance_grid_detail"
            && !command.resource_id.as_deref().is_some_and(positive)
        || command.offset.is_some_and(|n| n > 500)
        || command.limit.is_some_and(|n| n == 0 || n > 50)
    {
        return Err("invalid_command");
    }
    Ok(())
}
fn exact(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|key| o.contains_key(*key)))
}
fn decimal(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or("");
    (1..=30).contains(&whole.len())
        && whole.bytes().all(|b| b.is_ascii_digit())
        && (whole == "0" || !whole.starts_with('0'))
        && parts.next().is_none_or(|fraction| {
            (1..=20).contains(&fraction.len()) && fraction.bytes().all(|b| b.is_ascii_digit())
        })
        && parts.next().is_none()
}
fn valid_row(value: &Value, detail: bool) -> bool {
    let fields = if detail { DETAIL_FIELDS } else { LIST_FIELDS };
    if !exact(value, fields) {
        return false;
    }
    fields.iter().all(|key| {
        let item = &value[*key];
        if item.is_null() {
            return !matches!(*key, "id" | "symbol");
        }
        let Some(s) = item.as_str().filter(|s| s.len() <= 96) else {
            return false;
        };
        match *key {
            "id" | "count" | "leverage" | "created" => positive(s),
            "symbol" => s.strip_suffix("USDT").is_some_and(|base| {
                !base.is_empty()
                    && base.chars().count() <= 24
                    && base.chars().all(|c| {
                        c.is_ascii_uppercase()
                            || c.is_ascii_digit()
                            || ('\u{3400}'..='\u{4dbf}').contains(&c)
                            || ('\u{4e00}'..='\u{9fff}').contains(&c)
                    })
            }),
            "status" => {
                !s.is_empty()
                    && s.len() <= 64
                    && s.as_bytes()[0].is_ascii_uppercase()
                    && s.bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            }
            "direction" => matches!(s, "LONG" | "SHORT" | "NEUTRAL"),
            "spacing" => matches!(s, "ARITH" | "GEO"),
            "marginType" => matches!(s, "CROSSED" | "ISOLATED"),
            "orderCurrency" => matches!(s, "BASE" | "QUOTE"),
            _ => decimal(s),
        }
    })
}
pub(crate) fn validate_response(command: &ResearchCommand, value: &Value) -> ResearchResult<()> {
    if !handles(command) {
        return Ok(());
    }
    let r = &value["reader"];
    let valid = exact(value, &["schema", "kind", "reader"])
        && value["schema"] == "yilong.browser-research.result.v1"
        && value["kind"] == command.kind
        && r["schema"] == "yilong.binance-grid-read.v1"
        && r["request_id"].as_str() == command.request_id.as_deref();
    if !valid {
        return Err("invalid_result");
    }
    let ok = match r["status"].as_str() {
        Some("pending") => exact(r, &["schema", "request_id", "status"]),
        Some("failed") => {
            exact(r, &["schema", "request_id", "status", "error"])
                && r["error"].as_str().is_some_and(|s| ERRORS.contains(&s))
        }
        Some("ready") => ready(command, r),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err("invalid_result")
    }
}
fn ready(command: &ResearchCommand, r: &Value) -> bool {
    let (Some(observed), Some(expires)) =
        (r["observed_at_ms"].as_u64(), r["expires_at_ms"].as_u64())
    else {
        return false;
    };
    if observed == 0
        || expires < observed
        || expires - observed > 300_000
        || expires > 9_007_199_254_740_991
    {
        return false;
    }
    if command.kind == "binance_grid_detail" {
        return exact(
            r,
            &[
                "schema",
                "request_id",
                "status",
                "observed_at_ms",
                "expires_at_ms",
                "row",
            ],
        ) && valid_row(&r["row"], true)
            && r["row"]["id"].as_str() == command.resource_id.as_deref();
    }
    let (Some(items), Some(total), Some(offset)) = (
        r["items"].as_array(),
        r["total"].as_u64(),
        r["offset"].as_u64(),
    ) else {
        return false;
    };
    if !exact(
        r,
        &[
            "schema",
            "request_id",
            "status",
            "observed_at_ms",
            "expires_at_ms",
            "items",
            "total",
            "offset",
            "next_offset",
        ],
    ) || total > 500
        || offset != command.offset.unwrap_or(0)
        || offset > total
        || items.len() as u64 != u64::from(command.limit.unwrap_or(25)).min(total - offset)
        || !items.iter().all(|row| valid_row(row, false))
    {
        return false;
    }
    let ids: std::collections::HashSet<_> =
        items.iter().filter_map(|row| row["id"].as_str()).collect();
    let end = offset + items.len() as u64;
    ids.len() == items.len()
        && if end < total {
            r["next_offset"].as_u64() == Some(end)
        } else {
            r["next_offset"].is_null()
        }
}

#[cfg(test)]
#[path = "node_agent_browser_research_grid_tests.rs"]
mod tests;
