//! Background business runner. Uses the main workbench profile, never exported credentials.
use std::sync::Mutex;
use tauri::{
    webview::NewWindowResponse, AppHandle, Manager, Url, Webview, WebviewUrl, WebviewWindowBuilder,
};

const LABEL: &str = "group-ai-worker";
const PATH: &str = "/pc/group-ai-worker";
const CLOUD: &str = "http://43.139.149.158:8080";
static TARGET: Mutex<Option<Url>> = Mutex::new(None);

fn origin(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|_| "invalid_worker_origin")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err("invalid_worker_origin".into());
    }
    Ok(url)
}

fn target(main: &Url, cloud: &str, node: &str) -> Result<Url, String> {
    let mut cloud = origin(cloud)?;
    // Match the already trusted workbench or the installed production capability.
    if cloud.origin().ascii_serialization() != CLOUD && cloud.origin() != main.origin() {
        return Err("worker_cloud_not_trusted".into());
    }
    let node = node_origin(node)?;
    cloud.set_path(PATH);
    cloud
        .query_pairs_mut()
        .append_pair("node_admin", node.as_str());
    Ok(cloud)
}

fn node_origin(raw: &str) -> Result<Url, String> {
    let mut node = origin(raw)?;
    if !matches!(node.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")) {
        return Err("worker_node_not_loopback".into());
    }
    if node.host_str() == Some("localhost") {
        node.set_host(Some("127.0.0.1"))
            .map_err(|_| "invalid_worker_origin")?;
    }
    Ok(node)
}

fn same_target(actual: &Url, expected: &Url) -> bool {
    if actual.origin() != expected.origin()
        || actual.path() != PATH
        || actual.fragment().is_some()
        || !actual.username().is_empty()
        || actual.password().is_some()
    {
        return false;
    }
    let actual_pairs: Vec<_> = actual.query_pairs().collect();
    let expected_pairs: Vec<_> = expected.query_pairs().collect();
    if actual_pairs.len() != 1
        || expected_pairs.len() != 1
        || actual_pairs[0].0 != "node_admin"
        || expected_pairs[0].0 != "node_admin"
    {
        return false;
    }
    match (
        node_origin(&actual_pairs[0].1),
        node_origin(&expected_pairs[0].1),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

pub(crate) fn ensure_caller(webview: &Webview) -> Result<(), String> {
    if webview.label() == "main" {
        return Ok(());
    }
    let expected = TARGET.lock().map_err(|_| "worker_state_unavailable")?;
    if webview.label() == LABEL
        && expected.as_ref().is_some_and(|url| {
            webview
                .url()
                .ok()
                .is_some_and(|actual| same_target(&actual, url))
        })
    {
        Ok(())
    } else {
        Err("group_worker_not_trusted".into())
    }
}

#[tauri::command]
pub async fn ensure_group_ai_worker(
    app: AppHandle,
    webview: Webview,
    cloud_base_url: String,
    node_base_url: String,
) -> Result<bool, String> {
    if webview.label() != "main" {
        return Err("worker_requires_main".into());
    }
    let url = target(
        &webview.url().map_err(|_| "main_url_unavailable")?,
        &cloud_base_url,
        &node_base_url,
    )?;
    let mut expected = TARGET.lock().map_err(|_| "worker_state_unavailable")?;
    if app.get_webview(LABEL).is_some() {
        // Never reload a runner that may have an in-flight dispatch.
        return if expected
            .as_ref()
            .is_some_and(|current| same_target(&url, current))
        {
            Ok(true)
        } else {
            Err("worker_origin_changed_restart_required".into())
        };
    }
    let allowed = url.clone();
    let built = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::External(url.clone()))
        .title("Group AI worker")
        .visible(false)
        .focused(false)
        .skip_taskbar(true)
        .additional_browser_args(crate::WEBVIEW2_BROWSER_ARGS)
        // No data_directory override: same WebView2 profile and same-origin elon_auth as main.
        .on_navigation(move |next| same_target(next, &allowed))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()
        .map_err(|_| "worker_creation_failed")?;
    *expected = Some(url);
    built.hide().map_err(|_| "worker_hide_failed")?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_background_route_reuses_cloud_origin_without_credentials() {
        let main = Url::parse("http://127.0.0.1:7799/pc/local-tasks").unwrap();
        let url = target(&main, CLOUD, "http://127.0.0.1:7799").unwrap();
        assert_eq!(url.path(), PATH);
        assert_eq!(url.origin().ascii_serialization(), CLOUD);
        assert_eq!(url.query_pairs().count(), 1);
    }
    #[test]
    fn rejects_foreign_origins_credentials_paths_and_remote_nodes() {
        let main = Url::parse("http://127.0.0.1:7799/pc").unwrap();
        for cloud in [
            "https://example.org",
            "http://user@43.139.149.158:8080",
            "http://43.139.149.158:8080/pc",
            "http://43.139.149.158:8080/?secret=x",
        ] {
            assert!(target(&main, cloud, "http://127.0.0.1:7799").is_err());
        }
        for node in [
            "http://example.org",
            "http://127.0.0.1:7799/token",
            "http://127.0.0.1:7799/#secret",
        ] {
            assert!(target(&main, CLOUD, node).is_err());
        }
    }

    #[test]
    fn worker_target_compares_decoded_loopback_identity_without_widening_scope() {
        let main = Url::parse("http://127.0.0.1:7799/pc/local-tasks").unwrap();
        let expected = target(&main, CLOUD, "http://127.0.0.1:7799").unwrap();
        let equivalent =
            Url::parse(&format!("{CLOUD}{PATH}?node_admin=http://localhost:7799/")).unwrap();
        assert!(same_target(&equivalent, &expected));
        assert_eq!(
            target(&main, CLOUD, "http://localhost:7799").unwrap(),
            expected
        );
        for suffix in [
            "?node_admin=http://localhost:7800/",
            "?node_admin=http://example.org/",
            "?node_admin=http://localhost:7799/&extra=1",
            "?node_admin=http://localhost:7799/#fragment",
            "/other?node_admin=http://localhost:7799/",
        ] {
            assert!(!same_target(
                &Url::parse(&format!("{CLOUD}{PATH}{suffix}")).unwrap(),
                &expected
            ));
        }
    }
}
