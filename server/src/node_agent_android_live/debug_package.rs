use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

const MAX_SUFFIX_LEN: usize = 40;
const NODE_SCOPE_LEN: usize = 8;
const SHARED_DEBUG_PREFIX: &str = ".uituner";

pub(crate) fn node_debug_fingerprint(install_id: &str) -> Result<String> {
    let install_id = install_id.trim();
    if install_id.is_empty() {
        bail!("PC 节点缺少稳定安装标识，无法确定唯一调试包身份");
    }
    let digest = Sha256::digest(install_id.as_bytes());
    Ok(hex::encode(&digest[..NODE_SCOPE_LEN / 2]))
}

pub(crate) fn fixed_node_debug_suffix(install_id: &str) -> Result<String> {
    Ok(format!(
        "{SHARED_DEBUG_PREFIX}_{}",
        node_debug_fingerprint(install_id)?
    ))
}

/// Resolves every physical-device compatibility suffix to the node's one fixed
/// Launcher package. Emulator-only isolation is opt-in and cannot be used for a
/// physical device.
pub(crate) fn resolve_debug_application_id_suffix(
    requested_suffix: &str,
    install_id: &str,
    device_id: &str,
    isolated_emulator_package: bool,
) -> Result<String> {
    let requested = validate_requested_debug_family(requested_suffix)?;
    let emulator = device_id.trim().starts_with("emulator-");
    if isolated_emulator_package {
        if !emulator {
            bail!("ISOLATED_DEBUG_PACKAGE_PHYSICAL_DEVICE_FORBIDDEN: 真机只能使用节点唯一固定调试包，不能用测试后缀创建第二个 Launcher 应用");
        }
        return scoped_debug_application_id_suffix(requested, install_id);
    }
    fixed_node_debug_suffix(install_id)
}

pub(crate) fn normalize_debug_package_name(
    package_name: &str,
    install_id: &str,
    device_id: &str,
) -> Result<String> {
    let package_name = package_name.trim();
    let Some((base, suffix)) = split_known_debug_package(package_name) else {
        return Ok(package_name.to_string());
    };
    let resolved = resolve_debug_application_id_suffix(suffix, install_id, device_id, false)?;
    Ok(format!("{base}{resolved}"))
}

pub(crate) fn debug_base_package_name(package_name: &str) -> &str {
    split_known_debug_package(package_name.trim())
        .map(|(base, _)| base)
        .unwrap_or_else(|| package_name.trim())
}

fn split_known_debug_package(package_name: &str) -> Option<(&str, &str)> {
    [".uitest_anim", ".uituner", ".uitest"]
        .into_iter()
        .filter_map(|marker| {
            package_name
                .rfind(marker)
                .map(|index| (&package_name[..index], &package_name[index..]))
                .filter(|(_, suffix)| validate_requested_debug_family(suffix).is_ok())
        })
        .max_by_key(|(base, _)| base.len())
}

fn validate_requested_debug_family(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.len() > MAX_SUFFIX_LEN
        || ![".uituner", ".uitest", ".uitest_anim"]
            .into_iter()
            .any(|prefix| {
                value == prefix
                    || value.strip_prefix(prefix).is_some_and(|tail| {
                        tail.starts_with('_')
                            && tail[1..]
                                .bytes()
                                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    })
            })
    {
        bail!("ILLEGAL_DEBUG_SUFFIX: 只允许 .uituner、.uitest、.uitest_anim 兼容族；真机最终都会归一为节点唯一 .uituner_<指纹> 包")
    }
    Ok(value)
}

pub(crate) fn scoped_debug_application_id_suffix(
    requested_suffix: &str,
    install_id: &str,
) -> Result<String> {
    let requested_suffix = requested_suffix.trim();
    if requested_suffix.len() + NODE_SCOPE_LEN + 1 > MAX_SUFFIX_LEN {
        bail!("debugApplicationIdSuffix 过长，无法附加稳定的 PC 节点标识");
    }
    let node_scope = node_debug_fingerprint(install_id)?;
    Ok(format!("{requested_suffix}_{node_scope}"))
}

#[cfg(test)]
mod tests {
    use super::{
        debug_base_package_name, fixed_node_debug_suffix, normalize_debug_package_name,
        resolve_debug_application_id_suffix, scoped_debug_application_id_suffix,
    };

    #[test]
    fn suffix_is_stable_and_isolated_per_node() {
        let first = scoped_debug_application_id_suffix(".uituner", "install-a").unwrap();
        assert_eq!(
            first,
            scoped_debug_application_id_suffix(".uituner", "install-a").unwrap()
        );
        assert_ne!(
            first,
            scoped_debug_application_id_suffix(".uituner", "install-b").unwrap()
        );
        assert!(first.starts_with(".uituner_"));
        assert_eq!(first.len(), ".uituner_".len() + 8);
    }

    #[test]
    fn physical_compatibility_suffixes_cannot_create_second_package() {
        let expected = fixed_node_debug_suffix("install-a").unwrap();
        for requested in [".uituner", ".uitest", ".uitest_anim", ".uitest_legacy"] {
            assert_eq!(
                resolve_debug_application_id_suffix(requested, "install-a", "phone-a", false)
                    .unwrap(),
                expected
            );
        }
        assert_eq!(
            debug_base_package_name("com.elon.app.uitest_anim"),
            "com.elon.app"
        );
        assert_eq!(
            debug_base_package_name("com.elon.app.uitest_legacy"),
            "com.elon.app"
        );
        assert!(
            resolve_debug_application_id_suffix(".uitest_anim", "install-a", "phone-a", true)
                .is_err()
        );
        assert!(
            resolve_debug_application_id_suffix(".another", "install-a", "phone-a", false)
                .unwrap_err()
                .to_string()
                .contains("ILLEGAL_DEBUG_SUFFIX")
        );
    }

    #[test]
    fn emulator_isolation_is_explicit_and_does_not_change_formal_package() {
        let isolated =
            resolve_debug_application_id_suffix(".uitest_anim", "install-a", "emulator-5554", true)
                .unwrap();
        assert!(isolated.starts_with(".uitest_anim_"));
        assert_eq!(
            normalize_debug_package_name("com.elon.app", "install-a", "phone-a").unwrap(),
            "com.elon.app"
        );
        assert_eq!(
            normalize_debug_package_name("com.example.uitester", "install-a", "phone-a").unwrap(),
            "com.example.uitester"
        );
        assert_eq!(
            normalize_debug_package_name("com.elon.app.uitest_anim", "install-a", "phone-a")
                .unwrap(),
            format!(
                "com.elon.app{}",
                fixed_node_debug_suffix("install-a").unwrap()
            )
        );
    }
}
