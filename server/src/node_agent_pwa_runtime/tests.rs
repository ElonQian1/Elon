use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Notify,
    task::JoinHandle,
};

fn project_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon-pwa-runtime-{label}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn input(url: String) -> PwaCaptureInput {
    PwaCaptureInput {
        url,
        viewport: CaptureViewport {
            width: 360,
            height: 640,
            device_scale_factor: 1.0,
        },
        wait_for: CaptureWait {
            condition: "networkidle".to_string(),
            timeout_ms: 8_000,
            settle_ms: 100,
            selector: Some("#ready".to_string()),
        },
        capture: CaptureScope::default(),
        auth_profile: None,
        fixture_profile: None,
        steps: Vec::new(),
        evidence: CaptureEvidenceInput {
            source_revision: Some(format!("fixture-sha256:{}", "a".repeat(64))),
            source_revisions: Default::default(),
            route_revision: "fixture-route-r1".to_string(),
        },
    }
}

#[test]
fn interaction_wait_step_accepts_documented_camel_case_timeout() {
    let parsed: PwaCaptureInput = serde_json::from_value(json!({
        "url":"http://127.0.0.1:3000/",
        "viewport":{"width":360,"height":640},
        "steps":[{
            "action":"waitFor",
            "selector":"#ready",
            "state":"visible",
            "timeoutMs":2000
        }],
        "evidence":{
            "sourceRevision":format!("fixture-sha256:{}", "a".repeat(64)),
            "routeRevision":"fixture-route-r1"
        }
    }))
    .expect("tools/list schema should deserialize without field-name drift");
    assert!(matches!(
        parsed.steps.as_slice(),
        [CaptureInteractionStep::WaitFor {
            timeout_ms: 2_000,
            ..
        }]
    ));
}

#[tokio::test]
async fn real_headless_fixture_replays_click_and_text_assertion() {
    let root = project_root("interaction-replay");
    let (url, server) = fixture(
        r#"<!doctype html><body><button id="toggle" onclick="document.querySelector('#detail').hidden=false">展开</button><main id="ready">ready</main><p id="detail" hidden>第三行已展开</p></body>"#,
    )
    .await;
    let mut capture_input = input(url);
    capture_input.steps = vec![
        CaptureInteractionStep::Click {
            selector: "#toggle".into(),
        },
        CaptureInteractionStep::WaitFor {
            selector: "#detail".into(),
            state: "visible".into(),
            timeout_ms: 2_000,
        },
        CaptureInteractionStep::AssertText {
            selector: "#detail".into(),
            text: "第三行已展开".into(),
        },
    ];
    let result = capture(root.to_str().unwrap(), capture_input).await;
    if result.pointer("/diagnostic/code").and_then(Value::as_str) == Some("BROWSER_NOT_FOUND") {
        server.abort();
        fs::remove_dir_all(root).unwrap();
        return;
    }
    assert_eq!(result["ok"], true, "{result:#}");
    assert_eq!(result["interaction"]["executedStepCount"], 3);
    server.abort();
    fs::remove_dir_all(root).unwrap();
}

async fn fixture(html: &'static str) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });
    (
        format!("http://{address}/fixture?theme=proof#runtime"),
        task,
    )
}

#[test]
fn tool_schema_is_closed_and_does_not_accept_secret_arguments() {
    let tool = tool_definition();
    assert_eq!(tool["name"], TOOL_NAME);
    assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    assert!(tool["inputSchema"]["properties"].get("token").is_none());
    assert!(tool["inputSchema"]["properties"].get("cookies").is_none());
    assert_eq!(tool["annotations"]["openWorldHint"], false);
}

