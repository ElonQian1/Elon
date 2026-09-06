use super::{files, host::HostEvent, ingest, model::*, privacy, query};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture {
    root: PathBuf,
    session: Session,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "elon-research-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&root).unwrap();
        let site = SiteManifest {
            schema: "yilong.browser-research.site.v1".into(),
            id: "fixture".into(),
            name: "Synthetic research".into(),
            entry_url: "https://fixture.example/grid".into(),
            navigation_origins: vec!["https://fixture.example".into()],
            resource_origins: vec!["https://cdn.fixture.example".into()],
            api_origins: vec!["https://api.fixture.example".into()],
            identity_origins: vec!["https://identity.fixture.example".into()],
        };
        files::register(&root, site.clone()).unwrap();
        let session = Session {
            schema: "yilong.browser-research.session.v1".into(),
            id: hash(b"session"),
            project_key: hash(b"project"),
            owner_hash: hash(b"owner"),
            site_fingerprint: site.fingerprint(),
            site,
            active: true,
            generation: 1,
            expires_at_ms: now_ms() + 60000,
            phase: "observing".into(),
            bytes: 0,
            resources: vec![],
            requests: vec![],
            gaps: vec![],
        };
        Self { root, session }
    }
    fn event(&self, kind: &str, url: &str, body: Option<&str>) -> HostEvent {
        HostEvent {
            generation: 1,
            kind: kind.into(),
            url: url.into(),
            method: Some("POST".into()),
            status: Some(200),
            resource_type: Some("script".into()),
            request_id: Some("request-1".into()),
            script_id: Some("script-1".into()),
            request_body: None,
            body: body.map(Into::into),
            mime: Some("application/json".into()),
            initiator: None,
            truncated: false,
            error_code: None,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let (Ok(root), Ok(parent)) = (
            fs::canonicalize(&self.root),
            fs::canonicalize(std::env::temp_dir()),
        ) {
            if root != parent
                && root.starts_with(&parent)
                && root
                    .file_name()
                    .is_some_and(|v| v.to_string_lossy().starts_with("elon-research-test-"))
            {
                let _ = fs::remove_dir_all(root);
            }
        }
    }
}
fn command(value: serde_json::Value) -> ResearchCommand {
    serde_json::from_value(value).unwrap()
}

