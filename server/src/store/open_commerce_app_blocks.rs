use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row, Transaction};

use crate::{
    open_commerce_app_block_model::{
        normalize_app_block_note, normalize_app_block_reason, BlockOpenCommerceAppRequest,
        OpenCommerceAppBlock, OpenCommerceAppBlockOutcome, APP_BLOCK_STATUS_ACTIVE,
        APP_BLOCK_STATUS_UNBLOCKED,
    },
    open_commerce_model::normalize_app_id,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn block_open_commerce_app(
        &self,
        project_id: &str,
        actor_user_id: &str,
        request: BlockOpenCommerceAppRequest,
    ) -> Result<OpenCommerceAppBlockOutcome> {
        let project_id = project_id.trim();
        let merchant_id = request.merchant_id.trim();
        let requester_app_id = normalize_app_id(&request.requester_app_id)?;
        let reason_code = normalize_app_block_reason(&request.reason_code)?;
        let reason_note = normalize_app_block_note(&request.reason_note)?;
        reject_system_app(&requester_app_id)?;

        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        ensure_block_target(&tx, project_id, merchant_id, &requester_app_id)?;
        let id = tx
            .query_row(
                "SELECT id FROM open_commerce_merchant_app_blocks
                 WHERE merchant_id = ?1 AND requester_app_id = ?2",
                params![merchant_id, requester_app_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| new_id("appblock"));

        tx.execute(
            "INSERT INTO open_commerce_merchant_app_blocks (
               id, project_id, merchant_id, requester_app_id,
               reason_code, reason_note, status, blocked_by_user_id,
               unblocked_by_user_id, blocked_at, unblocked_at, created_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7,
               NULL, ?8, NULL, ?8, ?8
             )
             ON CONFLICT(merchant_id, requester_app_id) DO UPDATE SET
               reason_code = excluded.reason_code,
               reason_note = excluded.reason_note,
               status = 'active',
               blocked_by_user_id = CASE
                 WHEN open_commerce_merchant_app_blocks.status = 'active'
                   THEN open_commerce_merchant_app_blocks.blocked_by_user_id
                 ELSE excluded.blocked_by_user_id
               END,
               blocked_at = CASE
                 WHEN open_commerce_merchant_app_blocks.status = 'active'
                   THEN open_commerce_merchant_app_blocks.blocked_at
                 ELSE excluded.blocked_at
               END,
               unblocked_by_user_id = NULL,
               unblocked_at = NULL,
               updated_at = excluded.updated_at",
            params![
                id,
                project_id,
                merchant_id,
                requester_app_id,
                reason_code,
                reason_note,
                actor_user_id.trim(),
                timestamp
            ],
        )?;
        let revoked_grants = tx.execute(
            "UPDATE open_commerce_grants
                SET revoked_at = ?1, updated_at = ?1
              WHERE merchant_id = ?2 AND grantee_app_id = ?3
                AND revoked_at IS NULL",
            params![timestamp, merchant_id, requester_app_id],
        )?;
        let canceled_authorization_requests = tx.execute(
            "UPDATE open_commerce_authorization_requests
                SET status = 'canceled', decided_by_user_id = ?1,
                    decision_reason = 'merchant_app_blocked', updated_at = ?2
              WHERE merchant_id = ?3 AND requester_app_id = ?4
                AND status = 'pending'",
            params![
                actor_user_id.trim(),
                timestamp,
                merchant_id,
                requester_app_id
            ],
        )?;
        let block = tx.query_row(
            &format!("{APP_BLOCK_SELECT} WHERE id = ?1"),
            params![id],
            app_block_from_row,
        )?;
        tx.commit()?;
        Ok(OpenCommerceAppBlockOutcome {
            block,
            revoked_grants,
            canceled_authorization_requests,
            grants_restored: 0,
        })
    }

    pub(crate) fn unblock_open_commerce_app(
        &self,
        project_id: &str,
        block_id: &str,
        actor_user_id: &str,
    ) -> Result<OpenCommerceAppBlockOutcome> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let current = tx
            .query_row(
                &format!("{APP_BLOCK_SELECT} WHERE project_id = ?1 AND id = ?2"),
                params![project_id.trim(), block_id.trim()],
                app_block_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("App 封禁记录不存在"))?;
        if current.status == APP_BLOCK_STATUS_ACTIVE {
            tx.execute(
                "UPDATE open_commerce_merchant_app_blocks
                    SET status = 'unblocked', unblocked_by_user_id = ?1,
                        unblocked_at = ?2, updated_at = ?2
                  WHERE project_id = ?3 AND id = ?4 AND status = 'active'",
                params![
                    actor_user_id.trim(),
                    timestamp,
                    project_id.trim(),
                    block_id.trim()
                ],
            )?;
        }
        let block = tx.query_row(
            &format!("{APP_BLOCK_SELECT} WHERE project_id = ?1 AND id = ?2"),
            params![project_id.trim(), block_id.trim()],
            app_block_from_row,
        )?;
        tx.commit()?;
        Ok(OpenCommerceAppBlockOutcome {
            block,
            revoked_grants: 0,
            canceled_authorization_requests: 0,
            grants_restored: 0,
        })
    }

    pub(crate) fn list_project_open_commerce_app_blocks(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceAppBlock>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{APP_BLOCK_SELECT} WHERE project_id = ?1
             ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END, updated_at DESC"
        ))?;
        let blocks = stmt
            .query_map(params![project_id.trim()], app_block_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(blocks)
    }

    pub(crate) fn active_open_commerce_app_block(
        &self,
        merchant_id: &str,
        requester_app_id: &str,
    ) -> Result<Option<OpenCommerceAppBlock>> {
        let requester_app_id = normalize_app_id(requester_app_id)?;
        self.conn()?
            .query_row(
                &format!(
                    "{APP_BLOCK_SELECT} WHERE merchant_id = ?1
                     AND requester_app_id = ?2 AND status = ?3"
                ),
                params![
                    merchant_id.trim(),
                    requester_app_id,
                    APP_BLOCK_STATUS_ACTIVE
                ],
                app_block_from_row,
            )
            .optional()
            .map_err(Into::into)
    }
}

