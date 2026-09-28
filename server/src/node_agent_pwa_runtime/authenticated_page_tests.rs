//! Password controls on an authenticated settings page are not a login form.
use super::*;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const SECURITY: &str = r#"<!doctype html><main id="appView"><h1>Account security</h1><form action="/password/change"><input type="password" autocomplete="current-password"><input type="password" autocomplete="new-password"><input type="password" autocomplete="new-password"><button id="save">Update password</button></form></main>"#;
const LOGIN: &str =
    r#"<!doctype html><main id="loginView"><input type="password"><button>Sign in</button></main>"#;

async fn capture(
    html: &'static str,
    status: u16,
    route: &str,
    marker: bool,
    profile: bool,
) -> Value {
    capture_with_cookie(html, status, route, marker, profile, "local-fixture").await
}

async fn capture_with_cookie(
    html: &'static str,
    status: u16,
    route: &str,
    marker: bool,
    profile: bool,
    cookie_value: &str,
) -> Value {
    let root: PathBuf =
        std::env::temp_dir().join(format!("elon-auth-page-{}", uuid::Uuid::new_v4()));
    let sessions = root.join(".elon/ui-tuner/pwa-sessions");
    fs::create_dir_all(&sessions).unwrap();
    if marker {
        fs::write(
            root.join(".elon/ui-pwa-runtime.json"),
            r##"{"authenticatedReadySelector":"#appView:not(.hidden)"}"##,
        )
        .unwrap();
    }
    // Synthetic local-only session; no real credentials or production service.
    fs::write(
        sessions.join("test-auth.json"),
        serde_json::to_vec(
            &json!({"version":1,"cookies":[{"name":"test-session","value":cookie_value}]}),
        )
        .unwrap(),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}{route}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut request = [0u8; 8192];
            let length = stream.read(&mut request).await.unwrap_or(0);
            let authenticated =
                String::from_utf8_lossy(&request[..length]).contains("test-session=local-fixture");
            let body = if authenticated { html } else { LOGIN };
            let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });
    let mut args = json!({
        "url":url,"viewport":{"width":411,"height":842},
        "waitFor":{"condition":"load","selector":"main","timeoutMs":1500},
        "evidence":{"sourceRevision":"auth-page-boundary-test","routeRevision":"security-v1"}
    });
    if profile {
        args["authProfile"] = json!("test-auth");
    }
    let result = capture_tool(root.to_str(), args).await;
    server.abort();
    fs::remove_dir_all(root).unwrap();
    result
}

#[tokio::test]
async fn real_authenticated_security_page_with_password_fields_is_captured() {
    for html in [
        SECURITY,
        r#"<main id="appView">Authenticated app</main><section role="dialog"><h1>Account security</h1><input type="password" autocomplete="current-password"><input type="password" autocomplete="new-password"></section>"#,
    ] {
        let result = capture(html, 200, "/account/security", true, true).await;
        assert_eq!(result["ok"], true, "real browser required: {result:#}");
        assert_eq!(result["authentication"]["mode"], "prepared_profile");
    }
}

#[tokio::test]
async fn real_expired_profile_returning_login_is_rejected() {
    let result = capture_with_cookie(
        SECURITY,
        200,
        "/account/security",
        true,
        true,
        "expired-session",
    )
    .await;
    assert_eq!(
        result["diagnostic"]["code"], "AUTHENTICATION_FAILED",
        "{result:#}"
    );
    assert!(result.get("artifact").is_none());
}

#[tokio::test]
async fn real_authentication_rejections_survive_visible_ready_markers() {
    for (html, status, route, marker, profile, expected) in [
        (
            SECURITY,
            401,
            "/account/security",
            true,
            true,
            "AUTHENTICATION_FAILED",
        ),
        (
            SECURITY,
            403,
            "/account/security",
            true,
            true,
            "AUTHENTICATION_FAILED",
        ),
        (SECURITY, 200, "/login", true, true, "AUTHENTICATION_FAILED"),
        (LOGIN, 200, "/", true, true, "AUTHENTICATION_FAILED"),
        (
            SECURITY,
            200,
            "/account/security",
            false,
            true,
            "AUTHENTICATION_FAILED",
        ),
        (
            SECURITY,
            200,
            "/account/security",
            true,
            false,
            "AUTHENTICATION_REQUIRED",
        ),
        (
            r#"<main id="appView" style="display:none">App</main><input type="password">"#,
            200,
            "/",
            true,
            true,
            "AUTHENTICATION_FAILED",
        ),
        (
            r#"<main id="appView">App</main><main id="appView">Duplicate</main><input type="password">"#,
            200,
            "/",
            true,
            true,
            "AUTHENTICATION_FAILED",
        ),
        (
            r#"<main id="appView">App</main><form action="/login"><input type="password"></form>"#,
            200,
            "/",
            true,
            true,
            "AUTHENTICATION_FAILED",
        ),
        (
            r#"<main id="appView">App</main><form action="/signin"><button>Sign in</button></form>"#,
            200,
            "/",
            true,
            true,
            "AUTHENTICATION_FAILED",
        ),
    ] {
        let result = capture(html, status, route, marker, profile).await;
        assert_eq!(
            result["diagnostic"]["code"], expected,
            "{route} {status}: {result:#}"
        );
        assert!(
            result.get("artifact").is_none(),
            "failed authentication must not produce evidence"
        );
    }
}
