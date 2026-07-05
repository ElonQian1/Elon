// server/src/node_agent_admin_open.rs

#[cfg(windows)]
use std::time::Duration;

#[cfg(windows)]
const CLOUD_OPEN_PROBE_TIMEOUT: Duration = Duration::from_millis(2200);
#[cfg(any(windows, test))]
const DEFAULT_CLOUD_WS_URL: &str = "ws://43.139.149.158:8080/agent/ws";

pub fn admin_port_from_env() -> u16 {
    std::env::var("NODE_ADMIN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7799)
}

/// 默认打开用户熟悉的云端 PC 工作台；云端不可达时回退到本地 PC 工作台。
/// 旧本地管理页可通过 NODE_OPEN_LOCAL=1 切换，云端直开可通过 NODE_OPEN_CLOUD=1 强制。
#[cfg(windows)]
fn admin_url(port: u16) -> String {
    if std::env::var("NODE_OPEN_LOCAL")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return format!("http://127.0.0.1:{port}/local-admin");
    }
    let cloud_url = cloud_pc_url(port);
    if std::env::var("NODE_OPEN_CLOUD")
        .map(|v| v == "1")
        .unwrap_or(false)
        || cloud_workbench_reachable()
    {
        return cloud_url;
    }
    format!("http://127.0.0.1:{port}/pc")
}

#[cfg(windows)]
fn cloud_pc_url(port: u16) -> String {
    format!(
        "{}/pc?node_admin={}",
        cloud_http_base().trim_end_matches('/'),
        encode_query_component(&format!("http://127.0.0.1:{port}/"))
    )
}

#[cfg(windows)]
fn cloud_http_base() -> String {
    let cloud_base =
        std::env::var("NODE_CLOUD_URL").unwrap_or_else(|_| DEFAULT_CLOUD_WS_URL.to_string());
    cloud_http_base_from(&cloud_base)
}

#[cfg(any(windows, test))]
fn cloud_http_base_from(cloud_base: &str) -> String {
    let parsed =
        reqwest::Url::parse(cloud_base).or_else(|_| reqwest::Url::parse(DEFAULT_CLOUD_WS_URL));
    let Ok(url) = parsed else {
        return "http://43.139.149.158:8080".to_string();
    };
    let scheme = match url.scheme() {
        "wss" | "https" => "https",
        "ws" | "http" => "http",
        _ => "http",
    };
    let Some(host) = url.host_str() else {
        return "http://43.139.149.158:8080".to_string();
    };
    let host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    match url.port() {
        Some(port) => format!("{scheme}://{host}:{port}"),
        None => format!("{scheme}://{host}"),
    }
}

#[cfg(windows)]
fn cloud_workbench_reachable() -> bool {
    let health_url = format!("{}/health", cloud_http_base().trim_end_matches('/'));
    let client = match reqwest::blocking::Client::builder()
        .connect_timeout(CLOUD_OPEN_PROBE_TIMEOUT)
        .timeout(CLOUD_OPEN_PROBE_TIMEOUT)
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };
    client
        .get(health_url)
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .send()
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(windows)]
pub fn maybe_open_admin_page(port: u16) {
    if !auto_open_enabled() {
        return;
    }
    std::thread::spawn(move || {
        wait_for_admin_port(port);
        let url = admin_url(port);
        if let Err(err) = crate::node_client_launcher::command::open_url(&url) {
            tracing::warn!(%url, error = %err, "无法自动打开 node-agent 管理页");
        }
    });
}

#[cfg(not(windows))]
pub fn maybe_open_admin_page(_port: u16) {}

#[cfg(windows)]
fn wait_for_admin_port(port: u16) {
    for _ in 0..40 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[cfg(windows)]
fn auto_open_enabled() -> bool {
    auto_open_enabled_from(std::env::var("NODE_AUTO_OPEN_ADMIN").ok().as_deref())
}

#[cfg(any(windows, test))]
fn auto_open_enabled_from(value: Option<&str>) -> bool {
    value
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    #[test]
    fn cloud_http_base_accepts_ws_and_http_urls() {
        assert_eq!(
            super::cloud_http_base_from("ws://cloud.test:8080/agent/ws"),
            "http://cloud.test:8080"
        );
        assert_eq!(
            super::cloud_http_base_from("wss://cloud.test/agent/ws"),
            "https://cloud.test"
        );
        assert_eq!(
            super::cloud_http_base_from("https://cloud.test/agent/ws"),
            "https://cloud.test"
        );
    }

    #[test]
    fn auto_open_is_opt_in() {
        assert!(!super::auto_open_enabled_from(None));
        assert!(super::auto_open_enabled_from(Some("true")));
        assert!(super::auto_open_enabled_from(Some("1")));
        assert!(!super::auto_open_enabled_from(Some("0")));
        assert!(!super::auto_open_enabled_from(Some("false")));
    }
}