fn ensure_block_target(
    tx: &Transaction<'_>,
    project_id: &str,
    merchant_id: &str,
    requester_app_id: &str,
) -> Result<()> {
    let merchant_project_id = tx
        .query_row(
            "SELECT project_id FROM open_commerce_merchants WHERE id = ?1",
            params![merchant_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("商户节点不存在"))?;
    if merchant_project_id != project_id {
        bail!("商户节点不属于当前项目");
    }
    let app_exists = tx
        .query_row(
            "SELECT 1 FROM open_commerce_developer_apps WHERE app_id = ?1",
            params![requester_app_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !app_exists {
        bail!("开发者应用不存在");
    }
    Ok(())
}

fn reject_system_app(requester_app_id: &str) -> Result<()> {
    if matches!(requester_app_id, "pc-web" | "mcp-client") {
        bail!("共享系统入口不能按 App 整体封禁，请撤回目录或停用具体商业能力");
    }
    Ok(())
}

fn app_block_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceAppBlock> {
    let status = row.get::<_, String>(6)?;
    debug_assert!(matches!(
        status.as_str(),
        APP_BLOCK_STATUS_ACTIVE | APP_BLOCK_STATUS_UNBLOCKED
    ));
    Ok(OpenCommerceAppBlock {
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        requester_app_id: row.get(3)?,
        reason_code: row.get(4)?,
        reason_note: row.get(5)?,
        status,
        blocked_by_user_id: row.get(7)?,
        unblocked_by_user_id: row.get(8)?,
        blocked_at: row.get(9)?,
        unblocked_at: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

const APP_BLOCK_SELECT: &str = "SELECT id, project_id, merchant_id, requester_app_id,
       reason_code, reason_note, status, blocked_by_user_id, unblocked_by_user_id,
       blocked_at, unblocked_at, created_at, updated_at
  FROM open_commerce_merchant_app_blocks";
