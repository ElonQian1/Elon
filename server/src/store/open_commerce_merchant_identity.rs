use anyhow::Result;
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};
use std::collections::BTreeSet;

use crate::open_commerce_merchant_identity_model::{
    OpenCommerceMerchantIdentityKey, OpenCommercePublicMerchantIdentityKey,
    MERCHANT_IDENTITY_KEY_SCHEMA,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn save_open_commerce_merchant_identity_key(
        &self,
        project_id: &str,
        merchant_id: &str,
        key_id: &str,
        algorithm: &str,
        public_key_pem: &str,
        proof_signature_base64: &str,
        actor_user_id: &str,
        proof_verified_at: &str,
    ) -> Result<(OpenCommerceMerchantIdentityKey, bool)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                &format!(
                    "{IDENTITY_KEY_SELECT}
                       WHERE project_id=?1 AND merchant_id=?2 AND key_id=?3"
                ),
                params![project_id.trim(), merchant_id.trim(), key_id],
                identity_key_from_row,
            )
            .optional()?;
        if let Some(existing) = existing {
            tx.commit()?;
            return Ok((existing, false));
        }
        let id = new_id("merchant-identity-key");
        let created_at = now();
        tx.execute(
            "INSERT INTO open_commerce_merchant_identity_keys (
               id, project_id, merchant_id, key_id, algorithm, public_key_pem,
               proof_signature_base64, status, proof_verified_at,
               created_by_user_id, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8, ?9, ?10)",
            params![
                id,
                project_id.trim(),
                merchant_id.trim(),
                key_id,
                algorithm,
                public_key_pem,
                proof_signature_base64,
                proof_verified_at,
                actor_user_id.trim(),
                created_at,
            ],
        )?;
        let saved = tx.query_row(
            &format!("{IDENTITY_KEY_SELECT} WHERE id=?1"),
            params![id],
            identity_key_from_row,
        )?;
        tx.commit()?;
        Ok((saved, true))
    }

    pub(crate) fn list_open_commerce_merchant_identity_keys(
        &self,
        project_id: &str,
        merchant_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceMerchantIdentityKey>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{IDENTITY_KEY_SELECT}
               WHERE project_id=?1 AND merchant_id=?2
               ORDER BY created_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                project_id.trim(),
                merchant_id.trim(),
                limit.clamp(1, 100) as i64,
            ],
            identity_key_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn list_active_open_commerce_merchant_identity_keys(
        &self,
        merchant_id: &str,
    ) -> Result<Vec<OpenCommerceMerchantIdentityKey>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{IDENTITY_KEY_SELECT}
               WHERE merchant_id=?1 AND status='active'
               ORDER BY created_at DESC, rowid DESC LIMIT 3"
        ))?;
        let rows = stmt.query_map(params![merchant_id.trim()], identity_key_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn list_public_open_commerce_merchant_identity_keys(
        &self,
        merchant_id: &str,
    ) -> Result<Vec<OpenCommercePublicMerchantIdentityKey>> {
        Ok(self
            .list_active_open_commerce_merchant_identity_keys(merchant_id)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub(crate) fn published_open_commerce_merchant_ids_for_identity_keys(
        &self,
        key_ids: &[String],
    ) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT identity.merchant_id
               FROM open_commerce_merchant_identity_keys identity
               JOIN open_commerce_merchants merchant ON merchant.id=identity.merchant_id
               JOIN open_commerce_directory_publications publication
                 ON publication.merchant_id=identity.merchant_id
              WHERE identity.key_id=?1 AND identity.status='active'
                AND merchant.status='active' AND publication.status='published'
              ORDER BY identity.created_at DESC LIMIT 50",
        )?;
        let mut merchant_ids = BTreeSet::new();
        for key_id in key_ids.iter().take(3) {
            let rows = stmt.query_map(params![key_id], |row| row.get::<_, String>(0))?;
            merchant_ids.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
        }
        Ok(merchant_ids.into_iter().collect())
    }

    pub(crate) fn revoke_open_commerce_merchant_identity_key(
        &self,
        project_id: &str,
        merchant_id: &str,
        record_id: &str,
    ) -> Result<Option<OpenCommerceMerchantIdentityKey>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE open_commerce_merchant_identity_keys
                SET status='revoked', revoked_at=COALESCE(revoked_at, ?4)
              WHERE id=?1 AND project_id=?2 AND merchant_id=?3 AND status='active'",
            params![
                record_id.trim(),
                project_id.trim(),
                merchant_id.trim(),
                now()
            ],
        )?;
        let value = tx
            .query_row(
                &format!(
                    "{IDENTITY_KEY_SELECT}
                       WHERE id=?1 AND project_id=?2 AND merchant_id=?3"
                ),
                params![record_id.trim(), project_id.trim(), merchant_id.trim()],
                identity_key_from_row,
            )
            .optional()?;
        tx.commit()?;
        Ok(value)
    }
}

fn identity_key_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceMerchantIdentityKey> {
    Ok(OpenCommerceMerchantIdentityKey {
        schema: MERCHANT_IDENTITY_KEY_SCHEMA.to_string(),
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        key_id: row.get(3)?,
        algorithm: row.get(4)?,
        public_key_pem: row.get(5)?,
        proof_signature_base64: row.get(6)?,
        status: row.get(7)?,
        proof_verified_at: row.get(8)?,
        created_at: row.get(9)?,
        revoked_at: row.get(10)?,
    })
}

const IDENTITY_KEY_SELECT: &str = "SELECT id, project_id, merchant_id, key_id,
       algorithm, public_key_pem, proof_signature_base64, status,
       proof_verified_at, created_at, revoked_at
  FROM open_commerce_merchant_identity_keys";
