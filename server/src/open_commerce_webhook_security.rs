use anyhow::{anyhow, bail, Result};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub(crate) fn validate_webhook_callback_url(value: &str) -> Result<String> {
    let value = value.trim();
    if value.len() > 2048 {
        bail!("Webhook 回调地址超过 2048 字符限制");
    }
    let mut url = reqwest::Url::parse(value).map_err(|_| anyhow!("Webhook 回调地址无效"))?;
    if url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("Webhook 回调地址不能包含账号、查询参数或片段");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("Webhook 回调地址缺少主机"))?
        .to_ascii_lowercase();
    let local_test = cfg!(test) && matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1");
    if url.scheme() != "https" && !local_test {
        bail!("Webhook 回调地址必须使用 HTTPS");
    }
    if !local_test && !allowed_hosts().iter().any(|allowed| allowed == &host) {
        bail!("Webhook 主机未加入 OPEN_COMMERCE_WEBHOOK_ALLOWED_HOSTS 白名单");
    }
    let normalized_path = if url.path().is_empty() {
        "/".to_string()
    } else {
        url.path().to_string()
    };
    url.set_path(&normalized_path);
    Ok(url.to_string())
}

pub(crate) fn webhook_master_key_id() -> Result<String> {
    let master = webhook_master_secret()?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(master))))
}

pub(crate) fn derive_webhook_signing_secret(
    subscription_id: &str,
    signing_secret_version: i64,
) -> Result<String> {
    if signing_secret_version < 1 {
        bail!("Webhook 签名密钥版本无效");
    }
    let master = webhook_master_secret()?;
    let mut mac =
        HmacSha256::new_from_slice(&master).map_err(|_| anyhow!("Webhook 主密钥不可用于 HMAC"))?;
    let message = if signing_secret_version == 1 {
        format!("subscription:{}", subscription_id.trim())
    } else {
        format!(
            "subscription:{}:version:{}",
            subscription_id.trim(),
            signing_secret_version
        )
    };
    mac.update(message.as_bytes());
    Ok(format!("whsec_{}", hex_digest(mac.finalize().into_bytes())))
}

pub(crate) fn sign_webhook(secret: &str, timestamp: &str, body: &[u8]) -> Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| anyhow!("Webhook 签名密钥不可用于 HMAC"))?;
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    Ok(hex_digest(mac.finalize().into_bytes()))
}

fn webhook_master_secret() -> Result<Vec<u8>> {
    std::env::var("OPEN_COMMERCE_WEBHOOK_MASTER_SECRET")
        .ok()
        .map(|value| value.trim().as_bytes().to_vec())
        .filter(|value| value.len() >= 32)
        .ok_or_else(|| anyhow!("OPEN_COMMERCE_WEBHOOK_MASTER_SECRET 未配置或长度不足 32 字节"))
}

fn allowed_hosts() -> Vec<String> {
    std::env::var("OPEN_COMMERCE_WEBHOOK_ALLOWED_HOSTS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
