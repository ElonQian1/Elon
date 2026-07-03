use std::time::Duration;

use serde_json::{json, Value};

pub(crate) fn direct_cloud_client(timeout: Duration) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(timeout)
        .build()
}

pub(crate) fn direct_cloud_client_or_default(timeout: Duration) -> reqwest::Client {
    direct_cloud_client(timeout).unwrap_or_default()
}

pub(crate) fn status_payload(cloud_url: &str, cloud_http_url: &str) -> Value {
    json!({
        "cloudWsMode": "direct_websocket",
        "cloudHttpMode": "direct_reqwest_no_proxy",
        "proxyDefault": "off_for_elon_cloud",
        "userProxyOptIn": "only via explicit custom network configuration outside the default PC node path",
        "cloudHostsNoProxy": cloud_hosts(cloud_url, cloud_http_url),
        "noProxyEnv": std::env::var("NO_PROXY").ok(),
    })
}

fn cloud_hosts(cloud_url: &str, cloud_http_url: &str) -> Vec<String> {
    [cloud_url, cloud_http_url]
        .into_iter()
        .filter_map(host_from_url)
        .fold(Vec::new(), |mut hosts, host| {
            if !hosts.iter().any(|item| item.eq_ignore_ascii_case(&host)) {
                hosts.push(host);
            }
            hosts
        })
}

fn host_from_url(raw: &str) -> Option<String> {
    reqwest::Url::parse(raw.trim())
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
}
