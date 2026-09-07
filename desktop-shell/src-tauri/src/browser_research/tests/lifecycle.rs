use super::*;

#[test]
fn source_summary_reports_native_session_owner_for_sink_binding_check() {
    let fixture = Fixture::new();
    let result = query::execute(
        &fixture.root,
        &fixture.session,
        &ResearchCommand {
            kind: "status".into(),
            site_id: None,
            session_id: Some(fixture.session.id.clone()),
            resource_id: None,
            request_id: None,
            query: None,
            offset: None,
            limit: None,
            manifest: None,
        },
    )
    .unwrap();
    assert_eq!(result["session"]["owner_hash"], hash(b"owner"));
    assert_ne!(result["session"]["owner_hash"], hash(b"different-owner"));
    assert!(!result["session"]
        .as_object()
        .unwrap()
        .contains_key("owner_user_id"));
}

#[test]
fn only_acknowledged_host_becomes_observing_and_failure_cannot_revive() {
    let mut fixture = Fixture::new();
    fixture.session.phase = "opening".into();
    let mut phase = fixture.event("phase", "", None);
    phase.resource_type = Some("native_attached".into());
    ingest::accept(&mut fixture.session, &fixture.root, phase).unwrap();
    assert_eq!(fixture.session.phase, "opening");
    assert_eq!(
        fixture.session.host_stage.as_deref(),
        Some("native_attached")
    );
    let mut invalid = fixture.event("phase", "", None);
    invalid.resource_type = Some("untrusted_stage".into());
    ingest::accept(&mut fixture.session, &fixture.root, invalid).unwrap();
    assert_eq!(
        fixture.session.host_stage.as_deref(),
        Some("native_attached")
    );

    let mut failure = fixture.event("failed", "", None);
    failure.error_code = Some("page_enable_timed_out".into());
    ingest::accept(&mut fixture.session, &fixture.root, failure).unwrap();
    assert!(!fixture.session.active);
    assert_eq!(fixture.session.phase, "host_unavailable");
    assert!(fixture
        .session
        .gaps
        .contains(&"page_enable_timed_out".into()));
    for kind in ["ready", "navigation", "resource"] {
        let late = fixture.event(kind, "https://fixture.example/late.js", Some("late"));
        ingest::accept(&mut fixture.session, &fixture.root, late).unwrap();
    }
    assert!(!fixture.session.active);
    assert_eq!(fixture.session.phase, "host_unavailable");
    assert!(fixture.session.resources.is_empty());

    // Runtime explicitly authorizes a fresh resume, whose ACK is a new generation.
    fixture.session.active = true;
    fixture.session.phase = "resuming".into();
    fixture.session.generation = 2;
    let old = fixture.event("ready", "", None);
    ingest::accept(&mut fixture.session, &fixture.root, old).unwrap();
    assert_eq!(fixture.session.phase, "resuming");
    let mut ready = fixture.event("ready", "", None);
    ready.generation = 2;
    ingest::accept(&mut fixture.session, &fixture.root, ready).unwrap();
    assert_eq!(fixture.session.phase, "observing");
}
