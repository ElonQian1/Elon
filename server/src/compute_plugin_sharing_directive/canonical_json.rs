use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const MAX_IJSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_PREPARATION_LEDGER_JSON_BYTES: usize = 512 * 1024;
const PREPARATION_CONTEXT_DOMAIN: &[u8] =
    b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_CONTEXT_V1";
const PREPARATION_OBSERVED_DOMAIN: &[u8] =
    b"ELON_COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1";

/// RFC 8785 key ordering and string encoding over the I-JSON integer subset used by compute
/// plugin control records. Floats and integers outside JavaScript's exact range fail closed.
pub(crate) fn canonical_compute_plugin_ijson_and_sha256<T: Serialize>(
    value: &T,
    max_bytes: usize,
) -> Result<(String, String)> {
    let bytes = canonical_compute_plugin_ijson(value, max_bytes)?;
    let digest = hex::encode(Sha256::digest(&bytes));
    Ok((String::from_utf8(bytes)?, digest))
}

pub(crate) fn compute_plugin_install_plan_preparation_context_json_and_digest<T: Serialize>(
    value: &T,
) -> Result<(String, String)> {
    domain_separated_json_and_digest(PREPARATION_CONTEXT_DOMAIN, value)
}

pub(crate) fn compute_plugin_install_plan_preparation_observed_json_and_digest<T: Serialize>(
    value: &T,
) -> Result<(String, String)> {
    domain_separated_json_and_digest(PREPARATION_OBSERVED_DOMAIN, value)
}

fn domain_separated_json_and_digest<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<(String, String)> {
    let bytes = canonical_compute_plugin_ijson(value, MAX_PREPARATION_LEDGER_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(&bytes);
    Ok((String::from_utf8(bytes)?, hex::encode(digest.finalize())))
}

fn canonical_compute_plugin_ijson<T: Serialize>(value: &T, max_bytes: usize) -> Result<Vec<u8>> {
    let value = serde_json::to_value(value)?;
    let mut output = Vec::new();
    write_canonical_json(&value, &mut output)?;
    if output.len() > max_bytes {
        bail!("算力插件规范 JSON 超过有界账本大小");
    }
    Ok(output)
}

fn write_canonical_json(value: &Value, output: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(value) => output.extend_from_slice(if *value { b"true" } else { b"false" }),
        Value::Number(value) => {
            let safe_integer = value
                .as_u64()
                .is_some_and(|number| number <= MAX_IJSON_SAFE_INTEGER)
                || value.as_i64().is_some_and(|number| {
                    number >= -(MAX_IJSON_SAFE_INTEGER as i64)
                        && number <= MAX_IJSON_SAFE_INTEGER as i64
                });
            if !safe_integer {
                bail!("算力插件规范 JSON 只接受 I-JSON 安全整数");
            }
            output.extend_from_slice(value.to_string().as_bytes());
        }
        Value::String(value) => output.extend_from_slice(serde_json::to_string(value)?.as_bytes()),
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                write_canonical_json(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable_by_key(|key| key.encode_utf16().collect::<Vec<_>>());
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                output.extend_from_slice(serde_json::to_string(key)?.as_bytes());
                output.push(b':');
                write_canonical_json(&values[key], output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}
