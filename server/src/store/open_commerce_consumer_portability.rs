use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::{
    open_commerce_consumer_preference_model::{
        ConsumerPreferenceDisclosure, ConsumerPreferenceProfile,
    },
    open_commerce_data_request_model::OpenCommerceConsumerDataRequest,
    open_commerce_model::OpenCommerceInvocation,
    open_commerce_portability_model::{
        ConsumerPortabilityExport, ConsumerPortabilityPayload, ConsumerRelationshipRenewalLink,
    },
    open_commerce_relationship_model::OpenCommerceConsumerRelationship,
};

use super::{
    new_id, now,
    open_commerce_consumer_data_requests::{data_request_from_row, DATA_REQUEST_SELECT},
    open_commerce_consumer_preferences::{
        preference_disclosure_from_row, preference_profile_from_row,
    },
    open_commerce_consumer_relationships::relationship_from_row,
    open_commerce_invocations::{invocation_from_row, INVOCATION_SELECT},
    Store,
};

pub(crate) const MAX_CONSUMER_PORTABILITY_RECORDS: usize = 5_000;

pub(crate) struct ConsumerPortabilitySnapshotSources {
    pub relationships: Vec<OpenCommerceConsumerRelationship>,
    pub relationship_renewals: Vec<ConsumerRelationshipRenewalLink>,
    pub data_requests: Vec<OpenCommerceConsumerDataRequest>,
    pub preference_profile: Option<ConsumerPreferenceProfile>,
    pub preference_disclosures: Vec<ConsumerPreferenceDisclosure>,
    pub terminal_invocations: Vec<OpenCommerceInvocation>,
}

