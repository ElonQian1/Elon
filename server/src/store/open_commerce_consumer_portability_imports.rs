use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::{
    open_commerce_portability_import_model::{
        ConsumerPortabilityImport, ConsumerPortabilityPackageSignature,
        VerifiedConsumerPortabilitySignature, CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS,
        CONSUMER_PORTABILITY_IMPORT_SCHEMA, CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS,
        CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS,
    },
    open_commerce_portability_model::ConsumerPortabilityExport,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn save_consumer_portability_import(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        source_operator: &str,
        package: &ConsumerPortabilityExport,
        package_json: &str,
        envelope_sha256: &str,
        verified_signature: Option<&VerifiedConsumerPortabilitySignature>,
    ) -> Result<(ConsumerPortabilityImport, bool, bool)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                &format!(
                    "{PORTABILITY_IMPORT_SELECT}
                      WHERE destination_project_id=?1 AND consumer_user_id=?2
                        AND envelope_sha256=?3"
                ),
                params![
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                    envelope_sha256,
                ],
                portability_import_from_row,
            )
            .optional()?;
        if let Some(existing) = existing {
            let trust_upgraded =
                if existing.trust_status == CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS {
                    if let Some(proof) = verified_signature {
                        tx.execute(
                            "UPDATE open_commerce_consumer_portability_imports
                            SET trust_status=?2, signer_key_record_id=?3,
                                signature_algorithm=?4, signer_key_id=?5,
                                signature_base64=?6, signature_verified_at=?7
                          WHERE id=?1 AND trust_status=?8",
                            params![
                                existing.id,
                                CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS,
                                proof.key_record_id,
                                proof.signature.algorithm,
                                proof.signature.key_id,
                                proof.signature.signature_base64,
                                proof.verified_at,
                                CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS,
                            ],
                        )? > 0
                    } else {
                        false
                    }
                } else {
                    false
                };
            let existing = if trust_upgraded {
                tx.query_row(
                    &format!("{PORTABILITY_IMPORT_SELECT} WHERE id=?1"),
                    params![existing.id],
                    portability_import_from_row,
                )?
            } else {
                existing
            };
            tx.commit()?;
            return Ok((existing, false, trust_upgraded));
        }

        let id = new_id("portability-import");
        let imported_at = now();
        tx.execute(
            "INSERT INTO open_commerce_consumer_portability_imports (
               id, destination_project_id, consumer_user_id, source_operator,
               source_project_id, source_package_id, source_package_schema,
               envelope_sha256, payload_sha256, package_json, imported_at,
               trust_status, signer_key_record_id, signature_algorithm,
               signer_key_id, signature_base64, signature_verified_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                       ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                id,
                destination_project_id.trim(),
                consumer_user_id.trim(),
                source_operator,
                package.source_project_id,
                package.id,
                package.schema,
                envelope_sha256,
                package.payload_sha256,
                package_json,
                imported_at,
                verified_signature
                    .map(|_| CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS)
                    .unwrap_or(CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS),
                verified_signature.map(|value| value.key_record_id.as_str()),
                verified_signature.map(|value| value.signature.algorithm.as_str()),
                verified_signature.map(|value| value.signature.key_id.as_str()),
                verified_signature.map(|value| value.signature.signature_base64.as_str()),
                verified_signature.map(|value| value.verified_at.as_str()),
            ],
        )?;
        tx.commit()?;
        drop(conn);
        let saved = self
            .consumer_portability_import(destination_project_id, consumer_user_id, &id)?
            .ok_or_else(|| anyhow!("消费者外部数据包导入记录不存在"))?;
        Ok((saved, true, false))
    }

    pub(crate) fn list_consumer_portability_imports(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ConsumerPortabilityImport>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{PORTABILITY_IMPORT_SELECT}
              WHERE destination_project_id=?1 AND consumer_user_id=?2
              ORDER BY imported_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                destination_project_id.trim(),
                consumer_user_id.trim(),
                limit.clamp(1, 100) as i64,
            ],
            portability_import_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn consumer_portability_import(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        import_id: &str,
    ) -> Result<Option<ConsumerPortabilityImport>> {
        self.conn()?
            .query_row(
                &format!(
                    "{PORTABILITY_IMPORT_SELECT}
                      WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3"
                ),
                params![
                    import_id.trim(),
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                ],
                portability_import_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn delete_consumer_portability_import(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        import_id: &str,
    ) -> Result<Option<ConsumerPortabilityImport>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                &format!(
                    "{PORTABILITY_IMPORT_SELECT}
                      WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3"
                ),
                params![
                    import_id.trim(),
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                ],
                portability_import_from_row,
            )
            .optional()?;
        if existing.is_some() {
            tx.execute(
                "DELETE FROM open_commerce_consumer_portability_imports
                  WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3",
                params![
                    import_id.trim(),
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                ],
            )?;
        }
        tx.commit()?;
        Ok(existing)
    }
}

fn portability_import_from_row(row: &Row<'_>) -> rusqlite::Result<ConsumerPortabilityImport> {
    let package_json: String = row.get(8)?;
    let package: ConsumerPortabilityExport =
        serde_json::from_str(&package_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                package_json.len(),
                rusqlite::types::Type::Text,
                error.into(),
            )
        })?;
    let source_project_id: String = row.get(3)?;
    let source_package_id: String = row.get(4)?;
    let source_package_schema: String = row.get(5)?;
    let payload_sha256: String = row.get(7)?;
    if package.source_project_id != source_project_id
        || package.id != source_package_id
        || package.schema != source_package_schema
        || package.payload_sha256 != payload_sha256
    {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            package_json.len(),
            rusqlite::types::Type::Text,
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "导入记录元数据与数据包不一致",
            )
            .into(),
        ));
    }
    Ok(ConsumerPortabilityImport {
        schema: CONSUMER_PORTABILITY_IMPORT_SCHEMA.to_string(),
        id: row.get(0)?,
        destination_project_id: row.get(1)?,
        source_operator: row.get(2)?,
        source_project_id,
        source_package_id,
        source_package_schema,
        envelope_sha256: row.get(6)?,
        payload_sha256,
        package_json,
        package,
        trust_status: row.get(10)?,
        merge_status: CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS.to_string(),
        signer_key_record_id: row.get(11)?,
        signature: signature_from_row(row)?,
        signature_verified_at: row.get(15)?,
        imported_at: row.get(9)?,
    })
}

fn signature_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<Option<ConsumerPortabilityPackageSignature>> {
    let algorithm: Option<String> = row.get(12)?;
    let key_id: Option<String> = row.get(13)?;
    let signature_base64: Option<String> = row.get(14)?;
    match (algorithm, key_id, signature_base64) {
        (None, None, None) => Ok(None),
        (Some(algorithm), Some(key_id), Some(signature_base64)) => {
            Ok(Some(ConsumerPortabilityPackageSignature {
                algorithm,
                key_id,
                signature_base64,
            }))
        }
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            std::io::Error::new(std::io::ErrorKind::InvalidData, "导入记录签名字段不完整").into(),
        )),
    }
}

const PORTABILITY_IMPORT_SELECT: &str = "SELECT id, destination_project_id,
       source_operator, source_project_id, source_package_id, source_package_schema,
       envelope_sha256, payload_sha256, package_json, imported_at,
       trust_status, signer_key_record_id, signature_algorithm, signer_key_id,
       signature_base64, signature_verified_at
  FROM open_commerce_consumer_portability_imports";
