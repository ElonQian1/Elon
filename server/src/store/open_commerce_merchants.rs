use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Row};

use crate::open_commerce_model::{
    normalize_slug, slug_from_display_name, validate_display_name, validate_json_object,
    validate_status, CreateMerchantRequest, OpenCommerceMerchant, OpenCommerceMerchantDetail,
    UpdateMerchantRequest, OPEN_COMMERCE_SCHEMA,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_open_commerce_merchant(
        &self,
        project_id: &str,
        owner_user_id: &str,
        request: CreateMerchantRequest,
    ) -> Result<OpenCommerceMerchant> {
        let display_name = validate_display_name(&request.display_name, "商户名称")?;
        let slug = match request.slug.as_deref() {
            Some(value) => normalize_slug(value)?,
            None => slug_from_display_name(&display_name)?,
        };
        let node_mode = normalize_node_mode(&request.node_mode)?;
        let public_profile = validate_json_object(&request.public_profile, "公开资料")?;
        let id = new_id("merchant");
        let timestamp = now();
        self.conn()?
            .execute(
                "INSERT INTO open_commerce_merchants (
                    id, project_id, owner_user_id, slug, display_name, description,
                    status, node_mode, public_profile_json, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?8, ?9, ?9)",
                params![
                    id,
                    project_id.trim(),
                    owner_user_id.trim(),
                    slug,
                    display_name,
                    request.description.trim(),
                    node_mode,
                    serde_json::to_string(&public_profile)?,
                    timestamp
                ],
            )
            .map_err(map_merchant_conflict)?;
        self.open_commerce_merchant(&id)
    }

    pub(crate) fn update_open_commerce_merchant(
        &self,
        project_id: &str,
        merchant_id: &str,
        request: UpdateMerchantRequest,
    ) -> Result<OpenCommerceMerchant> {
        let current = self.open_commerce_merchant_for_project(project_id, merchant_id)?;
        let display_name = match request.display_name {
            Some(value) => validate_display_name(&value, "商户名称")?,
            None => current.display_name,
        };
        let description = request
            .description
            .map(|value| value.trim().to_string())
            .unwrap_or(current.description);
        let status = match request.status {
            Some(value) => validate_status(&value)?,
            None => current.status,
        };
        let node_mode = match request.node_mode {
            Some(value) => normalize_node_mode(&value)?,
            None => current.node_mode,
        };
        let public_profile = match request.public_profile {
            Some(value) => validate_json_object(&value, "公开资料")?,
            None => current.public_profile,
        };
        let updated = self.conn()?.execute(
            "UPDATE open_commerce_merchants
                SET display_name = ?1, description = ?2, status = ?3, node_mode = ?4,
                    public_profile_json = ?5, updated_at = ?6
              WHERE id = ?7 AND project_id = ?8",
            params![
                display_name,
                description,
                status,
                node_mode,
                serde_json::to_string(&public_profile)?,
                now(),
                merchant_id.trim(),
                project_id.trim()
            ],
        )?;
        if updated == 0 {
            bail!("商户节点不存在");
        }
        self.open_commerce_merchant(merchant_id)
    }

    pub(crate) fn open_commerce_merchant(&self, merchant_id: &str) -> Result<OpenCommerceMerchant> {
        self.conn()?
            .query_row(
                &format!("{MERCHANT_SELECT} WHERE id = ?1"),
                params![merchant_id.trim()],
                merchant_from_row,
            )
            .map_err(|error| anyhow!(error).context("商户节点不存在"))
    }

    pub(crate) fn open_commerce_merchant_for_project(
        &self,
        project_id: &str,
        merchant_id: &str,
    ) -> Result<OpenCommerceMerchant> {
        self.conn()?
            .query_row(
                &format!("{MERCHANT_SELECT} WHERE project_id = ?1 AND id = ?2"),
                params![project_id.trim(), merchant_id.trim()],
                merchant_from_row,
            )
            .map_err(|error| anyhow!(error).context("当前项目中不存在该商户节点"))
    }

    pub(crate) fn open_commerce_merchant_detail(
        &self,
        merchant_id: &str,
    ) -> Result<OpenCommerceMerchantDetail> {
        let merchant = self.open_commerce_merchant(merchant_id)?;
        let capabilities = self.list_open_commerce_capabilities(merchant_id)?;
        Ok(OpenCommerceMerchantDetail {
            schema: OPEN_COMMERCE_SCHEMA,
            merchant,
            capabilities,
        })
    }

    pub(crate) fn list_project_open_commerce_merchants(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceMerchantDetail>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{MERCHANT_SELECT} WHERE project_id = ?1 ORDER BY created_at DESC"
        ))?;
        let merchants = stmt
            .query_map(params![project_id.trim()], merchant_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        drop(conn);
        merchants
            .into_iter()
            .map(|merchant| {
                let capabilities = self.list_open_commerce_capabilities(&merchant.id)?;
                Ok(OpenCommerceMerchantDetail {
                    schema: OPEN_COMMERCE_SCHEMA,
                    merchant,
                    capabilities,
                })
            })
            .collect()
    }
}

fn normalize_node_mode(value: &str) -> Result<String> {
    match value.trim() {
        "platform_hosted" => Ok("platform_hosted".to_string()),
        "self_hosted" => Ok("self_hosted".to_string()),
        "third_party_hosted" => Ok("third_party_hosted".to_string()),
        _ => bail!("节点模式必须是 platform_hosted、self_hosted 或 third_party_hosted"),
    }
}

fn merchant_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceMerchant> {
    let profile: String = row.get(8)?;
    let public_profile = serde_json::from_str(&profile).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            profile.len(),
            rusqlite::types::Type::Text,
            anyhow!("公开资料 JSON 无效: {error}").into(),
        )
    })?;
    Ok(OpenCommerceMerchant {
        id: row.get(0)?,
        project_id: row.get(1)?,
        owner_user_id: row.get(2)?,
        slug: row.get(3)?,
        display_name: row.get(4)?,
        description: row.get(5)?,
        status: row.get(6)?,
        node_mode: row.get(7)?,
        public_profile,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn map_merchant_conflict(error: rusqlite::Error) -> anyhow::Error {
    if error.to_string().contains("UNIQUE constraint failed") {
        anyhow!("当前项目已存在相同商户 slug")
    } else {
        anyhow!(error)
    }
}

const MERCHANT_SELECT: &str =
    "SELECT id, project_id, owner_user_id, slug, display_name, description,
            status, node_mode, public_profile_json, created_at, updated_at
       FROM open_commerce_merchants";
