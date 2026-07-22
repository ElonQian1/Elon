use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{Mutex, RwLock};

use super::{LiveSessionState, LiveUiBroker, LiveUiSession};

impl LiveUiBroker {
    pub(crate) async fn create_session(
        &self,
        device_id: String,
        package_name: String,
        project_root: Option<String>,
        device_port: u16,
    ) -> Arc<LiveUiSession> {
        let device_identity = device_id.clone();
        let debug_project_id = project_root.clone().unwrap_or_default();
        self.create_session_with_identity(
            device_id,
            device_identity,
            package_name,
            project_root,
            debug_project_id,
            device_port,
        )
        .await
    }

    pub(crate) async fn create_session_with_identity(
        &self,
        device_id: String,
        device_identity: String,
        package_name: String,
        project_root: Option<String>,
        debug_project_id: String,
        device_port: u16,
    ) -> Arc<LiveUiSession> {
        let session = Arc::new(LiveUiSession {
            id: format!("live_{}", uuid::Uuid::new_v4().simple()),
            token: uuid::Uuid::new_v4().simple().to_string(),
            device_id,
            device_identity,
            debug_project_id,
            package_name,
            project_root,
            device_port,
            created_at: Utc::now().to_rfc3339(),
            state: RwLock::new(LiveSessionState::default()),
            runtime_tx: RwLock::new(None),
            pending: Mutex::new(HashMap::new()),
        });
        self.sessions
            .write()
            .await
            .insert(session.id.clone(), session.clone());
        session
    }
}
