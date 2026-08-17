use std::{
    fmt::Write as _,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[cfg(not(windows))]
use std::io::Read as _;

use super::{display_error, owner_profile, PROFILE_ROOT};

const GUEST_IDENTITY_SCHEMA: &str = "elon.local-ai.guest-owner.v1";
const GUEST_IDENTITY_FILE: &str = "guest-owner.v1.json";
const MAX_IDENTITY_BYTES: u64 = 512;
const NATIVE_OWNER_PREFIX: &str = "anonymous-device:pc-native:";
static GUEST_IDENTITY_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAiGuestOwnerIdentity {
    owner_key: String,
    persistence: &'static str,
    migrated_legacy: bool,
}

#[derive(Deserialize, Serialize)]
struct StoredGuestIdentity {
    schema: String,
    owner_key: String,
}

pub(super) fn resolve(
    app: AppHandle,
    legacy_owner_key: Option<String>,
) -> Result<LocalAiGuestOwnerIdentity, String> {
    let _guard = GUEST_IDENTITY_LOCK
        .lock()
        .map_err(|_| "本机游客身份锁不可用，请重启客户端后重试。".to_string())?;
    let root = app
        .path()
        .app_local_data_dir()
        .map(|path| path.join(PROFILE_ROOT))
        .map_err(display_error)?;
    let (owner_key, migrated_legacy) =
        resolve_at(&root, legacy_owner_key.as_deref(), generate_owner_key)?;
    Ok(LocalAiGuestOwnerIdentity {
        owner_key,
        persistence: "native_device",
        migrated_legacy,
    })
}

fn resolve_at<F>(
    root: &Path,
    legacy_owner_key: Option<&str>,
    generate: F,
) -> Result<(String, bool), String>
where
    F: FnOnce() -> Result<String, String>,
{
    let identity_path = root.join(GUEST_IDENTITY_FILE);
    if let Ok(Some(owner_key)) = load_identity(&identity_path) {
        return Ok((owner_key, false));
    }

    let legacy_owner_key = legacy_owner_key.and_then(|value| normalize_owner_key(value).ok());
    let migrated_legacy = legacy_owner_key.is_some();
    let owner_key = match legacy_owner_key {
        Some(value) => value,
        None => normalize_owner_key(&generate()?)?,
    };
    store_identity(&identity_path, &owner_key)?;

    let persisted =
        load_identity(&identity_path)?.ok_or_else(|| "本机游客身份写入后无法回读。".to_string())?;
    if persisted != owner_key {
        return Err("本机游客身份写入校验失败。".to_string());
    }
    Ok((persisted, migrated_legacy))
}

fn load_identity(path: &Path) -> Result<Option<String>, String> {
    let metadata = match fs::metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("无法读取本机游客身份状态：{error}")),
    };
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_IDENTITY_BYTES {
        return Err("本机游客身份文件大小无效。".to_string());
    }
    let bytes = fs::read(path).map_err(|error| format!("无法读取本机游客身份：{error}"))?;
    let stored: StoredGuestIdentity =
        serde_json::from_slice(&bytes).map_err(|error| format!("本机游客身份格式无效：{error}"))?;
    if stored.schema != GUEST_IDENTITY_SCHEMA {
        return Err("本机游客身份版本不受支持。".to_string());
    }
    normalize_owner_key(&stored.owner_key).map(Some)
}

fn store_identity(path: &Path, owner_key: &str) -> Result<(), String> {
    let root = path
        .parent()
        .ok_or_else(|| "本机游客身份目录无效。".to_string())?;
    fs::create_dir_all(root).map_err(|error| format!("无法创建本机游客身份目录：{error}"))?;
    let stored = StoredGuestIdentity {
        schema: GUEST_IDENTITY_SCHEMA.to_string(),
        owner_key: owner_key.to_string(),
    };
    let bytes =
        serde_json::to_vec(&stored).map_err(|error| format!("无法编码本机游客身份：{error}"))?;
    let temporary_path = temporary_path(path);
    let _ = fs::remove_file(&temporary_path);
    let mut temporary = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary_path)
        .map_err(|error| format!("无法创建本机游客身份临时文件：{error}"))?;
    temporary
        .write_all(&bytes)
        .and_then(|_| temporary.sync_all())
        .map_err(|error| format!("无法写入本机游客身份：{error}"))?;
    drop(temporary);

    let backup_path = if path.exists() {
        let backup = invalid_backup_path(path);
        fs::rename(path, &backup)
            .map_err(|error| format!("无法保留损坏的本机游客身份：{error}"))?;
        Some(backup)
    } else {
        None
    };
    if let Err(error) = fs::rename(&temporary_path, path) {
        if let Some(backup) = backup_path {
            let _ = fs::rename(backup, path);
        }
        return Err(format!("无法启用本机游客身份：{error}"));
    }
    Ok(())
}

