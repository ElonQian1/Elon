use super::*;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

const LOGIN: &str = r#"<!doctype html><main id="loginView"><h1>Sign in</h1><form action="/login"><label>Password<input type="password"></label></form></main>"#;

fn root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("elon-public-login-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(root.join(".elon")).unwrap();
    root
}

fn input(url: &str) -> Value {
    json!({
        "url":url,"viewport":{"width":411,"height":842},
        "waitFor":{"condition":"load","selector":"#loginView","timeoutMs":1000},
        "expectedPage":{"kind":"PUBLIC_LOGIN","pageId":"main-login","path":reqwest::Url::parse(url).unwrap().path(),"readySelector":"#loginView"},
        "evidence":{"sourceRevision":"test-revision","routeRevision":"login-v2"}
    })
}

fn prepare(
    root: &std::path::Path,
    value: Value,
) -> Result<security::PreparedCapture, CaptureDiagnostic> {
    security::prepare(
        root.to_str().unwrap(),
        serde_json::from_value(value).unwrap(),
    )
}

async fn fixture(html: &'static str, status: u16) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut buffer = [0_u8; 8192];
            let _ = stream.read(&mut buffer).await;
            let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len());
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });
    (url, task)
}

#[test]
fn declaration_is_closed_and_bound_to_requested_path() {
    let root = root();
    for (field, bad) in [
        ("kind", "SKIP_AUTH"),
        ("pageId", "unsafe label"),
        ("path", "/other"),
        ("path", "/?token=secret"),
        ("readySelector", ""),
    ] {
        let mut value = input("http://127.0.0.1:3000/");
        value["expectedPage"][field] = json!(bad);
        let parsed = serde_json::from_value(value);
        assert!(
            parsed.is_err() || security::prepare(root.to_str().unwrap(), parsed.unwrap()).is_err()
        );
    }
    for field in ["kind", "pageId", "path", "readySelector"] {
        let mut value = input("http://127.0.0.1:3000/");
        value["expectedPage"].as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<PwaCaptureInput>(value).is_err());
    }
    assert_eq!(
        tool_definition()["inputSchema"]["properties"]["expectedPage"]["additionalProperties"],
        false
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_login_refuses_explicit_and_default_auth_without_reading_credentials() {
    let root = root();
    let mut value = input("http://127.0.0.1:3000/");
    value["authProfile"] = json!("unprepared");
    assert_eq!(
        prepare(&root, value).unwrap_err().code,
        "PUBLIC_LOGIN_AUTH_PROFILE_CONFLICT"
    );
    fs::write(
        root.join(".elon/ui-pwa-runtime.json"),
        r#"{"defaultAuthProfile":"unprepared"}"#,
    )
    .unwrap();
    assert_eq!(
        prepare(&root, input("http://127.0.0.1:3000/"))
            .unwrap_err()
            .code,
        "PUBLIC_LOGIN_AUTH_PROFILE_CONFLICT"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_login_allows_only_observational_steps_and_keeps_url_security() {
    let root = root();
    for step in [
        json!({"action":"click","selector":"button"}),
        json!({"action":"fill","selector":"input","fixture_key":"demo"}),
        json!({"action":"pressKey","key":"Enter"}),
        json!({"action":"setChecked","selector":"input","checked":true}),
        json!({"action":"selectOption","selector":"select","fixture_key":"demo"}),
        json!({"action":"previewStyle","selector":"main","patches":[{"property":"color","value":"red"}]}),
        json!({"action":"restoreStyle","selector":"main"}),
    ] {
        let mut value = input("http://127.0.0.1:3000/");
        value["steps"] = json!([step]);
        assert_eq!(
            prepare(&root, value).unwrap_err().code,
            "PUBLIC_LOGIN_CAPTURE_ONLY"
        );
    }
    let mut value = input("http://127.0.0.1:3000/");
    value["steps"] = json!([
        {"action":"waitFor","selector":"#loginView"},
        {"action":"assertText","selector":"h1","text":"Sign in"},
        {"action":"scrollIntoView","selector":"#loginView"}
    ]);
    assert!(prepare(&root, value).is_ok());
    assert_eq!(
        prepare(&root, input("https://untrusted.example/"))
            .unwrap_err()
            .code,
        "URL_ORIGIN_NOT_ALLOWED"
    );
    assert_eq!(
        prepare(&root, input("http://127.0.0.1:3000/?token=secret"))
            .unwrap_err()
            .code,
        "URL_SECRET_QUERY_REJECTED"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn page_identity_rejects_redirects_even_to_another_allowed_origin() {
    let root = root();
    let prepared = prepare(&root, input("http://127.0.0.1:3000/")).unwrap();
    for href in ["http://127.0.0.1:3001/", "http://127.0.0.1:3000/other"] {
        assert_eq!(
            expected_page::ready(&prepared, href, &json!({"publicReady":true}))
                .unwrap_err()
                .code,
            "PUBLIC_LOGIN_PAGE_MISMATCH"
        );
    }
    assert!(!expected_page::ready(
        &prepared,
        "http://127.0.0.1:3000/",
        &json!({"publicReady":false})
    )
    .unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn real_browser_captures_declared_login_with_auditable_manifest_but_default_refuses() {
    let root = root();
    fs::write(
        root.join(".elon/ui-pwa-runtime.json"),
        r##"{"authenticatedReadySelector":"#appView:not(.hidden)"}"##,
    )
    .unwrap();
    let (url, server) = fixture(LOGIN, 200).await;
    let args = input(&url);
    let captured = capture_tool(root.to_str(), args.clone()).await;
    assert_eq!(captured["ok"], true, "real browser required: {captured:#}");
    assert_eq!(captured["authentication"]["mode"], "none");
    assert_eq!(captured["expectedPage"], args["expectedPage"]);
    let manifest: Value = serde_json::from_slice(
        &fs::read(captured["artifact"]["manifestPath"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["expectedPage"], args["expectedPage"]);
    assert_eq!(manifest["authenticationMode"], "none");
    let mut normal = args;
    normal.as_object_mut().unwrap().remove("expectedPage");
    let failed = capture_tool(root.to_str(), normal).await;
    assert_eq!(failed["diagnostic"]["code"], "AUTHENTICATION_REQUIRED");
    assert!(failed.get("artifact").is_none());
    server.abort();
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn real_browser_rejects_http_denials_even_for_declared_login() {
    let root = root();
    for status in [401, 403] {
        let (url, server) = fixture(LOGIN, status).await;
        let result = capture_tool(root.to_str(), input(&url)).await;
        assert_eq!(
            result["diagnostic"]["code"], "AUTHENTICATION_REQUIRED",
            "{result:#}"
        );
        assert!(result.get("artifact").is_none());
        server.abort();
    }
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn real_browser_requires_one_visible_login_anchor_and_matching_route() {
    let root = root();
    for (html, code) in [
        (
            r#"<main id="loginView" hidden><input type="password"></main>"#,
            "WAIT_TIMEOUT",
        ),
        (
            r#"<main id="loginView"><input type="password"></main><main id="loginView"><input type="password"></main>"#,
            "WAIT_TIMEOUT",
        ),
        (
            r#"<main id="loginView">Signed in dashboard</main>"#,
            "WAIT_TIMEOUT",
        ),
        (
            r#"<main id="loginView"><input type="password"></main><script>history.replaceState(null,'','/different')</script>"#,
            "PUBLIC_LOGIN_PAGE_MISMATCH",
        ),
    ] {
        let (url, server) = fixture(html, 200).await;
        let result = capture_tool(root.to_str(), input(&url)).await;
        assert_eq!(result["diagnostic"]["code"], code, "{result:#}");
        assert!(result.get("artifact").is_none());
        server.abort();
    }
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn stateful_browser_cannot_reuse_public_login_as_authenticated_evidence() {
    let root = root();
    let (url, server) = fixture(LOGIN, 200).await;
    let key = format!("public-login-{}", uuid::Uuid::new_v4());
    let args = input(&url);
    let result = stateful::start(
        &key,
        root.to_str().unwrap(),
        serde_json::from_value(args.clone()).unwrap(),
        false,
    )
    .await;
    assert_eq!(result["ok"], true, "{result:#}");
    let mut changed = args;
    changed.as_object_mut().unwrap().remove("expectedPage");
    let result = stateful::start(
        &key,
        root.to_str().unwrap(),
        serde_json::from_value(changed).unwrap(),
        false,
    )
    .await;
    let stopped = stateful::stop(&key).await;
    assert_eq!(
        result["diagnostic"]["code"],
        "BROWSER_SESSION_BINDING_CHANGED"
    );
    assert_eq!(stopped["ok"], true);
    server.abort();
    fs::remove_dir_all(root).unwrap();
}
