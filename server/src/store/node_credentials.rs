use anyhow::Result;
use rusqlite::{params, OptionalExtension, Transaction};

use super::{clean_optional, now, Store};

mod endpoint_authority;

impl Store {
    /// Adopt a legacy node credential for the same owner + device name when the
    /// Windows client has a stable install_id but older rows did not.
    pub fn renew_legacy_node_credential_by_device_name(
        &self,
        owner_user_id: &str,
        install_id: &str,
        new_secret_hash: &str,
        label: Option<&str>,
        device_name: Option<&str>,
    ) -> Result<Option<String>> {
        let install_id = match clean_optional(Some(install_id)) {
            Some(value) => value,
            None => return Ok(None),
        };
        let device_name = match clean_optional(device_name) {
            Some(value) => value,
            None => return Ok(None),
        };
        let label = clean_optional(label);
        let ts = now();

        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        let Some(agent_id) = select_legacy_credential(&tx, owner_user_id, device_name)? else {
            tx.commit()?;
            return Ok(None);
        };

        tx.execute(
            "UPDATE node_credentials
             SET secret_hash = ?3,
                 install_id = ?4,
                 label = COALESCE(NULLIF(label, ''), ?5, ''),
                 device_name = COALESCE(?6, device_name)
             WHERE agent_id = ?1
               AND owner_user_id = ?2",
            params![
                agent_id,
                owner_user_id,
                new_secret_hash,
                install_id,
                label,
                device_name
            ],
        )?;

        merge_legacy_device_duplicates(&tx, owner_user_id, device_name, &agent_id, &ts)?;
        tx.commit()?;
        Ok(Some(agent_id))
    }
}

fn select_legacy_credential(
    tx: &Transaction<'_>,
    owner_user_id: &str,
    device_name: &str,
) -> Result<Option<String>> {
    tx.query_row(
        "SELECT c.agent_id
           FROM node_credentials c
          WHERE c.owner_user_id = ?1
            AND (c.install_id IS NULL OR trim(c.install_id) = '')
            AND lower(trim(c.device_name)) = lower(trim(?2))
          ORDER BY
            (SELECT COUNT(*) FROM projects p
              WHERE p.node_id = c.agent_id OR p.storage_node_id = c.agent_id) DESC,
            c.created_at DESC
          LIMIT 1",
        params![owner_user_id, device_name],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn merge_legacy_device_duplicates(
    tx: &Transaction<'_>,
    owner_user_id: &str,
    device_name: &str,
    canonical_agent_id: &str,
    ts: &str,
) -> Result<()> {
    let old_ids = {
        let mut stmt = tx.prepare(
            "SELECT agent_id
               FROM node_credentials
              WHERE owner_user_id = ?1
                AND agent_id != ?2
                AND (install_id IS NULL OR trim(install_id) = '')
                AND lower(trim(device_name)) = lower(trim(?3))",
        )?;
        let rows = stmt.query_map(
            params![owner_user_id, canonical_agent_id, device_name],
            |row| row.get::<_, String>(0),
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    for old_id in old_ids {
        move_current_node_references(tx, &old_id, canonical_agent_id, ts)?;
        tx.execute(
            "DELETE FROM node_credentials
              WHERE agent_id = ?1
                AND owner_user_id = ?2",
            params![old_id, owner_user_id],
        )?;
    }
    Ok(())
}

fn move_current_node_references(
    tx: &Transaction<'_>,
    old_id: &str,
    new_id: &str,
    ts: &str,
) -> Result<()> {
    tx.execute(
        "UPDATE projects SET node_id = ?2, updated_at = ?3 WHERE node_id = ?1",
        params![old_id, new_id, ts],
    )?;
    tx.execute(
        "UPDATE projects SET storage_node_id = ?2, updated_at = ?3 WHERE storage_node_id = ?1",
        params![old_id, new_id, ts],
    )?;
    tx.execute(
        "UPDATE OR IGNORE project_pc_workspace_bindings
            SET node_id = ?2, updated_at = ?3
          WHERE node_id = ?1",
        params![old_id, new_id, ts],
    )?;
    tx.execute(
        "DELETE FROM project_pc_workspace_bindings WHERE node_id = ?1",
        params![old_id],
    )?;
    tx.execute(
        "UPDATE project_identities
            SET node_id = ?2, updated_at = ?3
          WHERE node_id = ?1",
        params![old_id, new_id, ts],
    )?;
    tx.execute(
        "UPDATE OR IGNORE project_workspace_health_snapshots
            SET node_id = ?2
          WHERE node_id = ?1",
        params![old_id, new_id],
    )?;
    tx.execute(
        "DELETE FROM project_workspace_health_snapshots WHERE node_id = ?1",
        params![old_id],
    )?;
    tx.execute(
        "UPDATE OR IGNORE project_ai_node_authorizations
            SET node_id = ?2, updated_at = ?3
          WHERE node_id = ?1",
        params![old_id, new_id, ts],
    )?;
    tx.execute(
        "DELETE FROM project_ai_node_authorizations WHERE node_id = ?1",
        params![old_id],
    )?;
    tx.execute(
        "UPDATE OR IGNORE project_ai_bots
            SET node_id = ?2, updated_at = ?3
          WHERE node_id = ?1",
        params![old_id, new_id, ts],
    )?;
    tx.execute(
        "DELETE FROM project_ai_bots WHERE node_id = ?1",
        params![old_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::store::Store;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-node-credential-dedupe-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn legacy_device_registration_reuses_and_compresses_credentials() {
        let (store, path) = temp_store();
        let owner = store
            .create_user("node-dedupe-owner@example.com", "secret1", None, None)
            .unwrap();
        store
            .create_node_credential(
                "node-old",
                "old-hash",
                &owner.id,
                Some("ELONQIAN"),
                Some("ELONQIAN"),
                None,
            )
            .unwrap();
        store
            .create_node_credential(
                "node-newer",
                "newer-hash",
                &owner.id,
                Some("ELONQIAN"),
                Some("ELONQIAN"),
                None,
            )
            .unwrap();

        let reused = store
            .renew_legacy_node_credential_by_device_name(
                &owner.id,
                "ins_same",
                "fresh-hash",
                Some("ELONQIAN"),
                Some("ELONQIAN"),
            )
            .unwrap();

        assert_eq!(reused.as_deref(), Some("node-newer"));
        let credentials = store.list_node_credentials(&owner.id).unwrap();
        assert_eq!(credentials.len(), 1);
        assert_eq!(credentials[0].agent_id, "node-newer");
        assert_eq!(credentials[0].install_id.as_deref(), Some("ins_same"));
        assert_eq!(
            store
                .get_node_credential_hash("node-newer")
                .unwrap()
                .as_deref(),
            Some("fresh-hash")
        );

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