fn normalize_owner_key(value: &str) -> Result<String, String> {
    let value = value.trim();
    if !value.starts_with("anonymous-device:") {
        return Err("本机游客身份前缀无效。".to_string());
    }
    owner_profile::fingerprint(value)?;
    Ok(value.to_string())
}

fn generate_owner_key() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    fill_random(&mut bytes)?;
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut token, "{byte:02x}").map_err(|_| "无法编码本机游客身份。".to_string())?;
    }
    Ok(format!("{NATIVE_OWNER_PREFIX}{token}"))
}

#[cfg(windows)]
fn fill_random(bytes: &mut [u8]) -> Result<(), String> {
    use windows_sys::Win32::Security::Cryptography::{
        BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    };

    let length =
        u32::try_from(bytes.len()).map_err(|_| "本机游客身份随机缓冲区过大。".to_string())?;
    let status = unsafe {
        BCryptGenRandom(
            std::ptr::null_mut(),
            bytes.as_mut_ptr(),
            length,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status >= 0 {
        Ok(())
    } else {
        Err(format!("无法生成本机游客身份：系统随机源错误 {status:#x}"))
    }
}

#[cfg(not(windows))]
fn fill_random(bytes: &mut [u8]) -> Result<(), String> {
    fs::File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(bytes))
        .map_err(|error| format!("无法生成本机游客身份：{error}"))
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension(format!("{}.tmp", std::process::id()))
}

fn invalid_backup_path(path: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    path.with_extension(format!("invalid-{stamp}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_browser_owner_without_changing_profile_identity() {
        let root = temporary_root("legacy");
        let legacy = "anonymous-device:pc:legacy-browser-owner";
        let resolved = resolve_at(&root, Some(legacy), || {
            panic!("legacy identity should avoid generation")
        })
        .unwrap();

        assert_eq!(resolved, (legacy.to_string(), true));
        assert_eq!(
            load_identity(&root.join(GUEST_IDENTITY_FILE)).unwrap(),
            Some(legacy.to_string())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_identity_wins_after_browser_storage_changes() {
        let root = temporary_root("native-wins");
        let first = resolve_at(&root, None, || {
            Ok("anonymous-device:pc-native:first".to_string())
        })
        .unwrap();
        let second = resolve_at(
            &root,
            Some("anonymous-device:pc:different-browser-owner"),
            || panic!("persisted identity should avoid generation"),
        )
        .unwrap();

        assert_eq!(
            first,
            ("anonymous-device:pc-native:first".to_string(), false)
        );
        assert_eq!(
            second,
            ("anonymous-device:pc-native:first".to_string(), false)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_identity_recovers_from_legacy_owner() {
        let root = temporary_root("repair");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(GUEST_IDENTITY_FILE), b"not-json").unwrap();
        let legacy = "anonymous-device:pc:recoverable-owner";

        let resolved = resolve_at(&root, Some(legacy), || {
            panic!("legacy repair should avoid generation")
        })
        .unwrap();

        assert_eq!(resolved, (legacy.to_string(), true));
        assert_eq!(
            load_identity(&root.join(GUEST_IDENTITY_FILE)).unwrap(),
            Some(legacy.to_string())
        );
        assert!(fs::read_dir(&root).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("invalid-")
        }));
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary_root(case: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "elon-guest-identity-{case}-{}-{unique}",
            std::process::id()
        ))
    }
}
