use anyhow::Result;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Deserialize)]
pub(crate) struct VersionInfo {
    #[serde(default, rename = "gitSha")]
    pub(crate) git_sha: String,
    #[serde(default, rename = "downloadUrl")]
    pub(crate) download_url: String,
    #[serde(default, rename = "windowsClientDownloadUrl")]
    pub(crate) windows_client_download_url: String,
    #[serde(default, rename = "sha256")]
    pub(crate) download_sha256: String,
    #[serde(default, rename = "fileSha256")]
    pub(crate) file_sha256: String,
    #[serde(default, rename = "windowsClientSha256")]
    pub(crate) windows_client_sha256: String,
}

pub(crate) fn preferred_sha256<'a>(primary: &'a str, fallback: &'a str) -> &'a str {
    if !primary.trim().is_empty() {
        primary
    } else {
        fallback
    }
}

pub(crate) fn verify_optional_sha256(bytes: &[u8], expected: &str, label: &str) -> Result<()> {
    let expected = expected.trim().to_ascii_lowercase();
    if expected.is_empty() {
        return Ok(());
    }
    if expected.len() != 64 || !expected.chars().all(|ch| ch.is_ascii_hexdigit()) {
        anyhow::bail!("{label}版本清单中的 SHA256 不合法");
    }
    let actual = hex::encode(Sha256::digest(bytes));
    if actual != expected {
        anyhow::bail!("{label}SHA256 校验失败: expected {expected}, actual {actual}");
    }
    Ok(())
}

pub(crate) fn read_local_git_sha(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value
        .get("gitSha")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use sha2::Digest;

    #[test]
    fn update_download_sha256_verification_rejects_mismatch() {
        let bytes = b"official package";
        let expected = hex::encode(sha2::Sha256::digest(bytes));

        assert!(super::verify_optional_sha256(bytes, &expected, "完整客户端包").is_ok());
        assert!(super::verify_optional_sha256(bytes, "", "完整客户端包").is_ok());
        assert!(super::verify_optional_sha256(bytes, &"0".repeat(64), "完整客户端包").is_err());
        assert!(super::verify_optional_sha256(bytes, "not-a-sha", "完整客户端包").is_err());
    }
}
