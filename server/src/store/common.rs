use anyhow::{anyhow, Result};
use chrono::Utc;
use ring::pbkdf2::{self, PBKDF2_HMAC_SHA256};
use sha2::{Digest, Sha256};
use std::num::NonZeroU32;
use uuid::Uuid;

#[cfg(not(test))]
const PASSWORD_PBKDF2_ITERATIONS: u32 = 310_000;
#[cfg(test)]
const PASSWORD_PBKDF2_ITERATIONS: u32 = 1_000;
const PASSWORD_HASH_BYTES: usize = 32;

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
    let salt = *Uuid::new_v4().as_bytes();
    let mut digest = [0_u8; PASSWORD_HASH_BYTES];
    pbkdf2::derive(
        PBKDF2_HMAC_SHA256,
        NonZeroU32::new(PASSWORD_PBKDF2_ITERATIONS).expect("password iterations are non-zero"),
        &salt,
        password.as_bytes(),
        &mut digest,
    );
    format!(
        "pbkdf2_sha256${}${}${}",
        PASSWORD_PBKDF2_ITERATIONS,
        hex::encode(salt),
        hex::encode(digest)
    )
}

pub(super) fn verify_password(password: &str, stored: &str) -> bool {
    let mut parts = stored.split('$');
    match (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) {
        (Some("pbkdf2_sha256"), Some(iterations), Some(salt), Some(expected), None) => {
            let Some(iterations) = iterations.parse::<u32>().ok().and_then(NonZeroU32::new) else {
                return false;
            };
            let (Ok(salt), Ok(expected)) = (hex::decode(salt), hex::decode(expected)) else {
                return false;
            };
            expected.len() == PASSWORD_HASH_BYTES
                && pbkdf2::verify(
                    PBKDF2_HMAC_SHA256,
                    iterations,
                    &salt,
                    password.as_bytes(),
                    &expected,
                )
                .is_ok()
        }
        (Some("sha256"), Some(salt), Some(expected), None, None) => {
            password_digest(salt, password) == expected
        }
        _ => false,
    }
}

pub(super) fn password_needs_rehash(stored: &str) -> bool {
    let mut parts = stored.split('$');
    match (parts.next(), parts.next()) {
        (Some("pbkdf2_sha256"), Some(iterations)) => iterations
            .parse::<u32>()
            .map_or(true, |value| value < PASSWORD_PBKDF2_ITERATIONS),
        _ => true,
    }
}

fn password_digest(salt: &str, password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_password_hashes_are_versioned_and_verifiable() {
        let stored = hash_password("correct horse battery staple");
        assert!(stored.starts_with("pbkdf2_sha256$"));
        assert!(verify_password("correct horse battery staple", &stored));
        assert!(!verify_password("wrong", &stored));
        assert!(!password_needs_rehash(&stored));
    }

    #[test]
    fn legacy_sha256_hashes_remain_readable_but_require_upgrade() {
        let salt = "legacy-salt";
        let stored = format!("sha256${salt}${}", password_digest(salt, "secret1"));
        assert!(verify_password("secret1", &stored));
        assert!(password_needs_rehash(&stored));
    }
}