#[test]
fn business_values_unknown_collections_and_precise_paths_survive() {
    let mut fixture = Fixture::new();
    let mut event=fixture.event("request","https://api.fixture.example/v2/grid/list-own?page=1",Some(r#"{"code":"0","success":true,"data":{"unusualRows":[{"symbol":"SYNTHUSDT","margin":42.75,"strategyId":789}]}}"#));
    event.request_body = Some(
        r#"{"page":1,"rows":25,"direction":"LONG","authorization":"Bearer synthetic-secret"}"#
            .into(),
    );
    ingest::accept(&mut fixture.session, &fixture.root, event).unwrap();
    let request = fixture.session.requests[0].clone();
    assert_eq!(
        request.url,
        "https://api.fixture.example/v2/grid/list-own?page=1"
    );
    let result = query::execute(
        &fixture.root,
        &fixture.session,
        &command(json!({"kind":"read_request","request_id":request.id})),
    )
    .unwrap();
    let text = result.to_string();
    assert!(text.contains("unusualRows"));
    assert!(text.contains("42.75"));
    assert!(text.contains("LONG"));
    assert!(!text.contains("synthetic-secret"));
    assert!(text.contains("credential_excluded"));
}

#[test]
fn no_adapter_needed_for_cdn_script_search_and_utf8_ranges() {
    let mut fixture = Fixture::new();
    let source =
        "const 中文='grid';fetch('/private/grid/create',{body:JSON.stringify({gridCount:20})});";
    let event = fixture.event(
        "resource",
        "https://cdn.fixture.example/chunk.abcdef.js",
        Some(source),
    );
    ingest::accept(&mut fixture.session, &fixture.root, event.clone()).unwrap();
    ingest::accept(&mut fixture.session, &fixture.root, event).unwrap();
    assert_eq!(fixture.session.resources.len(), 1);
    let result = query::execute(
        &fixture.root,
        &fixture.session,
        &command(json!({"kind":"search","query":"gridCount"})),
    )
    .unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    let item = &fixture.session.resources[0];
    let result = query::execute(
        &fixture.root,
        &fixture.session,
        &command(json!({"kind":"read_resource","resource_id":item.id,"offset":7})),
    );
    assert_eq!(result.unwrap_err(), "invalid_content_offset");
    assert_eq!(item.script_id.as_deref(), Some("script-1"));
}

#[test]
fn identity_cdn_api_and_late_generation_are_separate() {
    let mut fixture = Fixture::new();
    for url in [
        "https://identity.fixture.example/session",
        "https://other.example/a",
        "https://cdn.fixture.example/private",
    ] {
        let event = fixture.event("request", url, Some("private material"));
        ingest::accept(&mut fixture.session, &fixture.root, event).unwrap();
    }
    let mut late = fixture.event("resource", "https://fixture.example/chunk.js", Some("late"));
    late.generation = 0;
    ingest::accept(&mut fixture.session, &fixture.root, late).unwrap();
    assert!(fixture.session.resources.is_empty());
    assert!(fixture.session.requests.is_empty());
}

#[test]
fn manifest_revision_revokes_capture_without_erasing_evidence() {
    let mut fixture = Fixture::new();
    let event = fixture.event(
        "resource",
        "https://fixture.example/chunk.js",
        Some("retained"),
    );
    ingest::accept(&mut fixture.session, &fixture.root, event.clone()).unwrap();
    let mut changed = fixture.session.site.clone();
    changed.resource_origins.clear();
    files::register(&fixture.root, changed).unwrap();
    ingest::accept(&mut fixture.session, &fixture.root, event).unwrap();
    assert!(!fixture.session.active);
    assert_eq!(fixture.session.phase, "scope_changed");
    assert_eq!(fixture.session.resources.len(), 1);
}

#[test]
fn body_integrity_and_owner_restore_fail_closed() {
    let mut fixture = Fixture::new();
    let event = fixture.event(
        "resource",
        "https://fixture.example/chunk.js",
        Some("source"),
    );
    ingest::accept(&mut fixture.session, &fixture.root, event).unwrap();
    assert!(
        files::load_sessions(&fixture.root, &fixture.session.project_key, &hash(b"other"))
            .is_empty()
    );
    let restored = files::load_sessions(
        &fixture.root,
        &fixture.session.project_key,
        &fixture.session.owner_hash,
    );
    assert_eq!(restored.len(), 1);
    assert!(!restored[0].active);
    let resource = &fixture.session.resources[0];
    fs::write(
        fixture
            .root
            .join(&fixture.session.id)
            .join("content")
            .join(format!("{}.txt", resource.sha256)),
        b"changed",
    )
    .unwrap();
    assert_eq!(
        files::read_body(&fixture.root, &fixture.session.id, &resource.sha256).unwrap_err(),
        "resource_integrity_changed"
    );
    assert!(files::read_body(&fixture.root, "../", &resource.sha256).is_err());
}

#[test]
fn credentials_in_bootstrap_and_urls_are_targeted() {
    let input = r#"<script>window.bootstrap={accessToken:"fake-secret",gridCount:40,market:"SYNTH"};</script>"#;
    let (clean, changed) = privacy::clean_body(input).unwrap();
    assert!(changed);
    assert!(!clean.contains("fake-secret"));
    assert!(clean.contains("gridCount:40"));
    let clean = privacy::safe_url(
        "https://fixture.example/grid/list?signature=secret&strategyId=123#accessToken=secret",
    )
    .unwrap();
    assert!(!clean.contains("secret"));
    assert!(clean.contains("strategyId=123"));
    assert!(privacy::clean_body(&"x".repeat(BODY_LIMIT + 1)).is_err());
    let (clean, _) = privacy::clean_body(
        r#"{"token":"SYNTHETIC_SECRET","sessionId":"SYNTHETIC_SESSION","gridCount":20}"#,
    )
    .unwrap();
    assert!(!clean.contains("SYNTHETIC_SECRET"));
    assert!(!clean.contains("SYNTHETIC_SESSION"));
    assert!(clean.contains("20"));
    let (clean,_)=privacy::clean_body(r#"<meta name="csrf-token" content="SYNTHETIC_SECRET"><input value="OTHER_SECRET" name="xsrf-token">"#).unwrap();
    assert!(!clean.contains("SYNTHETIC_SECRET"));
    assert!(!clean.contains("OTHER_SECRET"));
}

#[test]
fn manifests_reject_code_paths_and_origin_escalation() {
    let fixture = Fixture::new();
    let mut site = fixture.session.site.clone();
    site.id = "../profile".into();
    assert!(site.validate().is_err());
    site.id = "valid".into();
    site.resource_origins = vec!["https://*.example".into()];
    assert!(site.validate().is_err());
    site.resource_origins = vec!["https://fixture.example/path".into()];
    assert!(site.validate().is_err());
    assert!(
        serde_json::from_value::<SiteManifest>(json!({"javascript":"document.cookie"})).is_err()
    );
    let bundled: Vec<SiteManifest> =
        serde_json::from_str(include_str!("../../browser-research/sites.json")).unwrap();
    assert!(!bundled.is_empty());
    assert!(bundled.iter().all(|site| site.validate().is_ok()));
    let mut site = fixture.session.site.clone();
    site.navigation_origins
        .push(site.navigation_origins[0].clone());
    assert!(site.validate().is_err());
    let mut site = fixture.session.site.clone();
    site.entry_url.push_str("?access_token=SYNTHETIC_SECRET");
    assert!(site.validate().is_err());
}

#[test]
fn response_pagination_continues_after_short_request_ends() {
    let mut fixture = Fixture::new();
    let response = "x".repeat(100);
    let mut event = fixture.event(
        "request",
        "https://api.fixture.example/detail",
        Some(&response),
    );
    event.request_body = Some("{}".into());
    ingest::accept(&mut fixture.session, &fixture.root, event).unwrap();
    let result=query::execute(&fixture.root,&fixture.session,&command(json!({"kind":"read_request","request_id":fixture.session.requests[0].id,"offset":20,"limit":10}))).unwrap();
    assert_eq!(result["request_body"]["complete"], true);
    assert_eq!(result["request_body"]["content"], "");
    assert_eq!(result["response_body"]["content"], "xxxxxxxxxx");
}

#[test]
fn sampled_search_reports_partial_and_same_transport_id_cannot_cross_urls() {
    let mut fixture = Fixture::new();
    let event = fixture.event(
        "resource",
        "https://fixture.example/chunk.js",
        Some(&"grid ".repeat(25)),
    );
    ingest::accept(&mut fixture.session, &fixture.root, event).unwrap();
    let result = query::execute(
        &fixture.root,
        &fixture.session,
        &command(json!({"kind":"search","query":"grid"})),
    )
    .unwrap();
    assert_eq!(result["partial"], true);
    assert_eq!(result["total"], 20);
    for url in [
        "https://api.fixture.example/old",
        "https://api.fixture.example/new",
    ] {
        let event = fixture.event("request", url, Some("{}"));
        ingest::accept(&mut fixture.session, &fixture.root, event).unwrap();
    }
    assert_eq!(fixture.session.requests.len(), 2);
    assert_ne!(
        fixture.session.requests[0].id,
        fixture.session.requests[1].id
    );
}
