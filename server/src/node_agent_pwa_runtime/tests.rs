use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
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
        evidence: CaptureEvidenceInput {
            source_revision: Some(format!("fixture-sha256:{}", "a".repeat(64))),
            source_revisions: Default::default(),
            route_revision: "fixture-route-r1".to_string(),
        },
    }
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
async fn real_headless_fixture_produces_exact_decodable_png_and_cleans_process() {
    let root = project_root("e2e");
    let (url, server) = fixture(
        r#"<!doctype html><meta charset="utf-8"><style>html,body{margin:0;width:100%;height:100%;background:#14324a}#ready{width:120px;height:80px;background:#f2c94c}</style><main id="ready">PWA runtime proof</main>"#,
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
