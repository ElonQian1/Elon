use anyhow::{anyhow, Result};
use chrono::Utc;
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_consumer_preference_model::{
        ConsumerPreferenceDisclosure, ConsumerPreferenceProfile, DisclosedConsumerPreferences,
    },
    open_commerce_relationship_model::OpenCommerceConsumerRelationship,
};

use super::{now, open_commerce_consumer_relationships::effective_relationship_status, Store};

impl Store {
    pub(crate) fn open_commerce_consumer_preference_profile(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
    ) -> Result<Option<ConsumerPreferenceProfile>> {
        self.conn()?
            .query_row(
                "SELECT preferences_json, revision, created_at, updated_at
                   FROM open_commerce_consumer_preference_profiles
                  WHERE consumer_project_id=?1 AND consumer_user_id=?2",
                params![consumer_project_id, consumer_user_id],
                preference_profile_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn upsert_open_commerce_consumer_preference_profile(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        preferences: &ConsumerPreferences,
    ) -> Result<ConsumerPreferenceProfile> {
        let timestamp = now();
        self.conn()?.execute(
            "INSERT INTO open_commerce_consumer_preference_profiles (
               consumer_project_id, consumer_user_id, preferences_json,
               revision, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 1, ?4, ?4)
             ON CONFLICT(consumer_project_id, consumer_user_id) DO UPDATE SET
               preferences_json=excluded.preferences_json,
               revision=open_commerce_consumer_preference_profiles.revision + 1,
               updated_at=excluded.updated_at",
            params![
                consumer_project_id,
                consumer_user_id,
                serde_json::to_string(preferences)?,
                timestamp,
            ],
        )?;
        self.open_commerce_consumer_preference_profile(consumer_project_id, consumer_user_id)?
            .ok_or_else(|| anyhow!("消费者偏好档案保存后不存在"))
    }

    pub(crate) fn delete_open_commerce_consumer_preference_profile(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
    ) -> Result<(bool, usize)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let removed_disclosures = tx.execute(
            "DELETE FROM open_commerce_consumer_preference_disclosures
              WHERE relationship_id IN (
                SELECT id FROM open_commerce_consumer_relationships
                 WHERE consumer_project_id=?1 AND consumer_user_id=?2
              )",
            params![consumer_project_id, consumer_user_id],
        )?;
        let deleted_profile = tx.execute(
            "DELETE FROM open_commerce_consumer_preference_profiles
              WHERE consumer_project_id=?1 AND consumer_user_id=?2",
            params![consumer_project_id, consumer_user_id],
        )? > 0;
        tx.commit()?;
        Ok((deleted_profile, removed_disclosures))
    }

    pub(crate) fn upsert_open_commerce_consumer_preference_disclosure(
        &self,
        relationship: &OpenCommerceConsumerRelationship,
        shared_fields: &[String],
        preferences: &DisclosedConsumerPreferences,
        profile_revision: i64,
    ) -> Result<ConsumerPreferenceDisclosure> {
        let timestamp = now();
        self.conn()?.execute(
            "INSERT INTO open_commerce_consumer_preference_disclosures (
               relationship_id, shared_fields_json, disclosure_json,
               profile_revision, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(relationship_id) DO UPDATE SET
               shared_fields_json=excluded.shared_fields_json,
               disclosure_json=excluded.disclosure_json,
               profile_revision=excluded.profile_revision,
               updated_at=excluded.updated_at",
            params![
                relationship.id,
                serde_json::to_string(shared_fields)?,
                serde_json::to_string(preferences)?,
                profile_revision,
                timestamp,
            ],
        )?;
        self.consumer_owned_open_commerce_preference_disclosure(
            &relationship.id,
            &relationship.subject_alias,
            &relationship.merchant_id,
            &relationship.status,
        )?
        .ok_or_else(|| anyhow!("消费者偏好披露保存后不存在"))
    }

