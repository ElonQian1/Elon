const LOCAL_NO_PROXY_HOSTS: &[&str] = &["localhost", "127.0.0.1", "::1"];

pub fn ensure_localhost_no_proxy() {
    ensure_hosts_no_proxy(LOCAL_NO_PROXY_HOSTS);
}

pub fn ensure_cloud_no_proxy(cloud_url: &str, cloud_http_url: &str) {
    let hosts = [host_from_url(cloud_url), host_from_url(cloud_http_url)];
    let values = hosts
        .iter()
        .filter_map(|host| host.as_deref())
        .collect::<Vec<_>>();
    ensure_hosts_no_proxy(&values);
}

fn ensure_hosts_no_proxy(hosts: &[&str]) {
    let existing = std::env::var("NO_PROXY")
        .or_else(|_| std::env::var("no_proxy"))
        .unwrap_or_default();
    if existing.split(',').any(|part| part.trim() == "*") {
        return;
    }

    let mut values: Vec<String> = existing
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    for host in hosts {
        let host = host.trim();
        if !host.is_empty() && !values.iter().any(|value| value.eq_ignore_ascii_case(host)) {
            values.push(host.to_string());
        }
    }

    let merged = values.join(",");
    std::env::set_var("NO_PROXY", &merged);
    std::env::set_var("no_proxy", merged);
}

fn host_from_url(raw: &str) -> Option<String> {
    reqwest::Url::parse(raw.trim())
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
}
