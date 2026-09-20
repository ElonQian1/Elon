//! Public home-screen identity and shell assets; no account state.
use crate::types::AppState;
use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::{
    io::Cursor,
    sync::{Arc, OnceLock},
};

const LOGO: &[u8] = include_bytes!("../../assets/brand/logo.png");
const MANIFEST: &str = r##"{
  "id": "/", "name": "一龙ai", "short_name": "一龙ai",
  "description": "好友与群聊、项目协作",
  "start_url": "/?tab=chat&source=pwa", "scope": "/", "display": "standalone",
  "background_color": "#0b1017", "theme_color": "#0b1017",
  "icons": [
    {"src":"/app/icon-192.png","sizes":"192x192","type":"image/png","purpose":"any"},
    {"src":"/app/icon-512.png","sizes":"512x512","type":"image/png","purpose":"any"}
  ]
}"##;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/manifest.json",
            get(|| async {
                (
                    [
                        (
                            header::CONTENT_TYPE,
                            "application/manifest+json; charset=utf-8",
                        ),
                        (header::CACHE_CONTROL, "no-cache"),
                    ],
                    MANIFEST,
                )
            }),
        )
        .route(
            "/sw.js",
            get(|| async {
                (
                    [
                        (header::CONTENT_TYPE, "application/javascript"),
                        (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
                    ],
                    include_str!("assets/mobile_shell_worker.js"),
                )
            }),
        )
        .route(
            "/assets/mobile_startup.js",
            get(|| async {
                (
                    [
                        (header::CONTENT_TYPE, "application/javascript"),
                        (header::CACHE_CONTROL, "no-cache"),
                    ],
                    include_str!("assets/mobile_startup.js"),
                )
            }),
        )
        .route("/app/icon-180.png", get(|| async { icon(180) }))
        .route("/apple-touch-icon.png", get(|| async { icon(180) }))
        .route("/app/icon-192.png", get(|| async { icon(192) }))
        .route("/app/icon-512.png", get(|| async { icon(512) }))
}

fn encode_icon(size: u32) -> Option<Vec<u8>> {
    let logo = image::load_from_memory(LOGO).ok()?;
    let image = logo.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    let mut png = Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
}

fn icon(size: u32) -> axum::response::Response {
    static ICONS: OnceLock<[Option<Vec<u8>>; 3]> = OnceLock::new();
    let icons = ICONS.get_or_init(|| [encode_icon(180), encode_icon(192), encode_icon(512)]);
    let index = match size {
        180 => 0,
        192 => 1,
        _ => 2,
    };
    match &icons[index] {
        Some(bytes) => (
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            bytes.clone(),
        )
            .into_response(),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn icons_match_manifest_dimensions_and_canonical_brand() {
        for size in [180, 192, 512] {
            let png = encode_icon(size).expect("canonical brand must decode");
            let decoded = image::load_from_memory(&png).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (size, size));
            let expected = image::load_from_memory(LOGO).unwrap().resize_exact(
                size,
                size,
                image::imageops::FilterType::Lanczos3,
            );
            assert_eq!(decoded.to_rgba8(), expected.to_rgba8());
        }
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["name"], "一龙ai");
        assert_eq!(manifest["short_name"], "一龙ai");
        assert_eq!(manifest["start_url"], "/?tab=chat&source=pwa");
    }
}
