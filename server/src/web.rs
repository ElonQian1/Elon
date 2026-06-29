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
const TAB_PROFILE_PNG_B64: &str = include_str!("assets/ic_tab_profile_user.b64");
const HOME_PROJECT_MARKER_PNG_B64: &str = include_str!("assets/ic_home_project_marker.b64");
const HOME_PULL_FILTER_PNG_B64: &str = include_str!("assets/ic_home_pull_filter.b64");
const PROJECT_AI_ICON_PNG_B64: &str = include_str!("assets/ic_project_ai_conversation.b64");
const PROJECT_DOCUMENT_ICON_PNG_B64: &str = include_str!("assets/ic_project_document.b64");
const SIDE_MENU_FOLDER_CLOSED_ICON_PNG: &[u8] =
    include_bytes!("../../android/app/src/main/res/drawable-xxxhdpi/ic_side_menu_folder_closed.png");
const PROJECT_MEMBERS_TOOLBAR_ICON_PNG: &[u8] =
    include_bytes!("../../android/app/src/main/res/drawable-xxxhdpi/ic_project_members_toolbar.png");
const PROJECT_SPACE_POST_SHARE_ICON_PNG: &[u8] =
    include_bytes!("../../android/app/src/main/res/drawable/ic_project_space_post_share.png");
const PROJECT_SPACE_POST_COMMENT_ICON_PNG: &[u8] =
    include_bytes!("../../android/app/src/main/res/drawable/ic_project_space_post_comment.png");
const PROJECT_SPACE_POST_LIKE_ICON_PNG: &[u8] =
    include_bytes!("../../android/app/src/main/res/drawable/ic_project_space_post_like.png");
const PLAZA_ENTER_SPACE_ICON_PNG: &[u8] = include_bytes!("assets/ic_plaza_enter_space.png");
const PLAZA_SHARE_PROJECT_ICON_PNG: &[u8] = include_bytes!("assets/ic_plaza_share_project.png");
const PLAZA_DOWNLOAD_APK_ICON_PNG: &[u8] = include_bytes!("assets/ic_plaza_download_apk.png");
const ADD_FRIEND_SCAN_ICON_PNG: &[u8] = include_bytes!("assets/ic_add_friend_scan.png");
const POPUP_NEW_PROJECT_PNG_B64: &str = include_str!("assets/ic_popup_new_project.b64");
const CHAT_SIDE_MENU_HANDLE_PNG_B64: &str = include_str!("assets/ic_chat_side_menu_handle.b64");
const PROJECT_PLAZA_CSS: &str = include_str!("assets/project_plaza.css");
const PROJECT_PLAZA_JS: &str = include_str!("assets/project_plaza.js");
const PROJECT_HOME_CSS: &str = include_str!("assets/project_home.css");
const PROJECT_HOME_JS: &str = include_str!("assets/project_home.js");
const VOICE_TTS_SDK_JS: &str = include_str!("assets/voice_tts_sdk.js");
const ELON_ROUTE_C_SDK_JS: &str = include_str!("assets/elon_route_c_sdk.js");

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
        .replace("__TAB_PROFILE_PNG_B64__", TAB_PROFILE_PNG_B64.trim())
        .replace(
            "__HOME_PROJECT_MARKER_PNG_B64__",
            HOME_PROJECT_MARKER_PNG_B64.trim(),
        )
        .replace(
            "__HOME_PULL_FILTER_PNG_B64__",
            HOME_PULL_FILTER_PNG_B64.trim(),
        )
        .replace(
            "__PROJECT_AI_ICON_PNG_B64__",
            PROJECT_AI_ICON_PNG_B64.trim(),
        )
        .replace(
            "__PROJECT_DOCUMENT_ICON_PNG_B64__",
            PROJECT_DOCUMENT_ICON_PNG_B64.trim(),
        )
        .replace(
            "__POPUP_NEW_PROJECT_PNG_B64__",
            POPUP_NEW_PROJECT_PNG_B64.trim(),
        )
        .replace(
            "__CHAT_SIDE_MENU_HANDLE_PNG_B64__",
            CHAT_SIDE_MENU_HANDLE_PNG_B64.trim(),
        )
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

pub async fn project_plaza_enter_space_icon() -> impl IntoResponse {
    plaza_icon_response(PLAZA_ENTER_SPACE_ICON_PNG)
}

pub async fn project_plaza_share_project_icon() -> impl IntoResponse {
    plaza_icon_response(PLAZA_SHARE_PROJECT_ICON_PNG)
}

pub async fn project_plaza_download_apk_icon() -> impl IntoResponse {
    plaza_icon_response(PLAZA_DOWNLOAD_APK_ICON_PNG)
}

pub async fn side_menu_folder_closed_icon() -> impl IntoResponse {
    plaza_icon_response(SIDE_MENU_FOLDER_CLOSED_ICON_PNG)
}

pub async fn project_members_toolbar_icon() -> impl IntoResponse {
    plaza_icon_response(PROJECT_MEMBERS_TOOLBAR_ICON_PNG)
}

pub async fn project_space_post_share_icon() -> impl IntoResponse {
    plaza_icon_response(PROJECT_SPACE_POST_SHARE_ICON_PNG)
}

pub async fn project_space_post_comment_icon() -> impl IntoResponse {
    plaza_icon_response(PROJECT_SPACE_POST_COMMENT_ICON_PNG)
}

pub async fn project_space_post_like_icon() -> impl IntoResponse {
    plaza_icon_response(PROJECT_SPACE_POST_LIKE_ICON_PNG)
}

pub async fn add_friend_scan_icon() -> impl IntoResponse {
    plaza_icon_response(ADD_FRIEND_SCAN_ICON_PNG)
}

fn plaza_icon_response(bytes: &'static [u8]) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        bytes,
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

pub async fn voice_tts_sdk_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        VOICE_TTS_SDK_JS,
    )
}

pub async fn elon_route_c_sdk_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        ELON_ROUTE_C_SDK_JS,
    )
}

/// GET /pc/* — SPA fallback：所有未匹配的路径都返回 index.html（200），让 React Router 接管。
pub async fn pc_spa_index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let path = state.data_dir.join("pc-next-dist/index.html");
    match tokio::fs::read(&path).await {
        Ok(bytes) => (
            axum::http::StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
            ],
            bytes,
        ).into_response(),
        Err(_) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "PC 前端尚未部署，请稍候重试。",
        ).into_response(),
    }
}
