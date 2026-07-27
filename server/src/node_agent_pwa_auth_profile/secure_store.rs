use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const STORE_SCHEMA: &str = "elon.pwa.remembered-auth.v1";
const STORE_FILE: &str = "pwa-remembered-auth.v1.json";
const MAX_STORE_BYTES: u64 = 64 * 1024;
const MAX_TOKEN_BYTES: usize = 8_192 - "Bearer ".len();
const MAX_LABEL_CHARS: usize = 120;

#[derive(Debug)]
pub(super) struct RememberedCredential {
    pub(super) token: String,
    pub(super) account_label: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProtectedRecord {
    schema: String,
    protection: String,
    account_label: String,
    protected_base64: String,
    updated_at: String,
}

pub(super) fn save(token: &str, account_label: Option<&str>) -> Result<String> {
    validate_token(token)?;
    let account_label = validate_label(account_label)?;
    let protected = protect_for_current_user(token.as_bytes())?;
    let record = ProtectedRecord {
        schema: STORE_SCHEMA.to_string(),
        protection: protection_name().to_string(),
        account_label: account_label.clone(),
        protected_base64: base64::engine::general_purpose::STANDARD.encode(protected),
        updated_at: Utc::now().to_rfc3339(),
    };
    let path = store_path()?;
    crate::node_agent_atomic_file::write(&path, &serde_json::to_vec_pretty(&record)?)
        .with_context(|| format!("无法保存 Windows 保护的 PWA 登录态 {}", path.display()))?;
    Ok(account_label)
}

pub(super) fn load() -> Result<Option<RememberedCredential>> {
    let path = store_path()?;
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_STORE_BYTES {
        bail!("PWA_REMEMBERED_AUTH_INVALID: 受保护登录态文件大小或类型无效");
    }
    let record: ProtectedRecord =
        serde_json::from_slice(&fs::read(&path)?).context("PWA_REMEMBERED_AUTH_INVALID")?;
    if record.schema != STORE_SCHEMA || record.protection != protection_name() {
        bail!("PWA_REMEMBERED_AUTH_INVALID: 受保护登录态版本或保护方式不匹配");
    }
    let account_label = validate_label(Some(&record.account_label))?;
    let protected = base64::engine::general_purpose::STANDARD
        .decode(record.protected_base64)
        .context("PWA_REMEMBERED_AUTH_INVALID")?;
    let plaintext = unprotect_for_current_user(&protected)?;
    let token = String::from_utf8(plaintext).context("PWA_REMEMBERED_AUTH_INVALID")?;
    validate_token(&token)?;
    Ok(Some(RememberedCredential {
        token,
        account_label,
    }))
}

pub(super) fn forget() -> Result<bool> {
    let path = store_path()?;
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn account_label() -> Result<Option<String>> {
    Ok(load()?.map(|credential| credential.account_label))
}

fn store_path() -> Result<PathBuf> {
    let state = crate::node_agent_config::state_path();
    let root = state
        .parent()
        .ok_or_else(|| anyhow!("节点状态目录不存在"))?;
    fs::create_dir_all(root)?;
    Ok(root.join(STORE_FILE))
}

fn validate_token(token: &str) -> Result<()> {
    if token.is_empty() || token.len() > MAX_TOKEN_BYTES || token.contains(['\r', '\n', '\0']) {
        bail!("PWA_REMEMBERED_AUTH_INVALID: 登录态为空、过长或包含控制字符");
    }
    Ok(())
}

fn validate_label(value: Option<&str>) -> Result<String> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("PC owner");
    if value.chars().count() > MAX_LABEL_CHARS || value.contains(['\r', '\n', '\0']) {
        bail!("PWA_REMEMBERED_AUTH_INVALID: 账号标识为空、过长或包含控制字符");
    }
    Ok(value.to_string())
}

#[cfg(windows)]
fn protection_name() -> &'static str {
    "WINDOWS_DPAPI_CURRENT_USER"
}

#[cfg(not(windows))]
fn protection_name() -> &'static str {
    "UNAVAILABLE"
}

#[cfg(windows)]
fn protect_for_current_user(plaintext: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = blob(plaintext)?;
    let entropy_bytes = b"elon.pwa.remembered-auth.v1";
    let mut entropy = blob(entropy_bytes)?;
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &mut input,
            std::ptr::null(),
            &mut entropy,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        bail!(
            "PWA_REMEMBERED_AUTH_PROTECT_FAILED: {}",
            std::io::Error::last_os_error()
        );
    }
    take_local_blob(output)
}

#[cfg(windows)]
fn unprotect_for_current_user(protected: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = blob(protected)?;
    let entropy_bytes = b"elon.pwa.remembered-auth.v1";
    let mut entropy = blob(entropy_bytes)?;
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let mut description = std::ptr::null_mut();
    let ok = unsafe {
        CryptUnprotectData(
            &mut input,
            &mut description,
            &mut entropy,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if !description.is_null() {
        unsafe {
            LocalFree(description.cast());
        }
    }
    if ok == 0 {
        bail!(
            "PWA_REMEMBERED_AUTH_UNPROTECT_FAILED: {}",
            std::io::Error::last_os_error()
        );
    }
    take_local_blob(output)
}

#[cfg(windows)]
fn blob(bytes: &[u8]) -> Result<windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB> {
    let len = u32::try_from(bytes.len()).context("PWA 登录态超过 Windows DPAPI 输入上限")?;
    Ok(
        windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB {
            cbData: len,
            pbData: bytes.as_ptr().cast_mut(),
        },
    )
}

#[cfg(windows)]
fn take_local_blob(
    output: windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB,
) -> Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;

    if output.pbData.is_null() || output.cbData == 0 {
        bail!("Windows DPAPI 返回空登录态");
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData.cast());
    }
    Ok(bytes)
}

#[cfg(not(windows))]
fn protect_for_current_user(_plaintext: &[u8]) -> Result<Vec<u8>> {
    bail!("PWA_REMEMBERED_AUTH_UNAVAILABLE: 长期登录态仅支持 Windows 节点")
}

#[cfg(not(windows))]
fn unprotect_for_current_user(_protected: &[u8]) -> Result<Vec<u8>> {
    bail!("PWA_REMEMBERED_AUTH_UNAVAILABLE: 长期登录态仅支持 Windows 节点")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_record_never_serializes_plaintext() {
        let secret = "test-secret-that-must-not-appear";
        let record = ProtectedRecord {
            schema: STORE_SCHEMA.into(),
            protection: protection_name().into(),
            account_label: "夜云".into(),
            protected_base64: base64::engine::general_purpose::STANDARD.encode([1, 2, 3]),
            updated_at: "now".into(),
        };
        let bytes = serde_json::to_vec(&record).unwrap();
        assert!(!String::from_utf8(bytes).unwrap().contains(secret));
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_round_trip_is_bound_to_current_windows_user() {
        let secret = b"fixture-token";
        let protected = protect_for_current_user(secret).unwrap();
        assert_ne!(protected, secret);
        assert_eq!(unprotect_for_current_user(&protected).unwrap(), secret);
    }
}
