// 网页版（APK 桌面投影）
//
// 设计原则：APK 是真理来源，网页只是把 APK 投影到浏览器上。
// 视觉、交互、文案对齐 android/app/src/main/res/layout/activity_main.xml。
// 响应式断点：< 720 手机、720~1100 平板、>= 1100 桌面（左侧栏 Tab）。

use std::sync::{Arc, OnceLock};

use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, header},
    response::{Html, IntoResponse},
};

use crate::types::AppState;

const BRAND_PNG_B64: &str = include_str!("assets/ic_app_brand.b64");
const TAB_CHAT_PNG_B64: &str = include_str!("assets/ic_tab_chat_edit.b64");
const TAB_PROJECT_PNG_B64: &str = include_str!("assets/ic_tab_project_stack.b64");

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
