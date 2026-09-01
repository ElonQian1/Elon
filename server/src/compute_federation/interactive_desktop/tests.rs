use std::collections::BTreeSet;

use super::{
    authority::{
        INTERACTIVE_DESKTOP_HOST_CONSENT_DIGEST_DOMAIN, INTERACTIVE_DESKTOP_HOST_CONSENT_SCHEMA,
        INTERACTIVE_DESKTOP_RELAY_AUTHORITY_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_RELAY_AUTHORITY_SCHEMA,
    },
    metering::{
        INTERACTIVE_DESKTOP_USAGE_RECEIPT_DIGEST_DOMAIN, INTERACTIVE_DESKTOP_USAGE_RECEIPT_SCHEMA,
        INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_SCHEMA,
    },
    offer::{
        INTERACTIVE_DESKTOP_OFFER_PROFILE_DIGEST_DOMAIN, INTERACTIVE_DESKTOP_OFFER_PROFILE_SCHEMA,
    },
    session::{
        InteractiveDesktopSessionState, INTERACTIVE_DESKTOP_CONTROL_EPOCH_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_CONTROL_EPOCH_SCHEMA, INTERACTIVE_DESKTOP_HOST_LEASE_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_HOST_LEASE_SCHEMA, INTERACTIVE_DESKTOP_MEDIA_EPOCH_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_MEDIA_EPOCH_SCHEMA, INTERACTIVE_DESKTOP_SESSION_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_SESSION_REQUEST_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_SESSION_REQUEST_SCHEMA, INTERACTIVE_DESKTOP_SESSION_SCHEMA,
        INTERACTIVE_DESKTOP_VIEWER_GRANT_DIGEST_DOMAIN, INTERACTIVE_DESKTOP_VIEWER_GRANT_SCHEMA,
    },
};

#[test]
fn interactive_contract_has_independent_schemas_and_digest_domains() {
    let schemas = [
        INTERACTIVE_DESKTOP_OFFER_PROFILE_SCHEMA,
        INTERACTIVE_DESKTOP_SESSION_REQUEST_SCHEMA,
        INTERACTIVE_DESKTOP_SESSION_SCHEMA,
        INTERACTIVE_DESKTOP_HOST_LEASE_SCHEMA,
        INTERACTIVE_DESKTOP_VIEWER_GRANT_SCHEMA,
        INTERACTIVE_DESKTOP_MEDIA_EPOCH_SCHEMA,
        INTERACTIVE_DESKTOP_CONTROL_EPOCH_SCHEMA,
        INTERACTIVE_DESKTOP_USAGE_RECEIPT_SCHEMA,
        INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_SCHEMA,
        INTERACTIVE_DESKTOP_HOST_CONSENT_SCHEMA,
        INTERACTIVE_DESKTOP_RELAY_AUTHORITY_SCHEMA,
    ];
    let domains = [
        INTERACTIVE_DESKTOP_OFFER_PROFILE_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_SESSION_REQUEST_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_SESSION_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_HOST_LEASE_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_VIEWER_GRANT_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_MEDIA_EPOCH_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_CONTROL_EPOCH_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_USAGE_RECEIPT_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_HOST_CONSENT_DIGEST_DOMAIN,
        INTERACTIVE_DESKTOP_RELAY_AUTHORITY_DIGEST_DOMAIN,
    ];

    assert_eq!(
        schemas.into_iter().collect::<BTreeSet<_>>().len(),
        schemas.len()
    );
    assert_eq!(
        domains.into_iter().collect::<BTreeSet<_>>().len(),
        domains.len()
    );
    assert!(schemas
        .iter()
        .all(|schema| schema.starts_with("compute_federation.interactive_desktop.")));
}

#[test]
fn batch_workload_admission_remains_a_closed_non_interactive_set() {
    const WORKLOAD: &str = include_str!("../workload.rs");
    const VALIDATION: &str =
        include_str!("../../store/compute_job_contract_validation/workload.rs");
    const MCP_SCHEMA: &str = include_str!("../../compute_federation_broker_mcp/schemas.rs");
    let batch_kinds = [
        "llm_chat",
        "embedding",
        "rerank",
        "image_generation",
        "video_generation",
        "evaluation_shard",
        "gpu_batch",
    ];

    for task_kind in batch_kinds {
        assert!(WORKLOAD.contains(task_kind));
        assert!(MCP_SCHEMA.contains(task_kind));
    }
    assert!(VALIDATION.contains("TASK_KIND_LLM_CHAT"));
    assert!(VALIDATION.contains("TASK_KIND_GPU_BATCH"));
    assert!(!WORKLOAD.contains("TASK_KIND_INTERACTIVE_DESKTOP"));
    assert!(!VALIDATION.contains("INTERACTIVE_DESKTOP"));
    assert!(!MCP_SCHEMA.contains("interactive_desktop"));
}

#[test]
fn session_state_machine_rejects_skips_and_terminal_revival() {
    use InteractiveDesktopSessionState as State;

    assert!(State::Requested.allows_transition(State::Reserved));
    assert!(State::Reserved.allows_transition(State::HostLeased));
    assert!(State::HostLeased.allows_transition(State::ViewerGranted));
    assert!(State::ViewerGranted.allows_transition(State::Connecting));
    assert!(State::Connecting.allows_transition(State::Active));
    assert!(State::Active.allows_transition(State::Reconnecting));
    assert!(State::Reconnecting.allows_transition(State::Connecting));
    assert!(State::Active.allows_transition(State::Ending));
    assert!(State::Ending.allows_transition(State::Ended));
    assert!(!State::Requested.allows_transition(State::Active));
    assert!(!State::Reconnecting.allows_transition(State::Active));
    assert!(!State::Ended.allows_transition(State::Active));
    assert!(State::Ended.is_terminal());
    assert!(State::Canceled.is_terminal());
    assert!(State::Failed.is_terminal());
}

#[test]
fn persistent_contracts_do_not_declare_sensitive_transport_or_content_fields() {
    let sources = [
        include_str!("offer.rs"),
        include_str!("session.rs"),
        include_str!("authority.rs"),
        include_str!("metering.rs"),
    ];
    let forbidden_fields = [
        "sdp",
        "ice_candidate",
        "bearer_token",
        "turn_credential",
        "media_payload",
        "video_frame",
        "audio_payload",
        "input_payload",
        "key_text",
        "clipboard_payload",
        "file_payload",
        "cookie",
        "opaque_surface_ref",
        "viewer_device_id",
        "relay_allocation_id",
    ];

    for source in sources {
        for line in source
            .lines()
            .filter(|line| line.trim_start().starts_with("pub "))
        {
            assert!(!forbidden_fields.iter().any(|field| line.contains(field)));
        }
        for (offset, _) in source.match_indices("pub(crate) struct ") {
            let prefix = &source[..offset];
            let attribute = prefix.rsplit("#[serde(").next().unwrap_or_default();
            assert!(attribute.contains("deny_unknown_fields"));
        }
    }
}
