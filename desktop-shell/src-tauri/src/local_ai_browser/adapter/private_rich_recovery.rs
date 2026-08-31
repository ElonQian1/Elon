use serde_json::{json, Value};

use super::{bounded_u64, clean_identifier, clean_identifiers};

pub(in crate::local_ai_browser) fn sanitize(value: Option<&Value>) -> Value {
    let recovery = value.and_then(Value::as_object);
    let field = |key: &str| recovery.and_then(|item| item.get(key));
    let boolean = |key: &str| field(key).and_then(Value::as_bool).unwrap_or(false);
    let outcome = clean_identifier(field("lastOutcome"), 32);
    let outcome = matches!(
        outcome.as_str(),
        "none"
            | "reset"
            | "accepted"
            | "accepted_detached"
            | "invalid"
            | "empty"
            | "stale_generation"
            | "route_mismatch"
            | "detached_incomplete"
            | "identity_mismatch"
            | "expired"
    )
    .then_some(outcome)
    .unwrap_or_else(|| "none".to_string());
    let rich_kinds = clean_identifiers(field("richKinds"), 8)
        .into_iter()
        .filter(|kind| {
            matches!(
                kind.as_str(),
                "finance" | "chart" | "renderer_upgrade_required"
            )
        })
        .collect::<Vec<_>>();
    json!({
        "version": bounded_u64(field("version"), 0, 8),
        "generation": bounded_u64(field("generation"), 0, 1_000_000_000),
        "active": boolean("active"),
        "detached": boolean("detached"),
        "conversationBound": boolean("conversationBound"),
        "turnBound": boolean("turnBound"),
        "messageBound": boolean("messageBound"),
        "richKinds": rich_kinds,
        "acceptedCount": bounded_u64(field("acceptedCount"), 0, 1_000_000),
        "rejectedCount": bounded_u64(field("rejectedCount"), 0, 1_000_000),
        "lastOutcome": outcome,
        "placeholderReconciled": boolean("placeholderReconciled"),
        "sampledAtMs": bounded_u64(field("sampledAtMs"), 0, 9_007_199_254_740_991),
    })
}
