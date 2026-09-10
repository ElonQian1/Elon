use super::*;
use affinity::{Affinity, HostInput, OwnedSession, HOST_LEASE_MS};
use serde_json::json;

fn command(value: Value) -> ResearchCommand {
    serde_json::from_value(value).unwrap()
}
fn host(id: &str, project: &str, session: Option<&str>) -> HostInput {
    HostInput {
        instance_id: id.into(),
        sessions: session
            .map(|s| OwnedSession {
                project_key: project.into(),
                session_id: s.into(),
            })
            .into_iter()
            .collect(),
    }
}
fn root() -> std::path::PathBuf {
    std::env::current_dir().unwrap()
}

#[test]
fn session_owner_wins_regardless_of_poll_order_and_legacy_cannot_claim() {
    for other_first in [true, false] {
        let hub = BrowserResearchHub::default();
        let project = project_key(&root()).unwrap();
        hub.register_host(host("owner_a", &project, Some("session_a")))
            .unwrap();
        hub.register_host(host("other_b", &project, None)).unwrap();
        let action = hub
            .enqueue(
                &root(),
                command(json!({"kind":"resources","session_id":"session_a"})),
            )
            .unwrap();
        assert_eq!(action.instance_id, "owner_a");
        let wrong = || {
            assert!(hub.pending(8, "other_b").unwrap().is_empty());
            assert_eq!(
                hub.claim(&action.action_id, "other_b").unwrap_err(),
                "host_mismatch"
            );
            assert_eq!(
                hub.claim(&action.action_id, "").unwrap_err(),
                "host_unavailable"
            );
        };
        if other_first {
            wrong();
        }
        let claim = hub.claim(&action.action_id, "owner_a").unwrap();
        if !other_first {
            wrong();
        }
        assert_eq!(
            hub.claim(&action.action_id, "owner_a").unwrap_err(),
            "action_not_claimable"
        );
        assert_eq!(claim.action.instance_id, "owner_a");
    }
}

#[test]
fn routing_is_explicit_for_multiple_hosts_and_project_scoped() {
    let mut a = Affinity::default();
    let p = "a".repeat(64);
    a.register(host("owner_a", &p, Some("session_a")), 100)
        .unwrap();
    a.register(host("owner_b", &p, None), 100).unwrap();
    assert_eq!(
        a.route(&p, &command(json!({"kind":"sites"})), 101),
        Err("host_ambiguous")
    );
    assert_eq!(
        a.route(
            &p,
            &command(json!({"kind":"sites","instance_id":"owner_b"})),
            101
        )
        .unwrap(),
        "owner_b"
    );
    assert_eq!(
        a.route(
            &p,
            &command(json!({"kind":"status","session_id":"session_a","instance_id":"owner_b"})),
            101
        ),
        Err("host_mismatch")
    );
    assert_eq!(
        a.route(
            &"b".repeat(64),
            &command(json!({"kind":"status","session_id":"session_a"})),
            101
        ),
        Err("host_ambiguous")
    );
    assert!(a
        .summaries(&"b".repeat(64), 101)
        .iter()
        .all(|h| h.sessions.is_empty()));
}

#[test]
fn expired_host_is_never_reassigned_to_restart_or_new_owner() {
    let mut a = Affinity::default();
    let p = "a".repeat(64);
    a.register(host("old_process", &p, Some("session_a")), 100)
        .unwrap();
    let now = 100 + HOST_LEASE_MS;
    a.register(host("new_process_or_user", &p, None), now)
        .unwrap();
    assert_eq!(
        a.route(
            &p,
            &command(json!({"kind":"status","session_id":"session_a"})),
            now
        ),
        Err("host_unavailable")
    );
    assert_eq!(
        a.register(host("new_process_or_user", &p, Some("session_a")), now),
        Err("host_mismatch")
    );
    // A heartbeat from the original still-running process restores its own lease.
    a.register(host("old_process", &p, Some("session_a")), now + 1)
        .unwrap();
    assert_eq!(
        a.route(
            &p,
            &command(json!({"kind":"status","session_id":"session_a"})),
            now + 1
        )
        .unwrap(),
        "old_process"
    );
}

