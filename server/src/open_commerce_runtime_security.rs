//! Endpoint and secret resolution guards for merchant runtime calls.

use anyhow::{anyhow, bail, Result};

pub(crate) fn validate_endpoint_base_url(value: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(value.trim()).map_err(|_| anyhow!("商户运行地址无效"))?;
    if url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("商户运行地址不能包含账号、查询参数或片段");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("商户运行地址缺少主机"))?
        .to_ascii_lowercase();
    let local_test = cfg!(test) && matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1");
    if url.scheme() != "https" && !local_test {
        bail!("商户运行地址必须使用 HTTPS");
    }
    if !local_test && !allowed_hosts().iter().any(|allowed| allowed == &host) {
        bail!("商户运行主机未加入 OPEN_COMMERCE_RUNTIME_ALLOWED_HOSTS 白名单");
    }
    let normalized_path = url.path().trim_end_matches('/').to_string();
    url.set_path(&normalized_path);
    Ok(url.to_string().trim_end_matches('/').to_string())
}

pub(crate) fn resolve_runtime_secret(credential_ref: &str) -> Result<String> {
    let secret = std::env::var(credential_ref)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| value.len() >= 32)
        .ok_or_else(|| anyhow!("商户运行密钥引用不可用或长度不足"))?;
    Ok(secret)
}

fn allowed_hosts() -> Vec<String> {
    std::env::var("OPEN_COMMERCE_RUNTIME_ALLOWED_HOSTS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_validation_is_fail_closed() {
        assert!(validate_endpoint_base_url("http://example.com").is_err());
        assert!(validate_endpoint_base_url("http://127.0.0.1:3000").is_ok());
        assert!(validate_endpoint_base_url("https://user:pass@example.com").is_err());
    }
}
