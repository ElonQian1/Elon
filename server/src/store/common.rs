use anyhow::{Result, anyhow};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(super) fn safe_external_id(value: &str, fallback: &str) -> String {
    let safe = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect::<String>();
    if safe.is_empty() {
        fallback.into()
    } else {
        safe
    }
}

pub(super) fn normalize_account(account: &str) -> Result<String> {
    let account = account.trim().to_ascii_lowercase();
    if account.len() < 3 {
        return Err(anyhow!("账号至少需要 3 个字符"));
    }
    Ok(account)
}

pub(super) fn validate_password(password: &str) -> Result<()> {
    if password.chars().count() < 6 {
        return Err(anyhow!("密码至少需要 6 个字符"));
    }
    Ok(())
}

pub(super) fn account_columns(account: &str) -> (Option<String>, Option<String>) {
    if account.contains('@') {
        (None, Some(account.to_string()))
    } else {
        (Some(account.to_string()), None)
    }
}

pub(super) fn clean_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub(super) fn now() -> String {
    Utc::now().to_rfc3339()
}

pub(super) fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

pub(super) fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub(super) fn hash_password(password: &str) -> String {
    let salt = Uuid::new_v4().simple().to_string();
    let digest = password_digest(&salt, password);
    format!("sha256${}${}", salt, digest)
}

pub(super) fn verify_password(password: &str, stored: &str) -> bool {
    let mut parts = stored.split('$');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("sha256"), Some(salt), Some(expected), None) => {
            password_digest(salt, password) == expected
        }
        _ => false,
    }
}

fn password_digest(salt: &str, password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}
