const LOCAL_NO_PROXY_HOSTS: &[&str] = &["localhost", "127.0.0.1", "::1"];

pub fn ensure_localhost_no_proxy() {
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
    for host in LOCAL_NO_PROXY_HOSTS {
        if !values.iter().any(|value| value.eq_ignore_ascii_case(host)) {
            values.push((*host).to_string());
        }
    }

    let merged = values.join(",");
    std::env::set_var("NO_PROXY", &merged);
    std::env::set_var("no_proxy", merged);
}
