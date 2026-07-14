use anyhow::Result;
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::Store;

pub(crate) const ANDROID_DEVICE_LEASE_TTL_SECONDS: i64 = 45;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectAndroidDeviceLease {
    pub(crate) lease_id: String,
    pub(crate) project_id: String,
    pub(crate) hardware_serial: String,
    pub(crate) owner_user_id: String,
    pub(crate) owner_display_name: String,
    pub(crate) client_instance_id: String,
    pub(crate) created_at: String,
    pub(crate) heartbeat_at: String,
    pub(crate) expires_at: String,
}

pub(crate) enum AcquireAndroidDeviceLease {
    Acquired(ProjectAndroidDeviceLease),
    Occupied(ProjectAndroidDeviceLease),
}

impl Store {
    pub(crate) fn list_project_android_device_leases(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAndroidDeviceLease>> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "DELETE FROM project_android_device_leases WHERE expires_at <= ?1",
            [&now],
        )?;
        let mut stmt = conn.prepare(
            "SELECT lease_id, project_id, hardware_serial, owner_user_id,
                    owner_display_name, client_instance_id, created_at, heartbeat_at, expires_at
             FROM project_android_device_leases WHERE project_id = ?1 ORDER BY hardware_serial",
        )?;
        let rows = stmt.query_map([project_id], map_lease)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn acquire_project_android_device_lease(
        &self,
        project_id: &str,
        hardware_serial: &str,
        user_id: &str,
        owner_display_name: &str,
        client_instance_id: &str,
    ) -> Result<AcquireAndroidDeviceLease> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = Utc::now();
        let now_text = now.to_rfc3339();
        let expires_at = (now + Duration::seconds(ANDROID_DEVICE_LEASE_TTL_SECONDS)).to_rfc3339();
        tx.execute(
            "DELETE FROM project_android_device_leases
             WHERE project_id = ?1 AND hardware_serial = ?2 AND expires_at <= ?3",
            params![project_id, hardware_serial, now_text],
        )?;
        let existing = load_lease(&tx, project_id, hardware_serial)?;
        if let Some(existing) = existing {
            if existing.owner_user_id != user_id
                || existing.client_instance_id != client_instance_id
            {
                tx.commit()?;
                return Ok(AcquireAndroidDeviceLease::Occupied(existing));
            }
            tx.execute(
                "UPDATE project_android_device_leases
                 SET heartbeat_at = ?3, expires_at = ?4, owner_display_name = ?5
                 WHERE project_id = ?1 AND hardware_serial = ?2",
                params![
                    project_id,
                    hardware_serial,
                    now_text,
                    expires_at,
                    owner_display_name
                ],
            )?;
        } else {
            tx.execute(
                "INSERT INTO project_android_device_leases (
                   lease_id, project_id, hardware_serial, owner_user_id, owner_display_name,
                   client_instance_id, created_at, heartbeat_at, expires_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8)",
                params![
                    format!("adl_{}", uuid::Uuid::new_v4().simple()),
                    project_id,
                    hardware_serial,
                    user_id,
                    owner_display_name,
                    client_instance_id,
                    now_text,
                    expires_at,
                ],
            )?;
        }
        let lease = load_lease(&tx, project_id, hardware_serial)?.expect("lease was upserted");
        tx.commit()?;
        Ok(AcquireAndroidDeviceLease::Acquired(lease))
    }

    pub(crate) fn heartbeat_project_android_device_lease(
        &self,
        project_id: &str,
        hardware_serial: &str,
        user_id: &str,
        lease_id: &str,
        client_instance_id: &str,
    ) -> Result<Option<ProjectAndroidDeviceLease>> {
        let conn = self.conn()?;
        let now = Utc::now();
        let updated = conn.execute(
            "UPDATE project_android_device_leases SET heartbeat_at = ?6, expires_at = ?7
             WHERE project_id = ?1 AND hardware_serial = ?2 AND owner_user_id = ?3
               AND lease_id = ?4 AND client_instance_id = ?5 AND expires_at > ?6",
            params![
                project_id,
                hardware_serial,
                user_id,
                lease_id,
                client_instance_id,
                now.to_rfc3339(),
                (now + Duration::seconds(ANDROID_DEVICE_LEASE_TTL_SECONDS)).to_rfc3339()
            ],
        )?;
        if updated == 0 {
            return Ok(None);
        }
        load_lease(&conn, project_id, hardware_serial)
    }

    pub(crate) fn release_project_android_device_lease(
        &self,
        project_id: &str,
        hardware_serial: &str,
        user_id: &str,
        lease_id: &str,
        client_instance_id: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        Ok(conn.execute(
            "DELETE FROM project_android_device_leases
             WHERE project_id = ?1 AND hardware_serial = ?2 AND owner_user_id = ?3
               AND lease_id = ?4 AND client_instance_id = ?5",
            params![
                project_id,
                hardware_serial,
                user_id,
                lease_id,
                client_instance_id
            ],
        )? > 0)
    }

    pub(crate) fn validate_project_android_device_lease(
        &self,
        project_id: &str,
        hardware_serial: &str,
        lease_id: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        let valid = conn
            .query_row(
                "SELECT 1 FROM project_android_device_leases
             WHERE project_id = ?1 AND hardware_serial = ?2
               AND lease_id = ?3 AND expires_at > ?4",
                params![project_id, hardware_serial, lease_id, now],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        Ok(valid)
    }
}

