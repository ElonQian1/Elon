use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::open_commerce_relationship_model::OpenCommerceConsumerRelationship;

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn replace_open_commerce_consumer_relationship(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        merchant_project_id: &str,
        merchant_id: &str,
        source_app_id: &str,
        scopes: &[String],
        purpose: &str,
        expires_at: &str,
    ) -> Result<OpenCommerceConsumerRelationship> {
        let id = new_id("relationship");
        let subject_alias = new_id("subject");
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE open_commerce_consumer_relationships
                SET status='revoked', revoked_at=?1, updated_at=?1
              WHERE consumer_user_id=?2 AND merchant_id=?3 AND status='active'",
            params![timestamp, consumer_user_id, merchant_id],
        )?;
        tx.execute(
            "INSERT INTO open_commerce_consumer_relationships (
               id, consumer_project_id, consumer_user_id, merchant_project_id,
               merchant_id, source_app_id, subject_alias, scopes_json, purpose,
               status, expires_at, revoked_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                       'active', ?10, NULL, ?11, ?11)",
            params![
                id,
                consumer_project_id,
                consumer_user_id,
                merchant_project_id,
                merchant_id,
                source_app_id,
                subject_alias,
                serde_json::to_string(scopes)?,
                purpose,
                expires_at,
                timestamp,
            ],
        )?;
        tx.commit()?;
        drop(conn);
        self.open_commerce_consumer_relationship(&id)
    }

    pub(crate) fn list_open_commerce_consumer_relationships(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceConsumerRelationship>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{RELATIONSHIP_SELECT}
              WHERE consumer_project_id=?1 AND consumer_user_id=?2
              ORDER BY updated_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                consumer_project_id,
                consumer_user_id,
                limit.clamp(1, 200) as i64
            ],
            relationship_from_row,
        )?;
        let relationships = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(relationships)
    }

    pub(crate) fn list_open_commerce_merchant_relationships(
        &self,
        merchant_project_id: &str,
        merchant_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceConsumerRelationship>> {
        self.open_commerce_merchant_for_project(merchant_project_id, merchant_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{RELATIONSHIP_SELECT}
              WHERE merchant_project_id=?1 AND merchant_id=?2
              ORDER BY updated_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![merchant_project_id, merchant_id, limit.clamp(1, 200) as i64],
            relationship_from_row,
        )?;
        let relationships = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(relationships)
    }

    pub(crate) fn revoke_open_commerce_consumer_relationship(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        relationship_id: &str,
    ) -> Result<OpenCommerceConsumerRelationship> {
        let current = self
            .consumer_owned_open_commerce_relationship(
                consumer_project_id,
                consumer_user_id,
                relationship_id,
            )?
            .ok_or_else(|| anyhow!("消费者关系凭证不存在"))?;
        if current.status != "revoked" {
            let timestamp = now();
            self.conn()?.execute(
                "UPDATE open_commerce_consumer_relationships
                    SET status='revoked', revoked_at=?1, updated_at=?1
                  WHERE id=?2 AND consumer_project_id=?3 AND consumer_user_id=?4
                    AND status='active'",
                params![
                    timestamp,
                    relationship_id,
                    consumer_project_id,
                    consumer_user_id
                ],
            )?;
        }
        self.open_commerce_consumer_relationship(relationship_id)
    }

    fn consumer_owned_open_commerce_relationship(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        relationship_id: &str,
    ) -> Result<Option<OpenCommerceConsumerRelationship>> {
        self.conn()?
            .query_row(
                &format!(
                    "{RELATIONSHIP_SELECT}
                      WHERE id=?1 AND consumer_project_id=?2 AND consumer_user_id=?3"
                ),
                params![relationship_id, consumer_project_id, consumer_user_id],
                relationship_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    fn open_commerce_consumer_relationship(
        &self,
        relationship_id: &str,
    ) -> Result<OpenCommerceConsumerRelationship> {
        self.conn()?
            .query_row(
                &format!("{RELATIONSHIP_SELECT} WHERE id=?1"),
                params![relationship_id],
                relationship_from_row,
            )
            .map_err(|error| anyhow!(error).context("消费者关系凭证不存在"))
    }
}

fn relationship_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceConsumerRelationship> {
    let scopes_json: String = row.get(4)?;
    let scopes = serde_json::from_str(&scopes_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            scopes_json.len(),
            rusqlite::types::Type::Text,
            error.into(),
        )
    })?;
    let stored_status: String = row.get(6)?;
    let expires_at: String = row.get(7)?;
    let status = if stored_status == "revoked" {
        "revoked"
    } else if DateTime::parse_from_rfc3339(&expires_at)
        .map(|value| value.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(true)
    {
        "expired"
    } else {
        "active"
    };
    Ok(OpenCommerceConsumerRelationship {
        id: row.get(0)?,
        merchant_id: row.get(1)?,
        source_app_id: row.get(2)?,
        subject_alias: row.get(3)?,
        scopes,
        purpose: row.get(5)?,
        status: status.to_string(),
        expires_at,
        revoked_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

const RELATIONSHIP_SELECT: &str = "SELECT id, merchant_id, source_app_id,
       subject_alias, scopes_json, purpose, status,
       expires_at, revoked_at, created_at, updated_at
  FROM open_commerce_consumer_relationships";