#[test]
fn security_gate_rejects_public_secret_and_oversized_inputs() {
    let root = project_root("security");
    let public = security::prepare(root.to_str().unwrap(), input("https://example.com/".into()))
        .unwrap_err();
    assert_eq!(public.code, "URL_ORIGIN_NOT_ALLOWED");

    let secret = security::prepare(
        root.to_str().unwrap(),
        input("http://127.0.0.1:4173/?access_token=do-not-store".into()),
    )
    .unwrap_err();
    assert_eq!(secret.code, "URL_SECRET_QUERY_REJECTED");
    assert!(!serde_json::to_string(&secret)
        .unwrap()
        .contains("do-not-store"));

    let mut oversized = input("http://127.0.0.1:4173/".into());
    oversized.viewport.width = 4096;
    oversized.viewport.height = 4096;
    oversized.viewport.device_scale_factor = 4.0;
    assert_eq!(
        security::prepare(root.to_str().unwrap(), oversized)
            .unwrap_err()
            .code,
        "VIEWPORT_PIXEL_LIMIT"
    );
    assert_eq!(
        super::process::missing_browser_diagnostic_for_test().code,
        "BROWSER_NOT_FOUND"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_revision_paths_and_auth_profiles_are_project_scoped() {
    let root = project_root("revision");
    let mut unsafe_revision = input("http://localhost:4173/".into());
    unsafe_revision.evidence.source_revision = None;
    unsafe_revision
        .evidence
        .source_revisions
        .insert("../outside.ts".to_string(), "a".repeat(64));
    assert_eq!(
        security::prepare(root.to_str().unwrap(), unsafe_revision)
            .unwrap_err()
            .code,
        "REVISION_INVALID"
    );
    let mut auth = input("http://localhost:4173/".into());
    auth.auth_profile = Some("local".to_string());
    assert_eq!(
        security::prepare(root.to_str().unwrap(), auth)
            .unwrap_err()
            .code,
        "AUTH_PROFILE_NOT_PREPARED"
    );
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn temporary_profile_cleanup_retries_until_delayed_unlock() {
    let root = project_root("cleanup-delayed-unlock");
    let profile = root.join("profile");
    fs::create_dir_all(&profile).unwrap();
    fs::write(profile.join("locked"), b"fixture").unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_remove = attempts.clone();

    let removed = process::remove_temporary_profile_for_test(
        &profile,
        Duration::from_millis(250),
        Duration::from_millis(2),
        Duration::from_millis(8),
        move |path| {
            if attempts_for_remove.fetch_add(1, Ordering::SeqCst) < 3 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "simulated delayed Windows profile lock",
                ));
            }
            fs::remove_dir_all(path)
        },
    )
    .await;

    assert!(removed);
    assert_eq!(attempts.load(Ordering::SeqCst), 4);
    assert!(!profile.exists());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::remove_dir(root).unwrap();
}

