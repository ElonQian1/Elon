use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, Row, Transaction};
use serde_json::{json, Value};

use crate::{
    open_commerce_capability_source_model::{
        LinkCapabilitySourceRequest, OpenCommerceCapabilitySourceLink,
    },
    open_commerce_integration_model::{normalize_string_list, INTEGRATION_STATUS_DISABLED},
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn link_open_commerce_capability_source_with_audit(
        &self,
        project_id: &str,
        capability_id: &str,
        linked_by_user_id: &str,
        actor_app_id: &str,
        request: LinkCapabilitySourceRequest,
    ) -> Result<OpenCommerceCapabilitySourceLink> {
        let project_id = project_id.trim();
        let capability_id = capability_id.trim();
        let integration_id = request.integration_id.trim();
        let sync_receipt_id = request.sync_receipt_id.trim();
        let data_domain = normalize_string_list(&[request.data_domain], "数据域", 1)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("数据域不能为空"))?;
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let (merchant_id, capability_version) = tx
            .query_row(
                "SELECT c.merchant_id, c.version
                   FROM open_commerce_capabilities c
                   JOIN open_commerce_merchants m ON m.id = c.merchant_id
                  WHERE c.id = ?1 AND m.project_id = ?2",
                params![capability_id, project_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .with_context(|| "当前项目中不存在该商业能力")?;
        let (integration_merchant_id, integration_status, data_domains_json) = tx
            .query_row(
                "SELECT merchant_id, status, data_domains_json
                   FROM open_commerce_integrations
                  WHERE id = ?1 AND project_id = ?2",
                params![integration_id, project_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .with_context(|| "当前项目中不存在该数据接入")?;
        if merchant_id != integration_merchant_id {
            bail!("商业能力和数据接入必须属于同一商户");
        }
        if integration_status == INTEGRATION_STATUS_DISABLED {
            bail!("已停用的数据接入不能绑定公开能力来源");
        }
        let data_domains = serde_json::from_str::<Vec<String>>(&data_domains_json)
            .with_context(|| "数据接入的数据域 JSON 无效")?;
        if !data_domains.contains(&data_domain) {
            bail!("所选数据域未在该数据接入中登记");
        }
        let receipt = tx
            .query_row(
                "SELECT integration_id, sync_kind, status
                    FROM open_commerce_sync_receipts
                   WHERE id = ?1 AND project_id = ?2",
                params![sync_receipt_id, project_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .with_context(|| "当前项目中不存在该同步回执")?;
        if receipt.0 != integration_id {
            bail!("同步回执不属于所选数据接入");
        }
        if receipt.1 == "health_check" {
            bail!("健康检查回执不能作为公开业务数据来源");
        }
        if !matches!(receipt.2.as_str(), "succeeded" | "partial") {
            bail!("只有成功或部分成功的同步回执可以绑定公开能力来源");
        }
        let timestamp = now();
        tx.execute(
            "INSERT INTO open_commerce_capability_source_links (
               id, project_id, merchant_id, capability_id, capability_version,
               integration_id, sync_receipt_id, data_domain, revision,
               linked_by_user_id, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?10)
             ON CONFLICT(capability_id) DO UPDATE SET
               capability_version = excluded.capability_version,
               integration_id = excluded.integration_id,
               sync_receipt_id = excluded.sync_receipt_id,
               data_domain = excluded.data_domain,
               revision = open_commerce_capability_source_links.revision + 1,
               linked_by_user_id = excluded.linked_by_user_id,
               updated_at = excluded.updated_at
             WHERE open_commerce_capability_source_links.project_id = excluded.project_id
               AND open_commerce_capability_source_links.merchant_id = excluded.merchant_id",
            params![
                new_id("capability_source"),
                project_id,
                merchant_id,
                capability_id,
                capability_version,
                integration_id,
                sync_receipt_id,
                data_domain,
                linked_by_user_id.trim(),
                timestamp
            ],
        )?;
        let link = open_commerce_capability_source_link_on(&tx, project_id, capability_id)?;
        insert_source_audit(
            &tx,
            project_id,
            linked_by_user_id,
            actor_app_id,
            "capability.source_linked",
            "capability_source_link",
            &link.id,
            &json!({
                "merchant_id": link.merchant_id,
                "capability_id": link.capability_id,
                "capability_version": link.capability_version,
                "integration_id": link.integration_id,
                "sync_receipt_id": link.sync_receipt_id,
                "data_domain": link.data_domain,
                "revision": link.revision,
                "externally_verified": false
            }),
        )?;
        tx.commit()?;
        Ok(link)
    }

    pub(crate) fn remove_open_commerce_capability_source_link_with_audit(
        &self,
        project_id: &str,
        capability_id: &str,
        actor_user_id: &str,
        actor_app_id: &str,
    ) -> Result<bool> {
        let project_id = project_id.trim();
        let capability_id = capability_id.trim();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let capability_exists = tx.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM open_commerce_capabilities c
               JOIN open_commerce_merchants m ON m.id = c.merchant_id
              WHERE c.id = ?1 AND m.project_id = ?2
             )",
            params![capability_id, project_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !capability_exists {
            bail!("当前项目中不存在该商业能力");
        }
        let removed = tx.execute(
            "DELETE FROM open_commerce_capability_source_links
              WHERE project_id = ?1 AND capability_id = ?2",
            params![project_id, capability_id],
        )? > 0;
        if removed {
            insert_source_audit(
                &tx,
                project_id,
                actor_user_id,
                actor_app_id,
                "capability.source_unlinked",
                "capability",
                capability_id,
                &json!({"externally_verified": false}),
            )?;
        }
        tx.commit()?;
        Ok(removed)
    }

    pub(crate) fn list_project_open_commerce_capability_source_links(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceCapabilitySourceLink>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{CAPABILITY_SOURCE_SELECT} WHERE l.project_id = ?1 ORDER BY l.updated_at DESC"
        ))?;
        let rows = stmt
            .query_map(params![project_id.trim()], capability_source_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub(crate) fn list_publishable_open_commerce_capability_source_links(
        &self,
        merchant_id: &str,
    ) -> Result<Vec<OpenCommerceCapabilitySourceLink>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{CAPABILITY_SOURCE_SELECT} WHERE l.merchant_id = ?1 ORDER BY l.updated_at DESC"
        ))?;
        let links = stmt
            .query_map(params![merchant_id.trim()], capability_source_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(links.into_iter().filter(|link| link.publishable).collect())
    }
}

fn open_commerce_capability_source_link_on(
    conn: &Connection,
    project_id: &str,
    capability_id: &str,
) -> Result<OpenCommerceCapabilitySourceLink> {
    conn.query_row(
        &format!("{CAPABILITY_SOURCE_SELECT} WHERE l.project_id = ?1 AND l.capability_id = ?2"),
        params![project_id.trim(), capability_id.trim()],
        capability_source_from_row,
    )
    .map_err(|error| anyhow!(error).context("商业能力来源绑定不存在"))
}

#[allow(clippy::too_many_arguments)]
fn insert_source_audit(
    tx: &Transaction<'_>,
    project_id: &str,
    actor_user_id: &str,
    actor_app_id: &str,
    action: &str,
    subject_type: &str,
    subject_id: &str,
    metadata: &Value,
) -> Result<()> {
    let metadata_json = serde_json::to_string(metadata)?;
    tx.execute(
        "INSERT INTO open_commerce_audit_events (
           id, project_id, actor_user_id, actor_app_id, action,
           subject_type, subject_id, metadata_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            new_id("audit"),
            project_id.trim(),
            actor_user_id.trim(),
            actor_app_id.trim(),
            action,
            subject_type,
            subject_id.trim(),
            metadata_json,
            now()
        ],
    )?;
    Ok(())
}

fn capability_source_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceCapabilitySourceLink> {
    let capability_version = row.get::<_, i64>(5)?;
    let current_capability_version = row.get::<_, i64>(6)?;
    let integration_status = row.get::<_, String>(12)?;
    let sync_kind = row.get::<_, String>(13)?;
    let receipt_status = row.get::<_, String>(14)?;
    let blocking_reason = if capability_version != current_capability_version {
        Some("capability_version_changed".to_string())
    } else if integration_status == INTEGRATION_STATUS_DISABLED {
        Some("integration_disabled".to_string())
    } else if sync_kind == "health_check" {
        Some("health_check_not_business_data".to_string())
    } else if !matches!(receipt_status.as_str(), "succeeded" | "partial") {
        Some("receipt_not_eligible".to_string())
    } else {
        None
    };
    Ok(OpenCommerceCapabilitySourceLink {
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        capability_id: row.get(3)?,
        capability_key: row.get(4)?,
        capability_version,
        current_capability_version,
        integration_id: row.get(7)?,
        sync_receipt_id: row.get(8)?,
        data_domain: row.get(9)?,
        provider_key: row.get(10)?,
        connection_mode: row.get(11)?,
        integration_status,
        sync_kind,
        receipt_status,
        receipt_sha256: row.get(15)?,
        receipt_completed_at: row.get(16)?,
        revision: row.get(17)?,
        linked_by_user_id: row.get(18)?,
        created_at: row.get(19)?,
        updated_at: row.get(20)?,
        publishable: blocking_reason.is_none(),
        blocking_reason,
    })
}

const CAPABILITY_SOURCE_SELECT: &str =
    "SELECT l.id, l.project_id, l.merchant_id, l.capability_id, c.capability_key,
            l.capability_version, c.version, l.integration_id, l.sync_receipt_id,
            l.data_domain, i.provider_key, i.connection_mode, i.status,
            r.sync_kind, r.status, r.receipt_fingerprint, r.completed_at,
            l.revision, l.linked_by_user_id, l.created_at, l.updated_at
       FROM open_commerce_capability_source_links l
       JOIN open_commerce_capabilities c ON c.id = l.capability_id
       JOIN open_commerce_integrations i ON i.id = l.integration_id
       JOIN open_commerce_sync_receipts r ON r.id = l.sync_receipt_id";
