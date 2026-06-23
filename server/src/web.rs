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
const PROJECT_AI_ICON_PNG_B64: &str = include_str!("assets/ic_project_ai_conversation.b64");
const PROJECT_DOCUMENT_ICON_PNG_B64: &str = include_str!("assets/ic_project_document.b64");
const PROJECT_PLAZA_CSS: &str = include_str!("assets/project_plaza.css");
const PROJECT_PLAZA_JS: &str = include_str!("assets/project_plaza.js");
const PROJECT_HOME_CSS: &str = include_str!("assets/project_home.css");
const PROJECT_HOME_JS: &str = include_str!("assets/project_home.js");
const PC_APP_CSS: &str = include_str!("assets/pc_app.css");
const PC_APP_NODE_CSS: &str = include_str!("assets/pc_app_node.css");
const PC_APP_PROJECT_READINESS_CSS: &str = include_str!("assets/pc_app_project_readiness.css");
const PC_APP_DOCTOR_CSS: &str = include_str!("assets/pc_app_doctor.css");
const PC_PROJECT_LANDING_CSS: &str = include_str!("assets/pc_project_landing.css");
const PC_VOICE_PROJECT_CSS: &str = include_str!("assets/pc_voice_project.css");
const PC_APP_MODELS_CSS: &str = include_str!("assets/pc_app_models.css");
const PC_APP_DEV_COMPOSER_CSS: &str = include_str!("assets/pc_app_dev_composer.css");
const PC_APP_DEV_TASKS_CSS: &str = include_str!("assets/pc_app_dev_tasks.css");
const PC_APP_UTILS_JS: &str = include_str!("assets/pc_app_utils.js");
const PC_APP_MARKDOWN_JS: &str = include_str!("assets/pc_app_markdown.js");
const PC_APP_NODE_ADMIN_JS: &str = include_str!("assets/pc_app_node_admin.js");
const PC_APP_NODE_JS: &str = include_str!("assets/pc_app_node.js");
const PC_APP_PROJECT_READINESS_JS: &str = include_str!("assets/pc_app_project_readiness.js");
const PC_APP_DOCTOR_JS: &str = include_str!("assets/pc_app_doctor.js");
const PC_PROJECT_LANDING_JS: &str = include_str!("assets/pc_project_landing.js");
const VOICE_TTS_SDK_JS: &str = include_str!("assets/voice_tts_sdk.js");
const PC_VOICE_PROJECT_JS: &str = include_str!("assets/pc_voice_project.js");
const PC_APP_NOTIFICATIONS_JS: &str = include_str!("assets/pc_app_notifications.js");
const PC_APP_MODELS_JS: &str = include_str!("assets/pc_app_models.js");
const PC_APP_PROJECT_CREATE_JS: &str = include_str!("assets/pc_app_project_create.js");
const PC_APP_DEV_COMPOSER_JS: &str = include_str!("assets/pc_app_dev_composer.js");
const PC_APP_DEV_TASKS_JS: &str = include_str!("assets/pc_app_dev_tasks.js");
const PC_APP_AGENT_RUNS_JS: &str = include_str!("assets/pc_app_agent_runs.js");
const PC_APP_TASK_SNAPSHOTS_JS: &str = include_str!("assets/pc_app_task_snapshots.js");
const PC_APP_JS: &str = include_str!("assets/pc_app.js");

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

pub async fn pc_app_page() -> impl IntoResponse {
    static HTML: OnceLock<String> = OnceLock::new();
    let body = HTML
        .get_or_init(|| PC_APP_HTML_TEMPLATE.replace("__BRAND_PNG_B64__", BRAND_PNG_B64.trim()))
        .as_str();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
    );
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(header::EXPIRES, HeaderValue::from_static("0"));
    (headers, Html(body))
}

fn build_html() -> String {
    WEB_HTML_TEMPLATE
        .replace("__BRAND_PNG_B64__", BRAND_PNG_B64.trim())
        .replace("__TAB_CHAT_PNG_B64__", TAB_CHAT_PNG_B64.trim())
        .replace("__TAB_PROJECT_PNG_B64__", TAB_PROJECT_PNG_B64.trim())
        .replace(
            "__PROJECT_AI_ICON_PNG_B64__",
            PROJECT_AI_ICON_PNG_B64.trim(),
        )
        .replace(
            "__PROJECT_DOCUMENT_ICON_PNG_B64__",
            PROJECT_DOCUMENT_ICON_PNG_B64.trim(),
        )
}

const WEB_HTML_TEMPLATE: &str = include_str!("assets/web_page.html");
const PC_APP_HTML_TEMPLATE: &str = include_str!("assets/pc_app.html");
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

pub async fn pc_app_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_CSS,
    )
}

pub async fn pc_app_node_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_NODE_CSS,
    )
}

pub async fn pc_app_project_readiness_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_PROJECT_READINESS_CSS,
    )
}

pub async fn pc_app_doctor_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_DOCTOR_CSS,
    )
}

pub async fn pc_project_landing_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_PROJECT_LANDING_CSS,
    )
}

pub async fn pc_voice_project_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_VOICE_PROJECT_CSS,
    )
}

pub async fn pc_app_models_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_MODELS_CSS,
    )
}

pub async fn pc_app_dev_composer_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_DEV_COMPOSER_CSS,
    )
}

pub async fn pc_app_dev_tasks_css() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_DEV_TASKS_CSS,
    )
}

pub async fn pc_app_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_JS,
    )
}

pub async fn pc_app_node_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_NODE_JS,
    )
}

pub async fn pc_app_project_readiness_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_PROJECT_READINESS_JS,
    )
}

pub async fn pc_app_node_admin_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_NODE_ADMIN_JS,
    )
}

pub async fn pc_project_landing_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_PROJECT_LANDING_JS,
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

pub async fn pc_voice_project_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_VOICE_PROJECT_JS,
    )
}

pub async fn pc_app_notifications_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_NOTIFICATIONS_JS,
    )
}

pub async fn pc_app_models_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_MODELS_JS,
    )
}

pub async fn pc_app_project_create_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_PROJECT_CREATE_JS,
    )
}

pub async fn pc_app_dev_composer_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_DEV_COMPOSER_JS,
    )
}

pub async fn pc_app_dev_tasks_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_DEV_TASKS_JS,
    )
}

pub async fn pc_app_agent_runs_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_AGENT_RUNS_JS,
    )
}

pub async fn pc_app_task_snapshots_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_TASK_SNAPSHOTS_JS,
    )
}

pub async fn pc_app_utils_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_UTILS_JS,
    )
}

pub async fn pc_app_markdown_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_MARKDOWN_JS,
    )
}

pub async fn pc_app_doctor_js() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        PC_APP_DOCTOR_JS,
    )
}

#[cfg(test)]
mod tests {
    use super::PC_APP_TASK_SNAPSHOTS_JS;

    #[test]
    fn pc_task_snapshot_asset_keeps_local_journal_pagination_contract() {
        assert!(PC_APP_TASK_SNAPSHOTS_JS.contains("local_journal_has_more"));
        assert!(PC_APP_TASK_SNAPSHOTS_JS.contains("shouldContinuePolling(snapshot)"));
        assert!(PC_APP_TASK_SNAPSHOTS_JS
            .contains("localJournalSeq(snapshot) > localJournalSeq(previous)"));
    }
}
