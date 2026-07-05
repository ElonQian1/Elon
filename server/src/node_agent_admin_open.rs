// server/src/node_agent_admin_open.rs

#[cfg(windows)]
use std::time::Duration;

pub fn admin_port_from_env() -> u16 {
    std::env::var("NODE_ADMIN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7799)
}

/// 打开本地 PC 工作台。工作台资源由本机节点提供，云端 API 作为数据源。
/// 旧本地管理页可通过 NODE_OPEN_LOCAL=1 切换，云端直开可通过 NODE_OPEN_CLOUD=1 切换。
#[cfg(windows)]
fn admin_url(port: u16) -> String {
    if std::env::var("NODE_OPEN_LOCAL")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return format!("http://127.0.0.1:{port}/local-admin");
    }
    if !std::env::var("NODE_OPEN_CLOUD")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return format!("http://127.0.0.1:{port}/pc");
    }
    let cloud_base = std::env::var("NODE_CLOUD_URL")
        .unwrap_or_else(|_| "ws://43.139.149.158:8080/agent/ws".to_string());
    let http_base = if let Some(rest) = cloud_base.strip_prefix("wss://") {
        format!(
            "https://{}",
            rest.split('/').next().unwrap_or("43.139.149.158:8080")
        )
    } else if let Some(rest) = cloud_base.strip_prefix("ws://") {
        format!(
            "http://{}",
            rest.split('/').next().unwrap_or("43.139.149.158:8080")
        )
    } else {
        "http://43.139.149.158:8080".to_string()
    };
    format!("{http_base}/pc")
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
    fn auto_open_is_opt_in() {
        assert!(!super::auto_open_enabled_from(None));
        assert!(super::auto_open_enabled_from(Some("true")));
        assert!(super::auto_open_enabled_from(Some("1")));
        assert!(!super::auto_open_enabled_from(Some("0")));
        assert!(!super::auto_open_enabled_from(Some("false")));
    }
}
