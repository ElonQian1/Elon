use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

const MAX_SUFFIX_LEN: usize = 40;
const NODE_SCOPE_LEN: usize = 8;

pub(crate) fn scoped_debug_application_id_suffix(
    requested_suffix: &str,
    install_id: &str,
) -> Result<String> {
    let requested_suffix = requested_suffix.trim();
    if requested_suffix.len() + NODE_SCOPE_LEN + 1 > MAX_SUFFIX_LEN {
        bail!("debugApplicationIdSuffix 过长，无法附加稳定的 PC 节点标识");
    }
    if install_id.trim().is_empty() {
        bail!("PC 节点缺少稳定安装标识，无法创建独立调试包");
    }
    let digest = Sha256::digest(install_id.trim().as_bytes());
    let node_scope = hex::encode(&digest[..NODE_SCOPE_LEN / 2]);
    Ok(format!("{requested_suffix}_{node_scope}"))
}

#[cfg(test)]
mod tests {
    use super::scoped_debug_application_id_suffix;

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
}
