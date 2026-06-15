//! User-provided AI API key encryption and BYOK feature gates.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

const KEY_PREFIX: &str = "v1";

pub(crate) fn user_byok_api_enabled() -> bool {
    env_bool("AI_USER_BYOK_API_ENABLED", true)
}

pub(crate) fn encrypt_api_key(plain: &str) -> Result<String> {
    let secret = encryption_secret()
        .ok_or_else(|| anyhow!("保存用户 API Key 需要配置 SECRET_KEY 或 USER_API_KEY_SECRET"))?;
    let cipher = cipher_from_secret(&secret)?;
    let nonce_uuid = uuid::Uuid::new_v4();
    let nonce_bytes = &nonce_uuid.as_bytes()[..12];
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(nonce_bytes), plain.as_bytes())
        .map_err(|_| anyhow!("用户 API Key 加密失败"))?;
    Ok(format!(
        "{}:{}:{}",
        KEY_PREFIX,
        URL_SAFE_NO_PAD.encode(nonce_bytes),
        URL_SAFE_NO_PAD.encode(ciphertext)
    ))
}

pub(crate) fn decrypt_api_key(encoded: &str) -> Result<String> {
    let (version, rest) = encoded
        .split_once(':')
        .ok_or_else(|| anyhow!("用户 API Key 密文格式无效"))?;
    if version != KEY_PREFIX {
        return Err(anyhow!("不支持的用户 API Key 密文版本: {}", version));
    }
    let (nonce_raw, ciphertext_raw) = rest
        .split_once(':')
        .ok_or_else(|| anyhow!("用户 API Key 密文缺少 nonce 或正文"))?;
    let nonce = URL_SAFE_NO_PAD
        .decode(nonce_raw)
        .context("用户 API Key nonce 解码失败")?;
    if nonce.len() != 12 {
        return Err(anyhow!("用户 API Key nonce 长度无效"));
    }
    let ciphertext = URL_SAFE_NO_PAD
        .decode(ciphertext_raw)
        .context("用户 API Key 密文解码失败")?;
    let secret = encryption_secret()
        .ok_or_else(|| anyhow!("读取用户 API Key 需要配置 SECRET_KEY 或 USER_API_KEY_SECRET"))?;
    let cipher = cipher_from_secret(&secret)?;
    let plain = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| anyhow!("用户 API Key 解密失败"))?;
    String::from_utf8(plain).context("用户 API Key 明文不是 UTF-8")
}

fn cipher_from_secret(secret: &str) -> Result<Aes256Gcm> {
    let digest = Sha256::digest(secret.as_bytes());
    Aes256Gcm::new_from_slice(&digest).map_err(|_| anyhow!("初始化用户 API Key 加密器失败"))
}

fn encryption_secret() -> Option<String> {
    read_secret("USER_API_KEY_SECRET").or_else(|| read_secret("SECRET_KEY"))
}

fn read_secret(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}