impl Store {
    pub(crate) fn consumer_portability_snapshot_sources(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
    ) -> Result<ConsumerPortabilitySnapshotSources> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let mut relationship_stmt = tx.prepare(&format!(
            "{PORTABILITY_RELATIONSHIP_SELECT}
              WHERE consumer_project_id=?1 AND consumer_user_id=?2
              ORDER BY created_at ASC, rowid ASC LIMIT ?3"
        ))?;
        let relationship_rows = relationship_stmt.query_map(
            params![
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                (MAX_CONSUMER_PORTABILITY_RECORDS + 1) as i64
            ],
            |row| {
                Ok((
                    relationship_from_row(row)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            },
        )?;
        let relationship_rows = relationship_rows.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(relationship_stmt);
        if relationship_rows.len() > MAX_CONSUMER_PORTABILITY_RECORDS {
            bail!("消费者关系记录超过单个导出包的 5000 条上限");
        }
        let mut relationships = Vec::with_capacity(relationship_rows.len());
        let mut relationship_renewals = Vec::new();
        for (relationship, renewed_from) in relationship_rows {
            if let Some(source_relationship_id) = renewed_from {
                relationship_renewals.push(ConsumerRelationshipRenewalLink {
                    source_relationship_id,
                    renewed_relationship_id: relationship.id.clone(),
                });
            }
            relationships.push(relationship);
        }

        let mut request_stmt = tx.prepare(&format!(
            "{DATA_REQUEST_SELECT}
              WHERE consumer_project_id=?1 AND consumer_user_id=?2
              ORDER BY requested_at ASC, rowid ASC LIMIT ?3"
        ))?;
        let request_rows = request_stmt.query_map(
            params![
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                (MAX_CONSUMER_PORTABILITY_RECORDS + 1) as i64
            ],
            data_request_from_row,
        )?;
        let data_requests = request_rows.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(request_stmt);
        if data_requests.len() > MAX_CONSUMER_PORTABILITY_RECORDS {
            bail!("消费者数据请求超过单个导出包的 5000 条上限");
        }

        let preference_profile = tx
            .query_row(
                "SELECT preferences_json, revision, created_at, updated_at
                   FROM open_commerce_consumer_preference_profiles
                  WHERE consumer_project_id=?1 AND consumer_user_id=?2",
                params![consumer_project_id.trim(), consumer_user_id.trim()],
                preference_profile_from_row,
            )
            .optional()?;
        let mut disclosure_stmt = tx.prepare(
            "SELECT disclosure.relationship_id, relationship.merchant_id,
                    relationship.subject_alias, relationship.status,
                    relationship.expires_at, disclosure.shared_fields_json,
                    disclosure.disclosure_json, disclosure.profile_revision,
                    disclosure.created_at, disclosure.updated_at
               FROM open_commerce_consumer_preference_disclosures disclosure
               JOIN open_commerce_consumer_relationships relationship
                 ON relationship.id=disclosure.relationship_id
              WHERE relationship.consumer_project_id=?1
                AND relationship.consumer_user_id=?2
              ORDER BY disclosure.created_at ASC, disclosure.rowid ASC
              LIMIT ?3",
        )?;
        let disclosure_rows = disclosure_stmt.query_map(
            params![
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                (MAX_CONSUMER_PORTABILITY_RECORDS + 1) as i64,
            ],
            preference_disclosure_from_row,
        )?;
        let preference_disclosures = disclosure_rows.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(disclosure_stmt);
        if preference_disclosures.len() > MAX_CONSUMER_PORTABILITY_RECORDS {
            bail!("消费者偏好披露超过单个 V2 导出包的 5000 条上限");
        }
        let mut invocation_stmt = tx.prepare(&format!(
            "{INVOCATION_SELECT}
              WHERE requester_user_id=?1 AND status IN ('succeeded', 'failed')
              ORDER BY created_at ASC, rowid ASC LIMIT ?2"
        ))?;
        let invocation_rows = invocation_stmt.query_map(
            params![
                consumer_user_id.trim(),
                (MAX_CONSUMER_PORTABILITY_RECORDS + 1) as i64,
            ],
            invocation_from_row,
        )?;
        let terminal_invocations = invocation_rows.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(invocation_stmt);
        if terminal_invocations.len() > MAX_CONSUMER_PORTABILITY_RECORDS {
            bail!("消费者调用凭证超过单个 V3 导出包的 5000 条上限");
        }
        tx.commit()?;
        Ok(ConsumerPortabilitySnapshotSources {
            relationships,
            relationship_renewals,
            data_requests,
            preference_profile,
            preference_disclosures,
            terminal_invocations,
        })
    }

    pub(crate) fn consumer_portability_export_by_key(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<ConsumerPortabilityExport>> {
        self.conn()?
            .query_row(
                &format!(
                    "{PORTABILITY_EXPORT_SELECT}
                      WHERE consumer_project_id=?1 AND consumer_user_id=?2
                        AND idempotency_key=?3"
                ),
                params![
                    consumer_project_id.trim(),
                    consumer_user_id.trim(),
                    idempotency_key.trim()
                ],
                portability_export_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn save_consumer_portability_export(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        idempotency_key: &str,
        package_schema: &str,
        payload_json: &str,
        payload_sha256: &str,
    ) -> Result<(ConsumerPortabilityExport, bool)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                &format!(
                    "{PORTABILITY_EXPORT_SELECT}
                      WHERE consumer_project_id=?1 AND consumer_user_id=?2
                        AND idempotency_key=?3"
                ),
                params![
                    consumer_project_id.trim(),
                    consumer_user_id.trim(),
                    idempotency_key.trim()
                ],
                portability_export_from_row,
            )
            .optional()?;
        if let Some(existing) = existing {
            tx.commit()?;
            return Ok((existing, false));
        }
        let id = new_id("portability");
        let timestamp = now();
        tx.execute(
            "INSERT INTO open_commerce_consumer_portability_exports (
               id, consumer_project_id, consumer_user_id, idempotency_key,
               package_schema, payload_json, payload_sha256, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                idempotency_key.trim(),
                package_schema,
                payload_json,
                payload_sha256,
                timestamp,
            ],
        )?;
        tx.commit()?;
        drop(conn);
        Ok((
            self.consumer_portability_export(consumer_project_id, consumer_user_id, &id)?
                .ok_or_else(|| anyhow!("消费者可移植数据包不存在"))?,
            true,
        ))
    }

    pub(crate) fn list_consumer_portability_exports(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ConsumerPortabilityExport>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{PORTABILITY_EXPORT_SELECT}
              WHERE consumer_project_id=?1 AND consumer_user_id=?2
              ORDER BY created_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                consumer_project_id.trim(),
                consumer_user_id.trim(),
                limit.clamp(1, 100) as i64
            ],
            portability_export_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn consumer_portability_export(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        export_id: &str,
    ) -> Result<Option<ConsumerPortabilityExport>> {
        self.conn()?
            .query_row(
                &format!(
                    "{PORTABILITY_EXPORT_SELECT}
                      WHERE id=?1 AND consumer_project_id=?2 AND consumer_user_id=?3"
                ),
                params![
                    export_id.trim(),
                    consumer_project_id.trim(),
                    consumer_user_id.trim()
                ],
                portability_export_from_row,
            )
            .optional()
            .map_err(Into::into)
    }
}

fn portability_export_from_row(row: &Row<'_>) -> rusqlite::Result<ConsumerPortabilityExport> {
    let source_project_id: String = row.get(1)?;
    let payload_json: String = row.get(4)?;
    let payload: ConsumerPortabilityPayload =
        serde_json::from_str(&payload_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                payload_json.len(),
                rusqlite::types::Type::Text,
                error.into(),
            )
        })?;
    if payload.source_project_id != source_project_id {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            payload_json.len(),
            rusqlite::types::Type::Text,
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "导出包来源项目与所有者记录不一致",
            )
            .into(),
        ));
    }
    Ok(ConsumerPortabilityExport {
        id: row.get(0)?,
        source_project_id,
        idempotency_key: row.get(2)?,
        schema: row.get(3)?,
        payload_json,
        payload,
        payload_sha256: row.get(5)?,
        created_at: row.get(6)?,
    })
}

const PORTABILITY_RELATIONSHIP_SELECT: &str = "SELECT id, merchant_id, source_app_id,
       subject_alias, scopes_json, purpose, status,
       expires_at, revoked_at, created_at, updated_at,
       renewed_from_relationship_id
  FROM open_commerce_consumer_relationships";

const PORTABILITY_EXPORT_SELECT: &str = "SELECT id, consumer_project_id,
       idempotency_key, package_schema, payload_json, payload_sha256, created_at
  FROM open_commerce_consumer_portability_exports";
