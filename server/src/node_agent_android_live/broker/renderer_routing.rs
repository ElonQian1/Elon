use std::collections::HashSet;

use anyhow::{bail, Result};

use super::{canonical_or_raw, LiveUiBroker};

impl LiveUiBroker {
    pub(crate) async fn renderer_devices_owned_by_other_sessions(
        &self,
        owner_session_id: &str,
    ) -> HashSet<String> {
        self.sessions
            .read()
            .await
            .values()
            .filter(|session| {
                session.id != owner_session_id
                    && session.device_id != "ui-design-bootstrap"
                    && session.package_name != "ui.design.bootstrap"
            })
            .map(|session| session.device_id.clone())
            .collect()
    }

    pub(crate) async fn effective_session_id(&self, session_id: &str) -> Result<String> {
        let requested = self.session(session_id).await?;
        if requested.view().await.connected {
            return Ok(requested.id.clone());
        }
        let Some(root) = requested.project_root.as_deref() else {
            return Ok(requested.id.clone());
        };
        let expected = canonical_or_raw(root);
        let sessions = self
            .sessions
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut matched = Vec::new();
        for session in sessions {
            if session.id == requested.id
                || session.device_id == "ui-design-bootstrap"
                || session.package_name == "ui.design.bootstrap"
            {
                continue;
            }
            let Some(candidate_root) = session.project_root.as_deref() else {
                continue;
            };
            if canonical_or_raw(candidate_root) == expected && session.view().await.connected {
                matched.push(session.id.clone());
            }
        }
        match matched.as_slice() {
            [] => Ok(requested.id.clone()),
            [only] => Ok(only.clone()),
            _ => bail!(
                "项目存在多个已连接 Android Renderer；当前 MCP bootstrap 会话未绑定明确 sessionId/deviceIdentity，拒绝仅按 projectRoot 猜测"
            ),
        }
    }
}
