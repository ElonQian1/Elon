use anyhow::Result;
use homecli_proto::NodeHardwareProfile;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{common::now, Store};

#[derive(Debug, Clone, Serialize)]
pub struct NodeHardwareSnapshot {
    pub node_id: String,
    pub owner_user_id: String,
    pub device_name: Option<String>,
    pub hardware: NodeHardwareProfile,
    pub created_at: String,
    pub updated_at: String,
}

impl Store {
    pub fn upsert_node_hardware_snapshot(
        &self,
        node_id: &str,
        owner_user_id: &str,
        device_name: Option<&str>,
        hardware: &NodeHardwareProfile,
    ) -> Result<()> {
        let node_id = node_id.trim();
        let owner_user_id = owner_user_id.trim();
        if node_id.is_empty() || owner_user_id.is_empty() {
            return Ok(());
        }
        if !has_hardware_signal(hardware) {
            return Ok(());
        }
        let device_name = device_name.map(str::trim).filter(|value| !value.is_empty());
        let hardware_json = serde_json::to_string(hardware)?;
        let ts = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO node_hardware_snapshots
               (node_id, owner_user_id, device_name, hardware_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(node_id) DO UPDATE SET
               owner_user_id = excluded.owner_user_id,
               device_name = COALESCE(excluded.device_name, node_hardware_snapshots.device_name),
               hardware_json = excluded.hardware_json,
               updated_at = excluded.updated_at",
            params![node_id, owner_user_id, device_name, hardware_json, ts],
        )?;
        Ok(())
    }

    pub fn get_node_hardware_snapshot(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeHardwareSnapshot>> {
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return Ok(None);
        }
        let conn = self.conn()?;
        Ok(conn
            .query_row(
                "SELECT node_id, owner_user_id, device_name, hardware_json, created_at, updated_at
             FROM node_hardware_snapshots
             WHERE node_id = ?1",
                params![node_id],
                read_node_hardware_snapshot,
            )
            .optional()?)
    }
}

fn has_hardware_signal(hardware: &NodeHardwareProfile) -> bool {
    hardware
        .cpu_brand
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
        || hardware.cpu_cores.unwrap_or(0) > 0
        || hardware.memory_total_bytes.unwrap_or(0) > 0
        || !hardware.gpu_names.is_empty()
        || hardware.gpu_memory_total_bytes.unwrap_or(0) > 0
}

fn read_node_hardware_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeHardwareSnapshot> {
    let hardware_json: String = row.get(3)?;
    let hardware = serde_json::from_str::<NodeHardwareProfile>(&hardware_json).unwrap_or_default();
    Ok(NodeHardwareSnapshot {
        node_id: row.get(0)?,
        owner_user_id: row.get(1)?,
        device_name: row.get(2)?,
        hardware,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-node-hardware-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn hardware_snapshot_upsert_keeps_latest_profile() {
        let (store, path) = temp_store();
        let first = NodeHardwareProfile {
            os: Some("windows".to_string()),
            arch: Some("x86_64".to_string()),
            cpu_brand: Some("CPU A".to_string()),
            cpu_cores: Some(8),
            memory_total_bytes: Some(16 * 1024 * 1024 * 1024),
            gpu_names: vec!["GPU A".to_string()],
            gpu_memory_total_bytes: Some(8 * 1024 * 1024 * 1024),
            disk_free_bytes: None,
        };
        store
            .upsert_node_hardware_snapshot("node-a", "user-a", Some("PC-A"), &first)
            .unwrap();

        let second = NodeHardwareProfile {
            cpu_cores: Some(12),
            gpu_names: vec!["GPU B".to_string()],
            ..first.clone()
        };
        store
            .upsert_node_hardware_snapshot("node-a", "user-a", None, &second)
            .unwrap();

        let snapshot = store
            .get_node_hardware_snapshot("node-a")
            .unwrap()
            .expect("snapshot");
        assert_eq!(snapshot.device_name.as_deref(), Some("PC-A"));
        assert_eq!(snapshot.hardware.cpu_cores, Some(12));
        assert_eq!(snapshot.hardware.gpu_names, vec!["GPU B"]);
        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
