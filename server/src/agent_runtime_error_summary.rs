// server/src/agent_runtime_error_summary.rs

use sha2::{Digest, Sha256};

pub(crate) fn operational_error_summary(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let fingerprint = hex::encode(Sha256::digest(compact.as_bytes()));
    format!(
        "category={}, chars={}, fingerprint={}",
        classify_error_hint(&compact),
        compact.chars().count(),
        &fingerprint[..16]
    )
}

fn classify_error_hint(body: &str) -> &'static str {
    let lower = body.to_ascii_lowercase();
    if lower.contains("rate limit") || lower.contains("429") {
        "rate_limit"
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "timeout"
    } else if lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("api key")
        || lower.contains("401")
        || lower.contains("403")
    {
        "auth"
    } else if lower.contains("quota") || lower.contains("insufficient") {
        "quota"
    } else {
        "provider_error"
    }
}
