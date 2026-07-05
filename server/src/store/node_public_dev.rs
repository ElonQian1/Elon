//! Public development sharing for registered PC nodes.

use anyhow::Result;
use homecli_proto::NodeDevRuntimeProfile;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{node_ledger::NodeCredential, now, Store};

#[derive(Debug, Clone, Serialize)]
pub struct NodePublicDevSharing {
    pub agent_id: String,
    pub owner_user_id: String,
    pub enabled: bool,
    pub allowed_clis: Vec<String>,
    pub permission_level: String,
    pub last_handshake_at: Option<String>,
    pub last_handshake_agent_version: Option<String>,
    pub last_handshake_allowed_clis: Vec<String>,
    pub last_handshake_route_a_ready: bool,
    pub last_handshake_api_runtime_ready: bool,
    pub last_handshake_server_runtime_ready: bool,
    pub last_handshake_ai_cli_ready: bool,
}

impl Store {
    pub fn get_node_public_dev_sharing(
        &self,
        agent_id: &str,
    ) -> Result<Option<NodePublicDevSharing>> {
        Ok(self
            .get_node_credential(agent_id)?
            .map(node_public_dev_sharing_from_credential))
    }

    pub fn update_node_public_dev_sharing(
        &self,
        owner_user_id: &str,
        agent_id: &str,
        enabled: bool,
        allowed_clis: &[String],
        permission_level: &str,
    ) -> Result<Option<NodePublicDevSharing>> {
        let agent_id = agent_id.trim();
        if agent_id.is_empty() {
            return Ok(None);
        }
        let allowed_clis = normalize_cli_list(allowed_clis);
        let allowed_clis_json = serde_json::to_string(&allowed_clis)?;
        let permission_level = super::node_ledger::normalize_permission_level(permission_level);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE node_credentials
             SET public_dev_enabled = ?3,
                 public_dev_allowed_clis_json = ?4,
                 public_dev_permission_level = ?5
             WHERE agent_id = ?1
               AND owner_user_id = ?2",
            params![
                agent_id,
                owner_user_id,
                if enabled { 1 } else { 0 },
                allowed_clis_json,
                permission_level,
            ],
        )?;
        conn.query_row(
            super::node_ledger::node_credential_select_sql(
                "WHERE agent_id = ?1 AND owner_user_id = ?2",
            )
            .as_str(),
            params![agent_id, owner_user_id],
            super::node_ledger::read_node_credential,
        )
        .optional()
        .map(|credential| credential.map(node_public_dev_sharing_from_credential))
        .map_err(Into::into)
    }

    pub fn record_node_handshake(
        &self,
        agent_id: &str,
        owner_user_id: &str,
        agent_version: &str,
        allowed_clis: &[String],
        dev_runtime: Option<&NodeDevRuntimeProfile>,
    ) -> Result<()> {
        let agent_id = agent_id.trim();
        let owner_user_id = owner_user_id.trim();
        if agent_id.is_empty() || owner_user_id.is_empty() {
            return Ok(());
        }
        let allowed_clis = normalize_cli_list(allowed_clis);
        let allowed_clis_json = serde_json::to_string(&allowed_clis)?;
        let route_a_ready = dev_runtime
            .map(|runtime| runtime.route_a_ready)
            .unwrap_or_else(|| !allowed_clis.is_empty());
        let api_runtime_ready = dev_runtime
            .map(|runtime| runtime.api_runtime_ready)
            .unwrap_or(false);
        let server_runtime_ready = dev_runtime
            .map(|runtime| runtime.server_runtime_ready)
            .unwrap_or(false);
        let ai_cli_ready = dev_runtime
            .map(|runtime| runtime.ai_cli_ready)
            .unwrap_or_else(|| !allowed_clis.is_empty());
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE node_credentials
             SET last_handshake_at = ?3,
                 last_handshake_agent_version = ?4,
                 last_handshake_allowed_clis_json = ?5,
                 last_handshake_route_a_ready = ?6,
                 last_handshake_api_runtime_ready = ?7,
                 last_handshake_server_runtime_ready = ?8,
                 last_handshake_ai_cli_ready = ?9
             WHERE agent_id = ?1
               AND owner_user_id = ?2",
            params![
                agent_id,
                owner_user_id,
                now(),
                agent_version.trim(),
                allowed_clis_json,
                if route_a_ready { 1 } else { 0 },
                if api_runtime_ready { 1 } else { 0 },
                if server_runtime_ready { 1 } else { 0 },
                if ai_cli_ready { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn list_all_node_credentials(&self) -> Result<Vec<NodeCredential>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            super::node_ledger::node_credential_select_sql("ORDER BY created_at DESC").as_str(),
        )?;
        let rows = stmt.query_map([], super::node_ledger::read_node_credential)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn list_public_dev_node_credentials(&self) -> Result<Vec<NodeCredential>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            super::node_ledger::node_credential_select_sql(
                "WHERE public_dev_enabled = 1 ORDER BY last_handshake_at DESC, created_at DESC",
            )
            .as_str(),
        )?;
        let rows = stmt.query_map([], super::node_ledger::read_node_credential)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

fn node_public_dev_sharing_from_credential(credential: NodeCredential) -> NodePublicDevSharing {
    NodePublicDevSharing {
        agent_id: credential.agent_id,
        owner_user_id: credential.owner_user_id,
        enabled: credential.public_dev_enabled,
        allowed_clis: credential.public_dev_allowed_clis,
        permission_level: credential.public_dev_permission_level,
        last_handshake_at: credential.last_handshake_at,
        last_handshake_agent_version: credential.last_handshake_agent_version,
        last_handshake_allowed_clis: credential.last_handshake_allowed_clis,
        last_handshake_route_a_ready: credential.last_handshake_route_a_ready,
        last_handshake_api_runtime_ready: credential.last_handshake_api_runtime_ready,
        last_handshake_server_runtime_ready: credential.last_handshake_server_runtime_ready,
        last_handshake_ai_cli_ready: credential.last_handshake_ai_cli_ready,
    }
}

fn normalize_cli_list(values: &[String]) -> Vec<String> {
    let mut normalized = values
        .iter()
        .filter_map(|value| {
            let value = value.trim().to_ascii_lowercase();
            (!value.is_empty()).then_some(value)
        })
        .fold(Vec::<String>::new(), |mut acc, value| {
            if !acc.iter().any(|existing| existing == &value) {
                acc.push(value);
            }
            acc
        });
    if normalized.is_empty() {
        normalized = vec![
            "codex".to_string(),
            "copilot".to_string(),
            "claude".to_string(),
            "gemini".to_string(),
        ];
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-node-public-dev-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn node_credentials_default_to_public_dev_and_record_handshake() {
        let (store, path) = temp_store();
        let owner = store
            .create_user(
                "public-dev-provider@example.com",
                "secret1",
                Some("provider"),
                None,
            )
            .unwrap();
        store
            .create_node_credential(
                "node-public-dev",
                "secret-hash",
                &owner.id,
                Some("provider-4060"),
                Some("PROVIDER4060"),
                Some("install-public-dev"),
            )
            .unwrap();

        let credential = store
            .get_node_credential("node-public-dev")
            .unwrap()
            .expect("credential");
        assert!(credential.public_dev_enabled);
        assert_eq!(credential.public_dev_permission_level, "project_write");
        assert!(credential
            .public_dev_allowed_clis
            .iter()
            .any(|cli| cli == "codex"));
        assert!(credential.last_handshake_at.is_none());

        let dev_runtime = NodeDevRuntimeProfile {
            ai_cli_ready: true,
            route_a_ready: true,
            api_runtime_ready: false,
            server_runtime_ready: true,
            workspace_provision_ready: true,
            ..NodeDevRuntimeProfile::default()
        };
        store
            .record_node_handshake(
                "node-public-dev",
                &owner.id,
                "0.3.70",
                &["Codex".to_string(), "copilot".to_string()],
                Some(&dev_runtime),
            )
            .unwrap();

        let credential = store
            .get_node_credential("node-public-dev")
            .unwrap()
            .expect("credential after handshake");
        assert!(credential.last_handshake_at.is_some());
        assert_eq!(
            credential.last_handshake_agent_version.as_deref(),
            Some("0.3.70")
        );
        assert_eq!(
            credential.last_handshake_allowed_clis,
            vec!["codex".to_string(), "copilot".to_string()]
        );
        assert!(credential.last_handshake_route_a_ready);
        assert!(!credential.last_handshake_api_runtime_ready);
        assert!(credential.last_handshake_server_runtime_ready);
        assert!(credential.last_handshake_ai_cli_ready);

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
