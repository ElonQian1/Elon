use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

const PC_DIST_DIR_NAME: &str = "pc-next-dist";
const OLD_ADMIN_HTML: &str = include_str!("node_agent_admin.html");

pub(crate) fn routes() -> Router<Arc<crate::NodeRuntime>> {
    let dist = local_pc_dist_dir();
    let asset_cache = SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    let assets = tower::ServiceBuilder::new()
        .layer(asset_cache)
        .service(ServeDir::new(dist.join("assets")));

    Router::new()
        .route("/", get(pc_root_redirect))
        .route("/local-admin", get(local_admin_index))
        .nest_service("/pc/assets", assets.clone())
        .route("/pc/pc-workbench-sw.js", get(pc_service_worker))
        .route("/pc", get(pc_spa_index))
        .route("/pc/*path", get(pc_spa_index))
        .nest_service("/pc-next/assets", assets)
        .route("/pc-next", get(pc_spa_index))
        .route("/pc-next/*path", get(pc_spa_index))
}

async fn local_admin_index() -> Html<&'static str> {
    Html(OLD_ADMIN_HTML)
}

async fn pc_root_redirect() -> Redirect {
    Redirect::temporary("/pc")
}

async fn pc_service_worker() -> impl IntoResponse {
    let path = local_pc_dist_dir().join("pc-workbench-sw.js");
    match tokio::fs::read(&path).await {
        Ok(bytes) => (
            StatusCode::OK,
            [
                (
                    header::CONTENT_TYPE,
                    "application/javascript; charset=utf-8",
                ),
                (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "PC Service Worker missing").into_response(),
    }
}

async fn pc_spa_index(State(runtime): State<Arc<crate::NodeRuntime>>) -> impl IntoResponse {
    let index_path = local_pc_dist_dir().join("index.html");
    match tokio::fs::read_to_string(&index_path).await {
        Ok(index) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
            ],
            inject_bootstrap(&index, &runtime.cloud_http_url()),
        )
            .into_response(),
        Err(error) => {
            tracing::warn!(
                path = %index_path.display(),
                error = %error,
                "本地 PC 工作台资源缺失，回退到旧本地管理页"
            );
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                    (header::CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
                ],
                Html(OLD_ADMIN_HTML),
            )
                .into_response()
        }
    }
}

fn inject_bootstrap(index: &str, cloud_http_url: &str) -> String {
    let bootstrap = json!({
        "mode": "local",
        "cloudBaseUrl": cloud_http_url,
        "localNodeBaseUrl": "",
    });
    let script = format!(
        "<script>window.__ELON_PC_BOOTSTRAP__ = {bootstrap}; window.__ELON_PC_BOOTSTRAP__.localNodeBaseUrl = location.origin;</script>"
    );
    if let Some(pos) = index.find("</head>") {
        let mut html = String::with_capacity(index.len() + script.len() + 1);
        html.push_str(&index[..pos]);
        html.push_str(&script);
        html.push_str(&index[pos..]);
        html
    } else {
        format!("{script}{index}")
    }
}

fn local_pc_dist_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join("_internal")))
        .unwrap_or_else(|| PathBuf::from("_internal"))
        .join(PC_DIST_DIR_NAME)
}

#[cfg(test)]
mod tests {
    use super::inject_bootstrap;

    #[test]
    fn bootstrap_is_inserted_before_head_end() {
        let html = inject_bootstrap(
            "<html><head><title>x</title></head><body></body></html>",
            "http://cloud.test:8080",
        );
        assert!(html.contains("window.__ELON_PC_BOOTSTRAP__"));
        assert!(html.contains("\"mode\":\"local\""));
        assert!(html.contains("http://cloud.test:8080"));
        let bootstrap_pos = html.find("__ELON_PC_BOOTSTRAP__").unwrap();
        let head_end_pos = html.find("</head>").unwrap();
        assert!(bootstrap_pos < head_end_pos);
    }
}