#[tokio::test]
async fn temporary_profile_cleanup_fails_closed_after_bounded_retry_window() {
    let root = project_root("cleanup-permanent-lock");
    let profile = root.join("profile");
    fs::create_dir_all(&profile).unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_for_remove = attempts.clone();
    let started = tokio::time::Instant::now();

    let removed = process::remove_temporary_profile_for_test(
        &profile,
        Duration::from_millis(35),
        Duration::from_millis(5),
        Duration::from_millis(10),
        move |_| {
            attempts_for_remove.fetch_add(1, Ordering::SeqCst);
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "simulated permanent Windows profile lock",
            ))
        },
    )
    .await;

    assert!(!removed);
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(attempts.load(Ordering::SeqCst) >= 4);
    assert!(profile.exists());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn temporary_profile_cleanup_backoff_is_cancellable() {
    let root = project_root("cleanup-cancel");
    let profile = root.join("profile");
    fs::create_dir_all(&profile).unwrap();
    let first_attempt = Arc::new(Notify::new());
    let first_attempt_for_remove = first_attempt.clone();
    let profile_for_cleanup = profile.clone();
    let cleanup = tokio::spawn(async move {
        process::remove_temporary_profile_for_test(
            &profile_for_cleanup,
            Duration::from_secs(30),
            Duration::from_secs(30),
            Duration::from_secs(30),
            move |_| {
                first_attempt_for_remove.notify_one();
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "simulated lock while cancellation is requested",
                ))
            },
        )
        .await
    });

    tokio::time::timeout(Duration::from_secs(1), first_attempt.notified())
        .await
        .expect("cleanup should try removal before backing off");
    cleanup.abort();
    assert!(tokio::time::timeout(Duration::from_secs(1), cleanup)
        .await
        .expect("cancelled cleanup should stop promptly")
        .unwrap_err()
        .is_cancelled());
    assert!(profile.exists());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn real_headless_http_200_fixture_ignores_long_lived_channel_and_captures_png() {
    let root = project_root("e2e");
    fs::create_dir_all(root.join(".elon")).unwrap();
    fs::write(
        root.join(".elon/ui-pwa-runtime.json"),
        r##"{
          "authenticatedReadySelector": "#authenticated-app:not(.hidden)"
        }"##,
    )
    .unwrap();
    let (url, server) = fixture(
        r#"<!doctype html><meta charset="utf-8"><style>html,body{margin:0;width:100%;height:100%;background:#14324a}#ready{width:120px;height:80px;background:#f2c94c}</style><main id="ready">PWA runtime proof</main><script>window.channel = new EventSource('/events')</script>"#,
    )
    .await;
    let mut capture_input = input(url);
    capture_input.viewport.device_scale_factor = 1.5;
    let result = capture(root.to_str().unwrap(), capture_input).await;
    if result.pointer("/diagnostic/code").and_then(Value::as_str) == Some("BROWSER_NOT_FOUND") {
        server.abort();
        fs::remove_dir_all(root).unwrap();
        return;
    }
    assert_eq!(result["ok"], true, "{result:#}");
    assert_eq!(result["status"], "CAPTURED");
    assert_eq!(result["artifact"]["mediaType"], "image/png");
    assert_eq!(result["artifact"]["width"], 540);
    assert_eq!(result["artifact"]["height"], 960);
    assert_eq!(result["route"]["path"], "/fixture");
    assert_eq!(result["route"]["queryKeys"], json!(["theme"]));
    assert_eq!(result["revision"]["routeRevision"], "fixture-route-r1");
    assert_eq!(result["browser"]["headless"], true);
    assert!(result["browser"]["product"].as_str().unwrap().contains('/'));
    assert_eq!(result["processCleanup"]["browserProcessReaped"], true);
    assert_eq!(result["processCleanup"]["temporaryProfileRemoved"], true);
    assert_eq!(result["base64Embedded"], false);
    let artifact = PathBuf::from(result["artifact"]["path"].as_str().unwrap());
    assert!(artifact.is_absolute());
    let bytes = fs::read(&artifact).unwrap();
    let image = image::load_from_memory(&bytes).unwrap();
    assert_eq!((image.width(), image.height()), (540, 960));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        result["artifact"]["sha256"].as_str().unwrap()
    );
    let manifest =
        fs::read_to_string(result["artifact"]["manifestPath"].as_str().unwrap()).unwrap();
    assert!(!manifest.contains("base64,"));
    assert!(!manifest.contains("theme=proof"));
    server.abort();
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn real_headless_fixture_reports_timeout_and_auth_without_false_png() {
    let root = project_root("failures");
    let (url, server) = fixture(r#"<!doctype html><main id="ready">ready</main>"#).await;
    let mut timeout_input = input(url);
    timeout_input.wait_for.selector = Some("#never".to_string());
    timeout_input.wait_for.timeout_ms = 500;
    let timeout = capture(root.to_str().unwrap(), timeout_input).await;
    if timeout.pointer("/diagnostic/code").and_then(Value::as_str) == Some("BROWSER_NOT_FOUND") {
        server.abort();
        fs::remove_dir_all(root).unwrap();
        return;
    }
    assert_eq!(timeout["diagnostic"]["code"], "WAIT_TIMEOUT", "{timeout:#}");
    assert_eq!(
        timeout["diagnostic"]["details"]["pageState"]["documentReadyState"],
        "complete"
    );
    assert_eq!(
        timeout["diagnostic"]["details"]["pageState"]["documentStatus"],
        200
    );
    assert_eq!(
        timeout["diagnostic"]["details"]["pageState"]["selectorResult"]["waitSelectorFound"],
        false
    );
    assert!(timeout["diagnostic"]["details"]["pageState"]["finalUrl"]
        .as_str()
        .unwrap()
        .starts_with("http://127.0.0.1:"));
    assert_eq!(
        timeout["diagnostic"]["details"]["browserStderr"]["captured"],
        true
    );
    assert!(timeout.get("artifact").is_none());
    server.abort();

    let (auth_url, auth_server) = fixture(
        r#"<!doctype html><form action="/login"><input type="password"><button>Login</button></form>"#,
    )
    .await;
    let mut auth_input = input(auth_url);
    auth_input.wait_for.selector = None;
    let auth = capture(root.to_str().unwrap(), auth_input).await;
    assert_eq!(
        auth["diagnostic"]["code"], "AUTHENTICATION_REQUIRED",
        "{auth:#}"
    );
    assert!(auth.get("artifact").is_none());
    auth_server.abort();
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn real_headless_fixture_uses_ephemeral_local_storage_auth() {
    let root = project_root("local-storage-auth");
    let session_dir = root.join(".elon/ui-tuner/pwa-sessions");
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        session_dir.join("fixture_auth.json"),
        serde_json::to_vec(&json!({
            "version": 1,
            "localStorage": { "lodex_token": "fixture-token" }
        }))
        .unwrap(),
    )
    .unwrap();
    let (url, server) = fixture(
        r#"<!doctype html><body><form id="login"><input type="password"></form><script>if(localStorage.getItem('lodex_token')==='fixture-token'){setTimeout(()=>{document.querySelector('#login').style.display='none';document.body.insertAdjacentHTML('beforeend','<main id="ready">authenticated</main>')},200)}</script></body>"#,
    )
    .await;
    let mut capture_input = input(url);
    capture_input.auth_profile = Some("fixture_auth".to_string());
    capture_input.wait_for.settle_ms = 500;
    let result = capture(root.to_str().unwrap(), capture_input).await;
    if result.pointer("/diagnostic/code").and_then(Value::as_str) == Some("BROWSER_NOT_FOUND") {
        server.abort();
        fs::remove_dir_all(root).unwrap();
        return;
    }
    assert_eq!(result["ok"], true, "{result:#}");
    assert_eq!(result["authentication"]["mode"], "prepared_profile");
    assert_eq!(result["processCleanup"]["temporaryProfileRemoved"], true);
    server.abort();
    fs::remove_dir_all(root).unwrap();
}
