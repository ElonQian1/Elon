// 网页版（APK 桌面投影）
//
// 设计原则：APK 是真理来源，网页只是把 APK 投影到浏览器上。
// 视觉、交互、文案对齐 android/app/src/main/res/layout/activity_main.xml。
// 响应式断点：< 720 手机、720~1100 平板、>= 1100 桌面（左侧栏 Tab）。

use std::sync::{Arc, OnceLock};

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::{Html, IntoResponse},
};

use crate::types::AppState;

const BRAND_PNG_B64: &str = include_str!("assets/ic_app_brand.b64");
const TAB_CHAT_PNG_B64: &str = include_str!("assets/ic_tab_chat_edit.b64");
const TAB_PROJECT_PNG_B64: &str = include_str!("assets/ic_tab_project_stack.b64");
const PROJECT_PLAZA_CSS: &str = include_str!("assets/project_plaza.css");
const PROJECT_PLAZA_JS: &str = include_str!("assets/project_plaza.js");
const PROJECT_HOME_CSS: &str = include_str!("assets/project_home.css");
const PROJECT_HOME_JS: &str = include_str!("assets/project_home.js");

pub async fn web_page() -> impl IntoResponse {
    static HTML: OnceLock<String> = OnceLock::new();
    let body = HTML.get_or_init(build_html).as_str();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    );
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(header::EXPIRES, HeaderValue::from_static("0"));
    (headers, Html(body))
}

pub async fn download_page(State(state): State<Arc<AppState>>) -> Html<String> {
    let public_url = state.public_url.trim_end_matches('/');
    let apk_url = format!("{public_url}/app/ElonSpeed-latest.apk");
    let page_url = format!("{public_url}/app/download");
    Html(
        DOWNLOAD_HTML_TEMPLATE
            .replace("__APK_URL__", &apk_url)
            .replace("__PAGE_URL__", &page_url)
            .replace("__BRAND_PNG_B64__", BRAND_PNG_B64.trim()),
    )
}

fn build_html() -> String {
    WEB_HTML_TEMPLATE
        .replace("__BRAND_PNG_B64__", BRAND_PNG_B64.trim())
        .replace("__TAB_CHAT_PNG_B64__", TAB_CHAT_PNG_B64.trim())
        .replace("__TAB_PROJECT_PNG_B64__", TAB_PROJECT_PNG_B64.trim())
}

const WEB_HTML_TEMPLATE: &str = include_str!("assets/web_page.html");
const DOWNLOAD_HTML_TEMPLATE: &str = include_str!("assets/download_page.html");

/// PWA manifest.json —— 让 iOS/Android 浏览器把网页识别为可安装应用。
pub async fn pwa_manifest() -> impl IntoResponse {
    let body = r##"{
  "name": "一龙 · 云端开发",
  "short_name": "一龙",
  "description": "用自然语言开发你的 App",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#101010",
  "theme_color": "#101010",
  "icons": [
    {
      "src": "/app/icon-192.png",
      "sizes": "192x192",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "/app/icon-512.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "any maskable"
    }
  ]
}"##;
    (
        [
            (header::CONTENT_TYPE, "application/manifest+json"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        body,
    )
}

/// Service Worker —— 仅注册壳，暂不做离线缓存，只保证 PWA 安装条件满足。
pub async fn service_worker() -> impl IntoResponse {
    let body = r#"// 一龙 Service Worker
// 当前仅作为 PWA 安装壳，不缓存任何请求。
self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (e) => e.waitUntil(self.clients.claim()));
"#;
    (
        [
            (header::CONTENT_TYPE, "application/javascript"),
            // SW 必须设短缓存，否则更新不及时
            (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
        ],
        body,
    )
}

pub async fn project_plaza_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PROJECT_PLAZA_CSS,
    )
}

pub async fn project_plaza_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PROJECT_PLAZA_JS,
    )
}

pub async fn project_home_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PROJECT_HOME_CSS,
    )
}

pub async fn project_home_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PROJECT_HOME_JS,
    )
}
