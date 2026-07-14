use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};

use super::Store;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectAndroidDevice {
    pub(crate) project_id: String,
    pub(crate) hardware_serial: String,
    pub(crate) display_name: String,
    pub(crate) manufacturer: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) android_sdk: Option<u32>,
    pub(crate) android_release: Option<String>,
    pub(crate) last_endpoint: String,
    pub(crate) wireless_mode: String,
    pub(crate) updated_by_user_id: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl Store {
    pub(crate) fn upsert_project_android_device(
        &self,
        project_id: &str,
        user_id: &str,
        device: &ProjectAndroidDevice,
    ) -> Result<ProjectAndroidDevice> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO project_android_devices (
               project_id, hardware_serial, display_name, manufacturer, model,
               android_sdk, android_release, last_endpoint, wireless_mode,
               updated_by_user_id, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
             ON CONFLICT(project_id, hardware_serial) DO UPDATE SET
               display_name = excluded.display_name,
               manufacturer = excluded.manufacturer,
               model = excluded.model,
               android_sdk = excluded.android_sdk,
               android_release = excluded.android_release,
               last_endpoint = excluded.last_endpoint,
               wireless_mode = excluded.wireless_mode,
               updated_by_user_id = excluded.updated_by_user_id,
               updated_at = excluded.updated_at",
            params![
                project_id,
                device.hardware_serial,
                device.display_name,
                device.manufacturer,
                device.model,
                device.android_sdk,
                device.android_release,
                device.last_endpoint,
                device.wireless_mode,
                user_id,
                now,
            ],
        )?;
        load_project_android_device(&conn, project_id, &device.hardware_serial)
    }

    pub(crate) fn list_project_android_devices(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAndroidDevice>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT project_id, hardware_serial, display_name, manufacturer, model,
                    android_sdk, android_release, last_endpoint, wireless_mode,
                    updated_by_user_id, created_at, updated_at
             FROM project_android_devices WHERE project_id = ?1
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([project_id], map_device)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn delete_project_android_device(
        &self,
        project_id: &str,
        hardware_serial: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM project_android_device_leases WHERE project_id = ?1 AND hardware_serial = ?2",
            params![project_id, hardware_serial],
        )?;
        let removed = tx.execute(
            "DELETE FROM project_android_devices WHERE project_id = ?1 AND hardware_serial = ?2",
            params![project_id, hardware_serial],
        )? > 0;
        tx.commit()?;
        Ok(removed)
    }
}

fn load_project_android_device(
    conn: &rusqlite::Connection,
    project_id: &str,
    hardware_serial: &str,
) -> Result<ProjectAndroidDevice> {
    Ok(conn.query_row(
        "SELECT project_id, hardware_serial, display_name, manufacturer, model,
                android_sdk, android_release, last_endpoint, wireless_mode,
                updated_by_user_id, created_at, updated_at
         FROM project_android_devices WHERE project_id = ?1 AND hardware_serial = ?2",
        params![project_id, hardware_serial],
        map_device,
    )?)
}

fn map_device(row: &Row<'_>) -> rusqlite::Result<ProjectAndroidDevice> {
    Ok(ProjectAndroidDevice {
        project_id: row.get(0)?,
        hardware_serial: row.get(1)?,
        display_name: row.get(2)?,
        manufacturer: row.get(3)?,
        model: row.get(4)?,
        android_sdk: row.get(5)?,
        android_release: row.get(6)?,
        last_endpoint: row.get(7)?,
        wireless_mode: row.get(8)?,
        updated_by_user_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}
