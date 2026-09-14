//! Public main-app branding and APK aliases. Both URLs use one stable storage file
//! so older publishers and clients remain compatible during rolling upgrades.
use std::{path::Path, sync::LazyLock};

use axum::{http::header, http::HeaderValue, Router};
use serde::Deserialize;
use tower_http::{services::ServeFile, set_header::SetResponseHeaderLayer};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBranding {
    pub display_name: String,
    pub apk_file_name: String,
    pub legacy_apk_file_name: String,
}

pub static BRANDING: LazyLock<AppBranding> = LazyLock::new(|| {
    serde_json::from_str(include_str!("assets/app_branding.json"))
        .expect("checked-in main-app branding must be valid")
});

pub fn download_path() -> String {
    format!("/app/{}", BRANDING.apk_file_name)
}

pub fn download_url(base: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), download_path())
}

pub fn content_disposition() -> HeaderValue {
    format!("attachment; filename=\"{}\"", BRANDING.apk_file_name)
        .parse()
        .expect("APK filename must be an ASCII header value")
}

pub fn routes<S: Clone + Send + Sync + 'static>(app_dir: &Path) -> Router<S> {
    // Keep the old physical pathname: older release scripts atomically replace it.
    // A second copy or a symlink at that pathname could silently serve a stale APK.
    let apk = ServeFile::new(app_dir.join(&BRANDING.legacy_apk_file_name));
    Router::new()
        .route_service(&download_path(), apk.clone())
        .route_service(&format!("/app/{}", BRANDING.legacy_apk_file_name), apk)
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_DISPOSITION,
            content_disposition(),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
}

#[cfg(test)]
#[path = "app_distribution_tests.rs"]
mod tests;
