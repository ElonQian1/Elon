use super::*;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

struct TestAppDir(std::path::PathBuf);
impl TestAppDir {
    fn new() -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("elon-apk-alias-{}-{unique}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        Self(dir)
    }
}
impl Drop for TestAppDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn canonical_url_and_storage_identity_are_stable() {
    assert_eq!(BRANDING.display_name, "一龙 AI");
    assert_eq!(BRANDING.legacy_apk_file_name, "ElonSpeed-latest.apk");
    assert_eq!(
        download_url("https://example.test/base/"),
        "https://example.test/base/app/ElonAI-latest.apk"
    );
}

#[tokio::test]
async fn old_and_new_links_serve_identical_current_bytes_with_head_and_ranges() {
    let dir = TestAppDir::new();
    let app = routes::<()>(&dir.0);
    let paths = [
        download_path(),
        format!("/app/{}", BRANDING.legacy_apk_file_name),
    ];
    for content in [b"first-apk".as_slice(), b"updated-apk-content".as_slice()] {
        std::fs::write(dir.0.join(&BRANDING.legacy_apk_file_name), content).unwrap();
        for path in &paths {
            let get = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(get.status(), StatusCode::OK);
            assert_eq!(
                get.headers()[header::CONTENT_DISPOSITION],
                "attachment; filename=\"ElonAI-latest.apk\""
            );
            assert_eq!(get.headers()[header::CACHE_CONTROL], "no-cache");
            assert_eq!(&to_bytes(get.into_body(), 100).await.unwrap()[..], content);

            let head = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("HEAD")
                        .uri(path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(head.status(), StatusCode::OK);
            assert_eq!(
                head.headers()[header::CONTENT_LENGTH],
                content.len().to_string()
            );
            assert!(to_bytes(head.into_body(), 100).await.unwrap().is_empty());

            let range = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .header(header::RANGE, "bytes=2-5")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(range.status(), StatusCode::PARTIAL_CONTENT);
            assert_eq!(
                range.headers()[header::CONTENT_RANGE],
                format!("bytes 2-5/{}", content.len())
            );
            assert_eq!(
                &to_bytes(range.into_body(), 100).await.unwrap()[..],
                &content[2..6]
            );
        }
    }
}

#[tokio::test]
async fn missing_artifact_returns_not_found_for_both_links() {
    let dir = TestAppDir::new();
    for path in [
        download_path(),
        format!("/app/{}", BRANDING.legacy_apk_file_name),
    ] {
        let response = routes::<()>(&dir.0)
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
