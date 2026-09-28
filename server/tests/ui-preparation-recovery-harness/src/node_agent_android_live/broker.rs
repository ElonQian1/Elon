use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Default)]
pub(crate) struct LiveUiBroker {
    sessions: RwLock<HashMap<String, Arc<LiveUiSession>>>,
    pub(crate) debug_deployments: super::deployment_serialization::DebugDeploymentRegistry,
    pub(crate) debug_integration: super::debug_integration::DebugIntegrationCoordinator,
    pub(crate) debug_runtime_preparations: super::build_verify::PreparationRegistry,
}
pub(crate) struct LiveUiSession {
    pub(crate) id: String,
    pub(crate) device_id: String,
    pub(crate) package_name: String,
    pub(crate) project_root: Option<String>,
}
pub(crate) struct SessionView {
    pub connected: bool,
}
impl LiveUiSession {
    pub(crate) async fn view(&self) -> SessionView {
        SessionView { connected: false }
    }
}
fn canonical_or_raw(root: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(root)
        .canonicalize()
        .unwrap_or_else(|_| root.into())
}
#[path = "../../../../src/node_agent_android_live/broker/renderer_routing.rs"]
mod renderer_routing;
impl LiveUiBroker {
    pub(crate) async fn remove_session(&self, id: &str) -> Option<Arc<LiveUiSession>> {
        self.sessions.write().await.remove(id)
    }
    pub(crate) fn for_node(install_id: &str, root: std::path::PathBuf) -> Self {
        Self {
            debug_deployments: super::deployment_serialization::DebugDeploymentRegistry::for_node(
                install_id,
                root.clone(),
            ),
            debug_integration: super::debug_integration::DebugIntegrationCoordinator::new(
                root,
                super::node_debug_fingerprint(install_id).unwrap(),
            ),
            ..Self::default()
        }
    }
    pub(crate) async fn create_session(
        &self,
        device_id: String,
        package_name: String,
        project_root: Option<String>,
        _: u16,
    ) -> Arc<LiveUiSession> {
        let session = Arc::new(LiveUiSession {
            id: format!("live_{}", uuid::Uuid::new_v4()),
            device_id,
            package_name,
            project_root,
        });
        self.sessions
            .write()
            .await
            .insert(session.id.clone(), session.clone());
        session
    }
    pub(crate) async fn session(&self, id: &str) -> anyhow::Result<Arc<LiveUiSession>> {
        self.sessions
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing session"))
    }
    pub(crate) async fn runtime_session(
        &self,
        device_id: &str,
        package_name: &str,
    ) -> Option<Arc<LiveUiSession>> {
        self.sessions
            .read()
            .await
            .values()
            .find(|s| s.device_id == device_id && s.package_name == package_name)
            .cloned()
    }
}
