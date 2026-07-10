use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::types::{AndroidDeviceIdentity, AndroidDeviceProfile};

const PROFILE_SCHEMA_VERSION: u32 = 1;
static PROFILE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileFile {
    schema_version: u32,
    profiles: Vec<AndroidDeviceProfile>,
}

fn profile_lock() -> &'static Mutex<()> {
    PROFILE_LOCK.get_or_init(|| Mutex::new(()))
}

fn profiles_path() -> PathBuf {
    crate::state_path().with_file_name("android-inspector-devices.json")
}

fn read_file(path: &Path) -> Result<ProfileFile> {
    if !path.exists() {
        return Ok(ProfileFile {
            schema_version: PROFILE_SCHEMA_VERSION,
            profiles: Vec::new(),
        });
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("读取 Android 设备档案失败: {:?}", path))?;
    let mut file: ProfileFile = serde_json::from_str(&text)
        .with_context(|| format!("解析 Android 设备档案失败: {:?}", path))?;
    if file.schema_version == 0 {
        file.schema_version = PROFILE_SCHEMA_VERSION;
    }
    Ok(file)
}

fn write_file(path: &Path, file: &ProfileFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建 Android 设备档案目录失败: {:?}", parent))?;
    }
    let text = serde_json::to_string_pretty(file).context("序列化 Android 设备档案失败")?;
    fs::write(path, format!("{text}\n"))
        .with_context(|| format!("保存 Android 设备档案失败: {:?}", path))
}

pub(crate) fn list_profiles() -> Result<Vec<AndroidDeviceProfile>> {
    let _guard = profile_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("Android 设备档案锁已损坏"))?;
    Ok(read_file(&profiles_path())?.profiles)
}

pub(crate) fn upsert_profile(
    identity: &AndroidDeviceIdentity,
    display_name: Option<&str>,
    endpoint: Option<&str>,
) -> Result<AndroidDeviceProfile> {
    let _guard = profile_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("Android 设备档案锁已损坏"))?;
    let path = profiles_path();
    let mut file = read_file(&path)?;
    let now = Utc::now().to_rfc3339();
    let profile = if let Some(existing) = file
        .profiles
        .iter_mut()
        .find(|profile| profile.hardware_serial == identity.hardware_serial)
    {
        existing.display_name =
            clean_display_name(display_name).unwrap_or_else(|| existing.display_name.clone());
        existing.manufacturer = identity.manufacturer.clone();
        existing.model = identity.model.clone();
        existing.android_sdk = identity.android_sdk;
        existing.android_release = identity.android_release.clone();
        if endpoint.is_some() {
            existing.last_endpoint = endpoint.map(str::to_string);
        }
        existing.last_seen_at = now;
        existing.clone()
    } else {
        let fallback_name = identity
            .model
            .clone()
            .unwrap_or_else(|| identity.hardware_serial.clone());
        let profile = AndroidDeviceProfile {
            id: format!("adp_{}", uuid::Uuid::new_v4().simple()),
            display_name: clean_display_name(display_name).unwrap_or(fallback_name),
            hardware_serial: identity.hardware_serial.clone(),
            manufacturer: identity.manufacturer.clone(),
            model: identity.model.clone(),
            android_sdk: identity.android_sdk,
            android_release: identity.android_release.clone(),
            wireless_mode: "unknown".to_string(),
            paired: false,
            last_endpoint: endpoint.map(str::to_string),
            created_at: now.clone(),
            last_seen_at: now,
        };
        file.profiles.push(profile.clone());
        profile
    };
    write_file(&path, &file)?;
    Ok(profile)
}

pub(crate) fn mark_paired(profile_id: Option<&str>) -> Result<()> {
    update_profile(profile_id, |profile| {
        profile.paired = true;
        profile.wireless_mode = "tls".to_string();
    })
}

pub(crate) fn remember_connection(
    hardware_serial: &str,
    endpoint: &str,
    wireless_mode: &str,
) -> Result<()> {
    let _guard = profile_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("Android 设备档案锁已损坏"))?;
    let path = profiles_path();
    let mut file = read_file(&path)?;
    if let Some(profile) = file
        .profiles
        .iter_mut()
        .find(|profile| profile.hardware_serial == hardware_serial)
    {
        profile.last_endpoint = Some(endpoint.to_string());
        profile.last_seen_at = Utc::now().to_rfc3339();
        profile.wireless_mode = wireless_mode.to_string();
        if wireless_mode == "tls" {
            profile.paired = true;
        }
        write_file(&path, &file)?;
    }
    Ok(())
}

pub(crate) fn forget_profile(profile_id: &str) -> Result<bool> {
    let _guard = profile_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("Android 设备档案锁已损坏"))?;
    let path = profiles_path();
    let mut file = read_file(&path)?;
    let previous_len = file.profiles.len();
    file.profiles.retain(|profile| profile.id != profile_id);
    let removed = file.profiles.len() != previous_len;
    if removed {
        write_file(&path, &file)?;
    }
    Ok(removed)
}

fn update_profile(
    profile_id: Option<&str>,
    update: impl FnOnce(&mut AndroidDeviceProfile),
) -> Result<()> {
    let Some(profile_id) = profile_id else {
        return Ok(());
    };
    let _guard = profile_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("Android 设备档案锁已损坏"))?;
    let path = profiles_path();
    let mut file = read_file(&path)?;
    if let Some(profile) = file
        .profiles
        .iter_mut()
        .find(|profile| profile.id == profile_id)
    {
        update(profile);
        profile.last_seen_at = Utc::now().to_rfc3339();
        write_file(&path, &file)?;
    }
    Ok(())
}

fn clean_display_name(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(80).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_file_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "elon-android-profile-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let path = dir.join("devices.json");
        let file = ProfileFile {
            schema_version: PROFILE_SCHEMA_VERSION,
            profiles: Vec::new(),
        };
        write_file(&path, &file).unwrap();
        let loaded = read_file(&path).unwrap();
        assert_eq!(loaded.schema_version, PROFILE_SCHEMA_VERSION);
        assert!(loaded.profiles.is_empty());
        let _ = fs::remove_dir_all(dir);
    }
}
