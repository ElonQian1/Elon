//! Shared-node inference admission boundary used by every HTTP/agent caller.

use anyhow::Result;

use crate::store::{NodeComputeRun, NodeComputeRunStart, NodeComputeSharingStatus, Store};

pub(crate) mod endpoint_authority;

pub(crate) const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 1024;
const MAX_MAX_OUTPUT_TOKENS: u32 = 1_000_000;

pub(crate) fn normalize_max_output_tokens(value: Option<u32>) -> u32 {
    value
        .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS)
        .clamp(1, MAX_MAX_OUTPUT_TOKENS)
}

pub(crate) fn estimate_token_budget(messages: &[serde_json::Value], max_output_tokens: u32) -> i64 {
    // UTF-8 bytes plus per-message overhead is intentionally conservative for
    // byte-fallback tokenizers; the provider's terminal usage remains the fact.
    let serialized_bytes = serde_json::to_vec(messages)
        .map(|value| value.len())
        .unwrap_or(0);
    let overhead = messages.len().saturating_mul(16).saturating_add(64);
    let input_budget = serialized_bytes.saturating_add(overhead).max(1);
    i64::try_from(input_budget)
        .unwrap_or(i64::MAX)
        .saturating_add(i64::from(max_output_tokens))
        .clamp(1, 1_000_000_000_000)
}

pub(crate) fn status(
    store: &Store,
    consumer_user_id: &str,
    provider_user_id: &str,
    node_id: &str,
    model_id: &str,
) -> Result<NodeComputeSharingStatus> {
    let consumer_user_id = consumer_user_id.trim();
    let provider_user_id = provider_user_id.trim();
    if !consumer_user_id.is_empty()
        && !provider_user_id.is_empty()
        && consumer_user_id == provider_user_id
    {
        let mut status = store.node_compute_sharing_status(node_id, provider_user_id, None)?;
        status.available = true;
        status.availability = "owner_self_use".to_string();
        return Ok(status);
    }
    store.node_compute_sharing_status(node_id, provider_user_id, Some(model_id))
}

pub(crate) fn admit(
    store: &Store,
    input: NodeComputeRunStart<'_>,
    reserved_token_budget: i64,
) -> Result<NodeComputeRun> {
    let provider_user_id = input.provider_user_id.unwrap_or_default().trim();
    let consumer_user_id = input.consumer_user_id.trim();
    if !provider_user_id.is_empty()
        && !consumer_user_id.is_empty()
        && provider_user_id == consumer_user_id
    {
        store.start_node_compute_run(input)
    } else {
        store.claim_shared_node_compute_run_with_budget(input, reserved_token_budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon-node-compute-sharing-boundary-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn owner_can_use_own_node_without_opening_shared_supply() {
        let store = temp_store();
        let owner = store
            .create_user("sharing-self@example.com", "secret1", None, None)
            .unwrap();
        store
            .create_node_credential(
                "self-node",
                "secret-hash",
                &owner.id,
                Some("self node"),
                None,
                Some("self-install"),
            )
            .unwrap();

        let status = status(&store, &owner.id, &owner.id, "self-node", "qwen").unwrap();
        assert!(status.available);
        assert_eq!(status.availability, "owner_self_use");
        assert!(!status.policy.enabled);

        let run = admit(
            &store,
            NodeComputeRunStart {
                compute_call_id: "node_llm:self-use",
                consumer_user_id: &owner.id,
                provider_user_id: Some(&owner.id),
                node_id: "self-node",
                model_id: Some("qwen"),
                feature: "node_llm",
                usage_mode: "server_node_llm",
                route_reason: Some("self_use_test"),
            },
            128,
        )
        .unwrap();
        assert_eq!(run.consumer_user_id, owner.id);
    }

    #[test]
    fn empty_identities_never_gain_owner_self_use() {
        let store = temp_store();
        let status = status(&store, "", "", "unknown-node", "qwen").unwrap();
        assert!(!status.available);
        assert_eq!(status.availability, "sharing_disabled");
    }

    #[test]
    fn token_budget_includes_conservative_input_and_bounded_output() {
        let messages = vec![serde_json::json!({"role": "user", "content": "你好"})];
        let max_output = normalize_max_output_tokens(Some(0));
        assert_eq!(max_output, 1);
        let budget = estimate_token_budget(&messages, max_output);
        assert!(budget > serde_json::to_vec(&messages).unwrap().len() as i64);
        assert_eq!(
            normalize_max_output_tokens(Some(u32::MAX)),
            MAX_MAX_OUTPUT_TOKENS
        );
    }
}
