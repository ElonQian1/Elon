//! Durable author-owned outbound publishing. No public read grants are created here.
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
mod account;
mod actions;
mod media;
pub(crate) mod migration;
mod model;
mod provider;
mod queue;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_media;
mod worker;
pub(crate) use model::*;
pub(crate) use queue::Enqueue;
pub(crate) use worker::spawn;

fn epoch() -> i64 {
    chrono::Utc::now().timestamp()
}
fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
pub(crate) const CREATOR_URL: &str = "https://www.binance.com/square/creator-center/home";
pub(crate) fn post_link(id: &str) -> Option<String> {
    (!id.is_empty() && id.len() <= 64 && id.bytes().all(|c| c.is_ascii_digit()))
        .then(|| format!("https://www.binance.com/en/square/post/{id}"))
}

fn secret_envelope(owner: &str, key: &str) -> Result<String> {
    crate::user_agent_secrets::encrypt_api_key(
        &json!({"owner":owner,"channel":"binance_square","key":key}).to_string(),
    )
    .map_err(|_| fail(503, "服务器密钥存储尚未配置，请联系管理员"))
}
fn reveal(owner: &str, encrypted: &str) -> Result<String> {
    let plain = crate::user_agent_secrets::decrypt_api_key(encrypted)
        .map_err(|_| fail(503, "发帖凭证暂不可读取，请重新绑定或联系管理员"))?;
    let value: Value = serde_json::from_str(&plain)?;
    if value["owner"] != owner || value["channel"] != "binance_square" {
        return Err(fail(403, "凭证归属校验失败"));
    }
    value["key"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| fail(503, "凭证格式无效"))
}
