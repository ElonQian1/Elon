use anyhow::{bail, Result};
use rusqlite::Connection;
use serde_json::Value;

use crate::compute_federation::external_pool_adapter_upstream_transport_target::server_upstream_transport_target_policy_catalog;

pub(super) fn install(conn: &Connection) -> Result<()> {
    let (policy, policy_digest) = server_upstream_transport_target_policy_catalog()?;
    let value = serde_json::to_value(&policy)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("V258 target policy catalog is not an object"))?;
    if object.len() != POLICY_FIELD_COUNT {
        bail!("V258 target policy field count drifted");
    }
    let policy_json =
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            &policy,
            1024 * 1024,
        )?
        .0;
    let mismatch = object
        .iter()
        .map(|(field, value)| mismatch(field, value))
        .collect::<Result<Vec<_>>>()?
        .join("\n          OR ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS external_pool_adapter_upstream_transport_target_policy_json_projection
         BEFORE INSERT ON compute_external_pool_adapter_upstream_transport_targets
         WHEN (SELECT COUNT(*) FROM json_each(NEW.target_policy_json))!={POLICY_FIELD_COUNT}
           OR {mismatch}
           OR NEW.target_policy_digest IS NOT '{}'
           OR NEW.target_policy_json IS NOT '{}'
         BEGIN SELECT RAISE(ABORT,'V258 server transport target policy projection is not exact'); END;",
        quoted(&policy_digest),
        quoted(&policy_json)
    ))?;
    Ok(())
}

#[cfg(test)]
pub(super) fn count() -> usize {
    POLICY_FIELD_COUNT
}

fn mismatch(field: &str, value: &Value) -> Result<String> {
    let path = format!("$.{field}");
    let (kind, literal) = match value {
        Value::String(value) => ("text", format!("'{}'", quoted(value))),
        Value::Number(value) => ("integer", value.to_string()),
        Value::Bool(value) => (
            if *value { "true" } else { "false" },
            if *value { "1".into() } else { "0".into() },
        ),
        _ => bail!("V258 target policy field {field} is not scalar"),
    };
    Ok(format!(
        "json_type(NEW.target_policy_json,'{path}') IS NOT '{kind}' OR json_extract(NEW.target_policy_json,'{path}') IS NOT {literal}"
    ))
}

fn quoted(value: &str) -> String {
    value.replace('\'', "''")
}

const POLICY_FIELD_COUNT: usize = 24;
