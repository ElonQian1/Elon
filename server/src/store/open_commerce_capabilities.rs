use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Row};
use serde_json::Value;

use crate::open_commerce_model::{
    normalize_capability_key, validate_access_level, validate_capability_kind,
    validate_display_name, validate_handler_type, validate_json_object, validate_status,
    CreateCapabilityRequest, OpenCommerceCapability, UpdateCapabilityRequest,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_open_commerce_capability(
        &self,
        project_id: &str,
        merchant_id: &str,
        request: CreateCapabilityRequest,
    ) -> Result<OpenCommerceCapability> {
        self.open_commerce_merchant_for_project(project_id, merchant_id)?;
        let capability_key = normalize_capability_key(&request.capability_key)?;
        let display_name = validate_display_name(&request.display_name, "能力名称")?;
        let kind = validate_capability_kind(&request.kind)?;
        let access_level = validate_access_level(&request.access_level)?;
        let input_schema = validate_json_object(&request.input_schema, "输入 schema")?;
        let output_schema = validate_json_object(&request.output_schema, "输出 schema")?;
        let handler_type = validate_handler_type(&request.handler_type)?;
        let handler_config =
            validate_handler_config(&handler_type, request.handler_config.as_ref())?;
        let currency = normalize_currency(&request.currency)?;
        if request.unit_price_micros < 0 || request.freshness_seconds < 0 {
            bail!("价格和新鲜度不能为负数");
        }
        let id = new_id("capability");
        let timestamp = now();
        self.conn()?
            .execute(
                "INSERT INTO open_commerce_capabilities (
                    id, merchant_id, capability_key, display_name, description, kind,
                    access_level, input_schema_json, output_schema_json, handler_type,
                    handler_config_json, unit_price_micros, currency, freshness_seconds,
                    status, version, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                    ?14, 'active', 1, ?15, ?15
                 )",
                params![
                    id,
                    merchant_id.trim(),
                    capability_key,
                    display_name,
                    request.description.trim(),
                    kind,
                    access_level,
                    serde_json::to_string(&input_schema)?,
                    serde_json::to_string(&output_schema)?,
                    handler_type,
                    optional_json_string(handler_config.as_ref())?,
                    request.unit_price_micros,
                    currency,
                    request.freshness_seconds,
                    timestamp
                ],
            )
            .map_err(map_capability_conflict)?;
        self.open_commerce_capability(&id)
    }

    pub(crate) fn update_open_commerce_capability(
        &self,
        project_id: &str,
        capability_id: &str,
        request: UpdateCapabilityRequest,
    ) -> Result<OpenCommerceCapability> {
        let current = self.open_commerce_capability_for_project(project_id, capability_id)?;
        let display_name = match request.display_name {
            Some(value) => validate_display_name(&value, "能力名称")?,
            None => current.display_name,
        };
        let description = request
            .description
            .map(|value| value.trim().to_string())
            .unwrap_or(current.description);
        let access_level = match request.access_level {
            Some(value) => validate_access_level(&value)?,
            None => current.access_level,
        };
        let input_schema = match request.input_schema {
            Some(value) => validate_json_object(&value, "输入 schema")?,
            None => current.input_schema,
        };
        let output_schema = match request.output_schema {
            Some(value) => validate_json_object(&value, "输出 schema")?,
            None => current.output_schema,
        };
        let handler_type = match request.handler_type {
            Some(value) => validate_handler_type(&value)?,
            None => current.handler_type,
        };
        let handler_config = validate_handler_config(
            &handler_type,
            request
                .handler_config
                .as_ref()
                .or(current.handler_config.as_ref()),
        )?;
        let unit_price_micros = request
            .unit_price_micros
            .unwrap_or(current.unit_price_micros);
        let currency = match request.currency {
            Some(value) => normalize_currency(&value)?,
            None => current.currency,
        };
        let freshness_seconds = request
            .freshness_seconds
            .unwrap_or(current.freshness_seconds);
        let status = match request.status {
            Some(value) => validate_status(&value)?,
            None => current.status,
        };
        if unit_price_micros < 0 || freshness_seconds < 0 {
            bail!("价格和新鲜度不能为负数");
        }
        let updated = self.conn()?.execute(
            "UPDATE open_commerce_capabilities
                SET display_name = ?1, description = ?2, access_level = ?3,
                    input_schema_json = ?4, output_schema_json = ?5, handler_type = ?6,
                    handler_config_json = ?7, unit_price_micros = ?8, currency = ?9,
                    freshness_seconds = ?10, status = ?11, version = version + 1,
                    updated_at = ?12
              WHERE id = ?13
                AND merchant_id IN (
                    SELECT id FROM open_commerce_merchants WHERE project_id = ?14
                )",
            params![
                display_name,
                description,
                access_level,
                serde_json::to_string(&input_schema)?,
                serde_json::to_string(&output_schema)?,
                handler_type,
                optional_json_string(handler_config.as_ref())?,
                unit_price_micros,
                currency,
                freshness_seconds,
                status,
                now(),
                capability_id.trim(),
                project_id.trim()
            ],
        )?;
        if updated == 0 {
            bail!("商业能力不存在");
        }
        self.open_commerce_capability(capability_id)
    }

    pub(crate) fn open_commerce_capability(
        &self,
        capability_id: &str,
    ) -> Result<OpenCommerceCapability> {
        self.conn()?
            .query_row(
                &format!("{CAPABILITY_SELECT} WHERE id = ?1"),
                params![capability_id.trim()],
                capability_from_row,
            )
            .map_err(|error| anyhow!(error).context("商业能力不存在"))
    }

    pub(crate) fn open_commerce_capability_for_project(
        &self,
        project_id: &str,
        capability_id: &str,
    ) -> Result<OpenCommerceCapability> {
        self.conn()?
            .query_row(
                &format!(
                    "{CAPABILITY_SELECT}
                     WHERE id = ?1 AND merchant_id IN (
                       SELECT id FROM open_commerce_merchants WHERE project_id = ?2
                     )"
                ),
                params![capability_id.trim(), project_id.trim()],
                capability_from_row,
            )
            .map_err(|error| anyhow!(error).context("当前项目中不存在该商业能力"))
    }

    pub(crate) fn open_commerce_capability_by_key(
        &self,
        merchant_id: &str,
        capability_key: &str,
    ) -> Result<OpenCommerceCapability> {
        let capability_key = normalize_capability_key(capability_key)?;
        self.conn()?
            .query_row(
                &format!(
                    "{CAPABILITY_SELECT}
                     WHERE merchant_id = ?1 AND capability_key = ?2"
                ),
                params![merchant_id.trim(), capability_key],
                capability_from_row,
            )
            .map_err(|error| anyhow!(error).context("商户未发布该商业能力"))
    }

    pub(crate) fn list_open_commerce_capabilities(
        &self,
        merchant_id: &str,
    ) -> Result<Vec<OpenCommerceCapability>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{CAPABILITY_SELECT} WHERE merchant_id = ?1 ORDER BY created_at ASC"
        ))?;
        Ok(stmt
            .query_map(params![merchant_id.trim()], capability_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

pub(super) fn public_capability(mut capability: OpenCommerceCapability) -> OpenCommerceCapability {
    capability.handler_config = None;
    capability
}

fn normalize_currency(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_uppercase();
    if value.len() != 3 || !value.chars().all(|ch| ch.is_ascii_uppercase()) {
        bail!("币种必须是 3 位大写 ISO 代码");
    }
    Ok(value)
}

fn validate_handler_config(handler_type: &str, config: Option<&Value>) -> Result<Option<Value>> {
    match handler_type {
        "merchant_profile" => Ok(None),
        "static_json" => config
            .map(|value| validate_json_object(value, "静态处理器配置"))
            .transpose(),
        _ => bail!("未知商业能力处理器"),
    }
}

fn optional_json_string(value: Option<&Value>) -> Result<Option<String>> {
    value
        .map(serde_json::to_string)
        .transpose()
        .map_err(Into::into)
}

fn parse_json(value: String, label: &str) -> rusqlite::Result<Value> {
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            anyhow!("{label}: {error}").into(),
        )
    })
}