fn load_lease(
    conn: &rusqlite::Connection,
    project_id: &str,
    hardware_serial: &str,
) -> Result<Option<ProjectAndroidDeviceLease>> {
    Ok(conn
        .query_row(
            "SELECT lease_id, project_id, hardware_serial, owner_user_id, owner_display_name,
                client_instance_id, created_at, heartbeat_at, expires_at
         FROM project_android_device_leases WHERE project_id = ?1 AND hardware_serial = ?2",
            params![project_id, hardware_serial],
            map_lease,
        )
        .optional()?)
}

fn map_lease(row: &Row<'_>) -> rusqlite::Result<ProjectAndroidDeviceLease> {
    Ok(ProjectAndroidDeviceLease {
        lease_id: row.get(0)?,
        project_id: row.get(1)?,
        hardware_serial: row.get(2)?,
        owner_user_id: row.get(3)?,
        owner_display_name: row.get(4)?,
        client_instance_id: row.get(5)?,
        created_at: row.get(6)?,
        heartbeat_at: row.get(7)?,
        expires_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_is_exclusive_renewable_and_owner_releasable() {
        let path = std::env::temp_dir().join(format!(
            "elon-android-device-lease-{}.db",
            uuid::Uuid::new_v4().simple()
        ));
        let store = Store::open(&path).unwrap();
        let owner = store
            .create_user("lease-owner@example.com", "Password123!", Some("甲"), None)
            .unwrap();
        let other = store
            .create_user("lease-other@example.com", "Password123!", Some("乙"), None)
            .unwrap();
        let project = store
            .create_project(&owner.id, "设备租约测试", None, None)
            .unwrap()
            .project;

        let first = match store
            .acquire_project_android_device_lease(
                &project.id,
                "phone-1",
                &owner.id,
                "甲",
                "client_owner_123",
            )
            .unwrap()
        {
            AcquireAndroidDeviceLease::Acquired(lease) => lease,
            AcquireAndroidDeviceLease::Occupied(_) => panic!("first acquire must succeed"),
        };
        assert!(matches!(
            store
                .acquire_project_android_device_lease(
                    &project.id,
                    "phone-1",
                    &other.id,
                    "乙",
                    "client_other_123",
                )
                .unwrap(),
            AcquireAndroidDeviceLease::Occupied(_)
        ));
        let renewed = match store
            .acquire_project_android_device_lease(
                &project.id,
                "phone-1",
                &owner.id,
                "甲",
                "client_owner_123",
            )
            .unwrap()
        {
            AcquireAndroidDeviceLease::Acquired(lease) => lease,
            AcquireAndroidDeviceLease::Occupied(_) => panic!("owner renewal must succeed"),
        };
        assert_eq!(first.lease_id, renewed.lease_id);
        assert!(!store
            .release_project_android_device_lease(
                &project.id,
                "phone-1",
                &owner.id,
                &first.lease_id,
                "wrong_client_123",
            )
            .unwrap());
        assert!(store
            .release_project_android_device_lease(
                &project.id,
                "phone-1",
                &owner.id,
                &first.lease_id,
                "client_owner_123",
            )
            .unwrap());
        assert!(matches!(
            store
                .acquire_project_android_device_lease(
                    &project.id,
                    "phone-1",
                    &other.id,
                    "乙",
                    "client_other_123",
                )
                .unwrap(),
            AcquireAndroidDeviceLease::Acquired(_)
        ));

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
