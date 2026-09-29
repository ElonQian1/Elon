use super::*;
use axum::body::to_bytes;
use std::future::IntoFuture;

const AUDIO: &[u8] = include_bytes!("../../scripts/fixtures/voice-tone.m4a");
struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("elon-voice-{}", uuid::Uuid::new_v4()));
        let directory = chat_attachment_dir(root.to_str().unwrap(), "fixture", "room");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("voice.m4a"), AUDIO).unwrap();
        Self(root)
    }
    async fn get(&self, method: &str, filename: &str, range: Option<&str>) -> Response {
        let mut request = Request::builder().method(method).uri("/voice.m4a");
        if let Some(range) = range {
            request = request.header(header::RANGE, range);
        }
        serve_chat_attachment(
            self.0.to_str().unwrap(),
            "fixture",
            "room",
            filename,
            request.body(Body::empty()).unwrap(),
        )
        .await
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn ios_probe_open_suffix_and_seek_ranges_serve_exact_m4a_bytes() {
    let root = Workspace::new();
    for (range, start, end) in [
        ("bytes=0-1".to_string(), 0, 1),
        ("bytes=100-199".to_string(), 100, 199),
        ("bytes=100-".to_string(), 100, AUDIO.len() - 1),
        ("bytes=-64".to_string(), AUDIO.len() - 64, AUDIO.len() - 1),
        (format!("bytes=0-{}", AUDIO.len() + 10), 0, AUDIO.len() - 1),
    ] {
        let response = root.get("GET", "voice.m4a", Some(&range)).await;
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT, "{range}");
        assert_eq!(response.headers()[header::CONTENT_TYPE], "audio/mp4");
        assert_eq!(response.headers()[header::ACCEPT_RANGES], "bytes");
        assert_eq!(
            response.headers()[header::CONTENT_RANGE],
            format!("bytes {start}-{end}/{}", AUDIO.len())
        );
        assert_eq!(
            response.headers()[header::CONTENT_LENGTH],
            (end - start + 1).to_string()
        );
        assert_eq!(
            &to_bytes(response.into_body(), AUDIO.len()).await.unwrap()[..],
            &AUDIO[start..=end]
        );
    }
}

#[tokio::test]
async fn full_download_and_head_remain_compatible() {
    let root = Workspace::new();
    for method in ["GET", "HEAD"] {
        let response = root.get(method, "voice.m4a", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "audio/mp4");
        assert_eq!(
            response.headers()[header::CONTENT_LENGTH],
            AUDIO.len().to_string()
        );
        assert_eq!(
            response.headers()[header::CONTENT_DISPOSITION],
            "inline; filename=\"voice.m4a\""
        );
        let bytes = to_bytes(response.into_body(), AUDIO.len()).await.unwrap();
        assert_eq!(&bytes[..], if method == "GET" { AUDIO } else { &[] });
    }
}

#[tokio::test]
async fn unsatisfiable_ranges_and_missing_or_traversal_paths_fail_closed() {
    let root = Workspace::new();
    let response = root
        .get("GET", "voice.m4a", Some(&format!("bytes={}-", AUDIO.len())))
        .await;
    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        response.headers()[header::CONTENT_RANGE],
        format!("bytes */{}", AUDIO.len())
    );
    assert!(to_bytes(response.into_body(), 1024)
        .await
        .unwrap()
        .is_empty());
    for filename in ["../outside.m4a", "..\\outside.m4a", "nested/voice.m4a"] {
        assert_eq!(
            root.get("GET", filename, Some("bytes=0-1")).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        root.get("GET", "missing.m4a", None).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn media_fixture_endpoint_for_browser_integration() {
    // Optional loopback-only fixture serves the actual production handler to browser tests.
    let Ok(port) = std::env::var("ELON_VOICE_FIXTURE_PORT") else {
        return;
    };
    let root = Workspace::new();
    let base = root.0.to_str().unwrap().to_string();
    let app =
        axum::Router::new().route(
            "/voice.m4a",
            axum::routing::get(move |request: Request| {
                let base = base.clone();
                async move {
                    serve_chat_attachment(&base, "fixture", "room", "voice.m4a", request).await
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    println!("VOICE_FIXTURE_READY={port}");
    // This explicitly requested fixture expires; ordinary unit tests never open a listener.
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(180),
        axum::serve(listener, app).into_future(),
    )
    .await;
}
