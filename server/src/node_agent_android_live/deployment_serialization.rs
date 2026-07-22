use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, OwnedMutexGuard};

/// Serializes build/install/handshake for one fixed app on one Android device.
///
/// Different Codex sessions and worktrees on the same PC node intentionally
/// share the node-scoped Debug package. They may edit source concurrently, but
/// only one deployment can own that package's process and Runtime handshake at
/// a time.
#[derive(Default)]
pub(crate) struct DebugDeploymentRegistry {
    node_install_id: Option<String>,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl DebugDeploymentRegistry {
    pub(crate) fn for_node(install_id: &str) -> Self {
        Self {
            node_install_id: Some(install_id.trim().to_string()),
            locks: Mutex::default(),
        }
    }

    pub(crate) fn node_install_id(&self) -> Option<&str> {
        self.node_install_id.as_deref()
    }

    pub(crate) async fn acquire(&self, device_id: &str, package_name: &str) -> OwnedMutexGuard<()> {
        let key = format!("{}\n{}", device_id.trim(), package_name.trim());
        let lock = self
            .locks
            .lock()
            .await
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        lock.lock_owned().await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::DebugDeploymentRegistry;

    #[tokio::test]
    async fn same_device_and_package_wait_for_the_active_deployment() {
        let registry = DebugDeploymentRegistry::default();
        let first = registry.acquire("phone-a", "com.elon.app.debug").await;
        assert!(tokio::time::timeout(
            Duration::from_millis(20),
            registry.acquire("phone-a", "com.elon.app.debug")
        )
        .await
        .is_err());
        drop(first);
        tokio::time::timeout(
            Duration::from_millis(100),
            registry.acquire("phone-a", "com.elon.app.debug"),
        )
        .await
        .expect("deployment slot should become available");
    }

    #[tokio::test]
    async fn different_node_packages_do_not_block_each_other() {
        let registry = DebugDeploymentRegistry::default();
        let _first = registry.acquire("phone-a", "com.elon.app.node_a").await;
        tokio::time::timeout(
            Duration::from_millis(100),
            registry.acquire("phone-a", "com.elon.app.node_b"),
        )
        .await
        .expect("different node-scoped packages may deploy independently");
    }

    #[test]
    fn node_identity_is_available_to_all_debug_entry_points() {
        let registry = DebugDeploymentRegistry::for_node(" install-a ");
        assert_eq!(registry.node_install_id(), Some("install-a"));
    }
}
