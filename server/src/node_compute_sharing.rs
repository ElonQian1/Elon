//! Shared-node inference admission boundary used by every HTTP/agent caller.

use anyhow::Result;

use crate::store::{NodeComputeRun, NodeComputeRunStart, NodeComputeSharingStatus, Store};

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

pub(crate) fn admit(store: &Store, input: NodeComputeRunStart<'_>) -> Result<NodeComputeRun> {
    let provider_user_id = input.provider_user_id.unwrap_or_default().trim();
    let consumer_user_id = input.consumer_user_id.trim();
    if !provider_user_id.is_empty()
        && !consumer_user_id.is_empty()
        && provider_user_id == consumer_user_id
    {
        store.start_node_compute_run(input)
    } else {
        store.claim_shared_node_compute_run(input)
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
}