fn capability_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceCapability> {
    let handler_config = row
        .get::<_, Option<String>>(10)?
        .map(|value| parse_json(value, "处理器配置 JSON 无效"))
        .transpose()?;
    Ok(OpenCommerceCapability {
        id: row.get(0)?,
        merchant_id: row.get(1)?,
        capability_key: row.get(2)?,
        display_name: row.get(3)?,
        description: row.get(4)?,
        kind: row.get(5)?,
        access_level: row.get(6)?,
        input_schema: parse_json(row.get(7)?, "输入 schema JSON 无效")?,
        output_schema: parse_json(row.get(8)?, "输出 schema JSON 无效")?,
        handler_type: row.get(9)?,
        handler_config,
        unit_price_micros: row.get(11)?,
        currency: row.get(12)?,
        freshness_seconds: row.get(13)?,
        status: row.get(14)?,
        version: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

fn map_capability_conflict(error: rusqlite::Error) -> anyhow::Error {
    if error.to_string().contains("UNIQUE constraint failed") {
        anyhow!("当前商户已发布相同能力键")
    } else {
        anyhow!(error)
    }
}

const CAPABILITY_SELECT: &str =
    "SELECT id, merchant_id, capability_key, display_name, description, kind,
            access_level, input_schema_json, output_schema_json, handler_type,
            handler_config_json, unit_price_micros, currency, freshness_seconds,
            status, version, created_at, updated_at
       FROM open_commerce_capabilities";