#[test]
fn node_restart_reconstructs_binding_from_native_heartbeat() {
    let p = "a".repeat(64);
    let mut restarted = Affinity::default();
    restarted
        .register(host("still_running", &p, Some("session_a")), 100)
        .unwrap();
    restarted.register(host("other", &p, None), 100).unwrap();
    assert_eq!(
        restarted
            .route(
                &p,
                &command(json!({"kind":"requests","session_id":"session_a"})),
                101
            )
            .unwrap(),
        "still_running"
    );
}

#[test]
fn open_receipt_binds_before_next_heartbeat_and_delivery_retry_does_not_move_it() {
    let hub = BrowserResearchHub::default();
    let p = project_key(&root()).unwrap();
    hub.register_host(host("owner_a", &p, None)).unwrap();
    hub.register_host(host("owner_b", &p, None)).unwrap();
    let open = hub
        .enqueue(
            &root(),
            command(json!({"kind":"open","site_id":"fixture","instance_id":"owner_a"})),
        )
        .unwrap();
    let claim = hub.claim(&open.action_id, "owner_a").unwrap();
    let input = || ReceiptInput {
        claim_token: claim.claim_token.clone(),
        status: "succeeded".into(),
        result: Some(
            json!({"schema":"yilong.browser-research.result.v1","kind":"open","session":{"id":"new_session"}}),
        ),
        error_code: None,
    };
    hub.record_receipt(&open.action_id, input()).unwrap();
    hub.record_receipt(&open.action_id, input()).unwrap();
    let read = hub
        .enqueue(
            &root(),
            command(json!({"kind":"status","session_id":"new_session"})),
        )
        .unwrap();
    assert_eq!(read.instance_id, "owner_a");
}

#[test]
fn stale_lease_blocks_claim_without_reassigning_queued_action() {
    let hub = BrowserResearchHub::default();
    let p = project_key(&root()).unwrap();
    hub.register_host(host("owner_a", &p, None)).unwrap();
    let action = hub
        .enqueue(&root(), command(json!({"kind":"sites"})))
        .unwrap();
    hub.inner.lock().unwrap().affinity = Affinity::default();
    hub.register_host(host("owner_b", &p, None)).unwrap();
    assert_eq!(
        hub.claim(&action.action_id, "owner_a").unwrap_err(),
        "host_unavailable"
    );
    assert_eq!(
        hub.claim(&action.action_id, "owner_b").unwrap_err(),
        "host_mismatch"
    );
    assert_eq!(
        hub.admin_action(&action.action_id).unwrap().status,
        "queued"
    );
}

#[test]
fn host_registration_is_bounded_atomic_and_does_not_steal_sessions() {
    let p = "a".repeat(64);
    let mut a = Affinity::default();
    a.register(host("owner_a", &p, Some("s")), 100).unwrap();
    let mut input = host("other", &p, Some("s"));
    input.sessions.insert(
        0,
        OwnedSession {
            project_key: p.clone(),
            session_id: "should_not_bind".into(),
        },
    );
    assert_eq!(a.register(input, 101), Err("host_mismatch"));
    assert!(!a.live("other", 101));
    assert!(a.bind(&p, "should_not_bind", "owner_a", 101).is_ok());
    let mut oversized = host("large", &p, Some("s"));
    oversized.sessions = vec![oversized.sessions[0].clone(); 9];
    assert_eq!(a.register(oversized, 101), Err("invalid_host"));
    for n in 0..31 {
        a.register(host(&format!("host_{n}"), &p, None), 101)
            .unwrap();
    }
    assert_eq!(
        a.register(host("overflow", &p, None), 102),
        Err("host_limit")
    );
    assert!(a
        .register(host("after_expiry", &p, None), 102 + HOST_LEASE_MS)
        .is_ok());
}
