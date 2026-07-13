use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const MAX_MANAGED_MARKER_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManagedCodexHomeMarkerIdentity {
    slot_id: String,
    lease_id: Option<String>,
    lease_expires_at: Option<String>,
    sha256: [u8; 32],
}

impl ManagedCodexHomeMarkerIdentity {
    pub(crate) fn lease_id(&self) -> Option<&str> {
        self.lease_id.as_deref()
    }

    pub(crate) fn lease_expires_at(&self) -> Option<&str> {
        self.lease_expires_at.as_deref()
    }
}

pub(crate) fn current_valid_codex_home_env() -> Option<String> {
    let value = std::env::var("CODEX_HOME").ok()?.trim().to_string();
    if value.is_empty() {
        return None;
    }
    let home = PathBuf::from(&value);
    if !home.exists() {
        return None;
    }
    if codex_home_path_is_managed(&home) {
        if managed_codex_home_marker_identity(&home).is_err() {
            // Keep the invalid managed selection visible to task admission so
            // it fails closed instead of silently falling back to ~/.codex.
            return None;
        }
    }
    Some(value)
}

pub(crate) fn managed_codex_home_marker_identity(
    home: &Path,
) -> Result<ManagedCodexHomeMarkerIdentity> {
    let path = home.join("elon-codex-vault-slot.json");
    let bytes = std::fs::read(&path)
        .with_context(|| format!("无法读取托管 CODEX_HOME marker：{}", path.display()))?;
    if bytes.is_empty() || bytes.len() > MAX_MANAGED_MARKER_BYTES {
        bail!("托管 CODEX_HOME marker 为空或过大：{}", path.display());
    }
    let value: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("托管 CODEX_HOME marker 已损坏：{}", path.display()))?;
    let slot_id = marker_required_string(&value, "slot_id")?;
    if slot_id.chars().count() > 200 || slot_id.chars().any(char::is_control) {
        bail!("托管 CODEX_HOME marker 的 slot_id 无效");
    }
    let lease_id = marker_optional_string(&value, "lease_id")?;
    let lease_expires_at = marker_optional_string(&value, "lease_expires_at")?;
    let shared = slot_id.to_ascii_lowercase().starts_with("shared-") || lease_id.is_some();
    if shared {
        let _ = lease_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("共享 CODEX_HOME marker 缺少 lease_id"))?;
        let expiry = lease_expires_at
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("共享 CODEX_HOME marker 缺少 lease_expires_at"))?;
        require_future_expiry(expiry)?;
    } else if lease_expires_at.is_some() {
        bail!("托管 CODEX_HOME marker 的 lease_expires_at 缺少对应 lease_id");
    }
    let sha256: [u8; 32] = Sha256::digest(&bytes).into();
    Ok(ManagedCodexHomeMarkerIdentity {
        slot_id,
        lease_id,
        lease_expires_at,
        sha256,
    })
}

pub(crate) fn codex_home_path_is_managed(path: &Path) -> bool {
    let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = data_base_dir().join("Elon").join("codex-vault");
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    full.starts_with(root)
}

fn marker_required_string(value: &Value, field: &str) -> Result<String> {
    marker_optional_string(value, field)?
        .ok_or_else(|| anyhow::anyhow!("托管 CODEX_HOME marker 缺少 {field}"))
}

fn marker_optional_string(value: &Value, field: &str) -> Result<Option<String>> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => {
            let clean = raw.trim();
            if clean.is_empty() {
                bail!("托管 CODEX_HOME marker 的 {field} 不能为空");
            }
            if clean.chars().count() > 500 || clean.chars().any(char::is_control) {
                bail!("托管 CODEX_HOME marker 的 {field} 无效");
            }
            Ok(Some(clean.to_string()))
        }
        Some(_) => bail!("托管 CODEX_HOME marker 的 {field} 类型无效"),
    }
}

fn require_future_expiry(raw: &str) -> Result<()> {
    let expiry = DateTime::parse_from_rfc3339(raw)
        .with_context(|| "共享 CODEX_HOME marker 的 lease_expires_at 无效")?
        .with_timezone(&Utc);
    if expiry <= Utc::now() {
        bail!("共享 CODEX_HOME 租约已过期");
    }
    Ok(())
}

fn data_base_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
    } else {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".local").join("share"))
            })
            .unwrap_or_else(std::env::temp_dir)
    }
}