    pub(crate) fn consumer_owned_open_commerce_preference_disclosure(
        &self,
        relationship_id: &str,
        subject_alias: &str,
        merchant_id: &str,
        relationship_status: &str,
    ) -> Result<Option<ConsumerPreferenceDisclosure>> {
        self.conn()?
            .query_row(
                "SELECT shared_fields_json, disclosure_json, profile_revision,
                        created_at, updated_at
                   FROM open_commerce_consumer_preference_disclosures
                  WHERE relationship_id=?1",
                params![relationship_id],
                |row| {
                    disclosure_from_values(
                        row,
                        relationship_id,
                        merchant_id,
                        subject_alias,
                        relationship_status,
                    )
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn delete_open_commerce_consumer_preference_disclosure(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        relationship_id: &str,
    ) -> Result<bool> {
        let deleted = self.conn()?.execute(
            "DELETE FROM open_commerce_consumer_preference_disclosures
              WHERE relationship_id=?1 AND relationship_id IN (
                SELECT id FROM open_commerce_consumer_relationships
                 WHERE consumer_project_id=?2 AND consumer_user_id=?3
              )",
            params![relationship_id, consumer_project_id, consumer_user_id],
        )?;
        Ok(deleted > 0)
    }

    pub(crate) fn list_open_commerce_consumer_preference_disclosures(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ConsumerPreferenceDisclosure>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
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
              ORDER BY disclosure.updated_at DESC
              LIMIT ?3",
        )?;
        let rows = stmt.query_map(
            params![
                consumer_project_id,
                consumer_user_id,
                limit.clamp(1, 200) as i64
            ],
            merchant_disclosure_from_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn list_open_commerce_merchant_preference_disclosures(
        &self,
        merchant_project_id: &str,
        merchant_id: &str,
        limit: usize,
    ) -> Result<Vec<ConsumerPreferenceDisclosure>> {
        self.open_commerce_merchant_for_project(merchant_project_id, merchant_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT disclosure.relationship_id, relationship.merchant_id,
                    relationship.subject_alias, relationship.status,
                    relationship.expires_at, disclosure.shared_fields_json,
                    disclosure.disclosure_json, disclosure.profile_revision,
                    disclosure.created_at, disclosure.updated_at
               FROM open_commerce_consumer_preference_disclosures disclosure
               JOIN open_commerce_consumer_relationships relationship
                 ON relationship.id=disclosure.relationship_id
              WHERE relationship.merchant_project_id=?1
                AND relationship.merchant_id=?2
                AND relationship.status='active'
                AND relationship.expires_at>?3
              ORDER BY disclosure.updated_at DESC
              LIMIT ?4",
        )?;
        let timestamp = Utc::now().to_rfc3339();
        let rows = stmt.query_map(
            params![
                merchant_project_id,
                merchant_id,
                timestamp,
                limit.clamp(1, 200) as i64
            ],
            merchant_disclosure_from_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

fn preference_profile_from_row(row: &Row<'_>) -> rusqlite::Result<ConsumerPreferenceProfile> {
    let preferences_json: String = row.get(0)?;
    let preferences = parse_json(&preferences_json)?;
    Ok(ConsumerPreferenceProfile {
        preferences,
        revision: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

fn merchant_disclosure_from_row(row: &Row<'_>) -> rusqlite::Result<ConsumerPreferenceDisclosure> {
    let stored_status: String = row.get(3)?;
    let expires_at: String = row.get(4)?;
    let relationship_status = effective_relationship_status(&stored_status, &expires_at);
    let shared_fields_json: String = row.get(5)?;
    let disclosure_json: String = row.get(6)?;
    Ok(ConsumerPreferenceDisclosure {
        relationship_id: row.get(0)?,
        merchant_id: row.get(1)?,
        subject_alias: row.get(2)?,
        relationship_status,
        shared_fields: parse_json(&shared_fields_json)?,
        preferences: parse_json(&disclosure_json)?,
        profile_revision: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn disclosure_from_values(
    row: &Row<'_>,
    relationship_id: &str,
    merchant_id: &str,
    subject_alias: &str,
    relationship_status: &str,
) -> rusqlite::Result<ConsumerPreferenceDisclosure> {
    let shared_fields_json: String = row.get(0)?;
    let disclosure_json: String = row.get(1)?;
    Ok(ConsumerPreferenceDisclosure {
        relationship_id: relationship_id.to_string(),
        merchant_id: merchant_id.to_string(),
        subject_alias: subject_alias.to_string(),
        relationship_status: relationship_status.to_string(),
        shared_fields: parse_json(&shared_fields_json)?,
        preferences: parse_json(&disclosure_json)?,
        profile_revision: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(raw: &str) -> rusqlite::Result<T> {
    serde_json::from_str(raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            raw.len(),
            rusqlite::types::Type::Text,
            error.into(),
        )
    })
}
