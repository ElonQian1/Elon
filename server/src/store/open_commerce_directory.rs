use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::{
    open_commerce_directory_model::{
        OpenCommerceDirectoryMerchantDetail, OpenCommerceDirectoryPublication,
        DIRECTORY_STATUS_PUBLISHED, DIRECTORY_STATUS_UNPUBLISHED,
    },
    open_commerce_model::{
        normalize_capability_key, ACCESS_OWNER_ONLY, CAPABILITY_STATUS_ACTIVE,
        MERCHANT_STATUS_ACTIVE,
    },
};

use super::{now, Store};

impl Store {
    pub(crate) fn set_open_commerce_directory_publication(
        &self,
        project_id: &str,
        merchant_id: &str,
        actor_user_id: &str,
        published: bool,
    ) -> Result<OpenCommerceDirectoryPublication> {
        self.open_commerce_merchant_for_project(project_id, merchant_id)?;
        if published {
            let discoverable = self
                .list_open_commerce_capabilities(merchant_id)?
                .into_iter()
                .any(|capability| {
                    capability.status == CAPABILITY_STATUS_ACTIVE
                        && capability.access_level != ACCESS_OWNER_ONLY
                });
            if !discoverable {
                bail!("发布到开放目录前，至少需要一项有效的 public 或 authorized 能力");
            }
        }
        let timestamp = now();
        let status = if published {
            DIRECTORY_STATUS_PUBLISHED
        } else {
            DIRECTORY_STATUS_UNPUBLISHED
        };
        self.conn()?.execute(
            "INSERT INTO open_commerce_directory_publications (
               merchant_id, project_id, status, revision, published_by_user_id,
               published_at, unpublished_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, 1, ?4,
               CASE WHEN ?3 = 'published' THEN ?5 ELSE NULL END,
               CASE WHEN ?3 = 'unpublished' THEN ?5 ELSE NULL END,
               ?5
             )
             ON CONFLICT(merchant_id) DO UPDATE SET
               status = excluded.status,
               revision = open_commerce_directory_publications.revision + 1,
               published_by_user_id = excluded.published_by_user_id,
               published_at = CASE
                 WHEN excluded.status = 'published' THEN excluded.updated_at
                 ELSE open_commerce_directory_publications.published_at
               END,
               unpublished_at = CASE
                 WHEN excluded.status = 'unpublished' THEN excluded.updated_at
                 ELSE NULL
               END,
               updated_at = excluded.updated_at
             WHERE open_commerce_directory_publications.project_id = excluded.project_id",
            params![
                merchant_id.trim(),
                project_id.trim(),
                status,
                actor_user_id.trim(),
                timestamp
            ],
        )?;
        self.open_commerce_directory_publication(merchant_id)?
            .ok_or_else(|| anyhow!("目录发布状态保存失败"))
    }

    pub(crate) fn open_commerce_directory_publication(
        &self,
        merchant_id: &str,
    ) -> Result<Option<OpenCommerceDirectoryPublication>> {
        self.conn()?
            .query_row(
                &format!("{PUBLICATION_SELECT} WHERE merchant_id = ?1"),
                params![merchant_id.trim()],
                publication_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn list_project_open_commerce_directory_publications(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceDirectoryPublication>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{PUBLICATION_SELECT} WHERE project_id = ?1 ORDER BY updated_at DESC"
        ))?;
        let rows = stmt
            .query_map(params![project_id.trim()], publication_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub(crate) fn open_commerce_directory_is_published(&self, merchant_id: &str) -> Result<bool> {
        Ok(self
            .open_commerce_directory_publication(merchant_id)?
            .is_some_and(|publication| publication.status == DIRECTORY_STATUS_PUBLISHED))
    }

    pub(crate) fn published_open_commerce_merchant_detail(
        &self,
        merchant_id: &str,
    ) -> Result<OpenCommerceDirectoryMerchantDetail> {
        let publication = self
            .open_commerce_directory_publication(merchant_id)?
            .filter(|value| value.status == DIRECTORY_STATUS_PUBLISHED)
            .ok_or_else(|| anyhow!("商户节点未发布到开放目录"))?;
        let merchant = self.open_commerce_merchant(merchant_id)?;
        if merchant.status != MERCHANT_STATUS_ACTIVE {
            bail!("商户节点当前不可用");
        }
        let capabilities = self
            .list_open_commerce_capabilities(merchant_id)?
            .into_iter()
            .filter(|capability| {
                capability.status == CAPABILITY_STATUS_ACTIVE
                    && capability.access_level != ACCESS_OWNER_ONLY
            })
            .collect();
        let portable_identity_keys =
            self.list_public_open_commerce_merchant_identity_keys(merchant_id)?;
        let capability_source_links =
            self.list_publishable_open_commerce_capability_source_links(merchant_id)?;
        Ok(OpenCommerceDirectoryMerchantDetail::from_domain(
            merchant,
            capabilities,
            publication,
            portable_identity_keys,
            capability_source_links,
        ))
    }

    pub(crate) fn search_published_open_commerce_merchants(
        &self,
        query: Option<&str>,
        capability_key: Option<&str>,
        limit: usize,
    ) -> Result<Vec<OpenCommerceDirectoryMerchantDetail>> {
        let query = query.map(str::trim).filter(|value| !value.is_empty());
        let like = query.map(|value| format!("%{}%", value.replace('%', "\\%")));
        let capability_key = capability_key.map(normalize_capability_key).transpose()?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT m.id
               FROM open_commerce_merchants m
               JOIN open_commerce_directory_publications p ON p.merchant_id = m.id
              WHERE m.status = 'active' AND p.status = 'published'
                AND (?1 IS NULL OR m.display_name LIKE ?1 ESCAPE '\\'
                     OR m.slug LIKE ?1 ESCAPE '\\' OR m.description LIKE ?1 ESCAPE '\\')
                AND EXISTS (
                  SELECT 1 FROM open_commerce_capabilities c
                   WHERE c.merchant_id = m.id AND c.status = 'active'
                     AND c.access_level != 'owner_only'
                     AND (?2 IS NULL OR c.capability_key = ?2)
                )
              ORDER BY p.updated_at DESC, m.updated_at DESC LIMIT ?3",
        )?;
        let merchant_ids = stmt
            .query_map(
                params![like, capability_key, limit.clamp(1, 100) as i64],
                |row| row.get::<_, String>(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        drop(conn);
        merchant_ids
            .into_iter()
            .map(|merchant_id| self.published_open_commerce_merchant_detail(&merchant_id))
            .collect()
    }
}

fn publication_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceDirectoryPublication> {
    Ok(OpenCommerceDirectoryPublication {
        merchant_id: row.get(0)?,
        project_id: row.get(1)?,
        status: row.get(2)?,
        revision: row.get(3)?,
        published_by_user_id: row.get(4)?,
        published_at: row.get(5)?,
        unpublished_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

const PUBLICATION_SELECT: &str = "SELECT merchant_id, project_id, status, revision,
       published_by_user_id, published_at, unpublished_at, updated_at
  FROM open_commerce_directory_publications";
