use std::{fmt::Write as _, fs, path::Path};

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

use super::{display_error, ProviderDefinition, PROFILE_ROOT};

pub(super) fn fingerprint(owner_key: &str) -> Result<String, String> {
    let owner_key = owner_key.trim();
    if owner_key.is_empty()
        || owner_key.chars().count() > 128
        || owner_key.chars().any(char::is_control)
    {
        return Err("一龙账号标识无效，无法创建本地隔离 Profile。".to_string());
    }
    let digest = Sha256::digest(owner_key.as_bytes());
    let mut fingerprint = String::with_capacity(32);
    for byte in &digest[..16] {
        write!(&mut fingerprint, "{byte:02x}")
            .map_err(|_| "无法生成一龙账号隔离指纹。".to_string())?;
    }
    Ok(fingerprint)
}

pub(super) fn resolve(
    app: &AppHandle,
    provider: &ProviderDefinition,
    owner_key: &str,
) -> Result<String, String> {
    let current = fingerprint(owner_key)?;
    let legacy = legacy_fingerprint(owner_key.trim());
    let root = app
        .path()
        .app_local_data_dir()
        .map(|path| path.join(PROFILE_ROOT))
        .map_err(display_error)?;
    let legacy_provider = root.join(&legacy).join(provider.id);
    let current_provider = root.join(&current).join(provider.id);
    if legacy_provider.is_dir() && !current_provider.exists() {
        if let Err(error) = migrate_provider(&root, provider.id, &legacy, &current) {
            if current_provider.is_dir() {
                return Ok(current);
            }
            eprintln!(
                "[elon-desktop][local-ai] {} 账号 Profile 指纹迁移延后：{}",
                provider.id, error
            );
            return Ok(legacy);
        }
    }
    Ok(current)
}

fn legacy_fingerprint(owner_key: &str) -> String {
    let hash = owner_key
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("{hash:016x}")
}

fn migrate_provider(
    root: &Path,
    provider_id: &str,
    legacy_fingerprint: &str,
    current_fingerprint: &str,
) -> std::io::Result<()> {
    let legacy_owner = root.join(legacy_fingerprint);
    let legacy_provider = legacy_owner.join(provider_id);
    let current_provider = root.join(current_fingerprint).join(provider_id);
    if current_provider.exists() || !legacy_provider.is_dir() {
        return Ok(());
    }
    let current_owner = current_provider
        .parent()
        .expect("provider profile always has an owner directory");
    fs::create_dir_all(current_owner)?;
    fs::rename(&legacy_provider, &current_provider)?;
    let _ = fs::remove_dir(legacy_owner);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_separate_and_path_safe() {
        let first = fingerprint("account-15692409892").unwrap();
        let second = fingerprint("account-15692409892").unwrap();
        let other = fingerprint("another-account").unwrap();
        assert_eq!(first, second);
        assert_ne!(first, other);
        assert_ne!(first, legacy_fingerprint("account-15692409892"));
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|value| value.is_ascii_hexdigit()));
    }

    #[test]
    fn legacy_profile_is_migrated_without_losing_provider_data() {
        let root = temporary_root("migration");
        let legacy = legacy_fingerprint("account-15692409892");
        let current = fingerprint("account-15692409892").unwrap();
        let legacy_provider = root.join(&legacy).join("chatgpt");
        fs::create_dir_all(&legacy_provider).unwrap();
        fs::write(legacy_provider.join("Cookies"), b"opaque-webview-data").unwrap();

        migrate_provider(&root, "chatgpt", &legacy, &current).unwrap();

        assert!(!legacy_provider.exists());
        assert_eq!(
            fs::read(root.join(&current).join("chatgpt").join("Cookies")).unwrap(),
            b"opaque-webview-data"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_never_overwrites_an_existing_current_profile() {
        let root = temporary_root("no-overwrite");
        let legacy = legacy_fingerprint("account-15692409892");
        let current = fingerprint("account-15692409892").unwrap();
        let legacy_provider = root.join(&legacy).join("chatgpt");
        let current_provider = root.join(&current).join("chatgpt");
        fs::create_dir_all(&legacy_provider).unwrap();
        fs::create_dir_all(&current_provider).unwrap();
        fs::write(legacy_provider.join("Cookies"), b"legacy").unwrap();
        fs::write(current_provider.join("Cookies"), b"current").unwrap();

        migrate_provider(&root, "chatgpt", &legacy, &current).unwrap();

        assert_eq!(
            fs::read(legacy_provider.join("Cookies")).unwrap(),
            b"legacy"
        );
        assert_eq!(
            fs::read(current_provider.join("Cookies")).unwrap(),
            b"current"
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn temporary_root(case: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "elon-owner-profile-{case}-{}-{unique}",
            std::process::id()
        ))
    }
}
