use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub(super) const TOOL_CONTRACT_SCHEMA: &str = "elon.ui_tool_contract.v1";

pub(super) fn manifest(tools: &[Value]) -> Result<Value> {
    let mut names = BTreeSet::new();
    for tool in tools {
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .context("MCP tool definition 缺少 name")?;
        if !names.insert(name.to_string()) {
            bail!("MCP tool name 重复: {name}")
        }
        if !tool.get("inputSchema").is_some_and(Value::is_object) {
            bail!("MCP tool {name} 缺少 inputSchema")
        }
    }
    let bytes = serde_json::to_vec(tools).context("serialize MCP tool contract")?;
    Ok(json!({
        "schema": TOOL_CONTRACT_SCHEMA,
        "digest": hex::encode(Sha256::digest(&bytes)),
        "toolCount": tools.len(),
        "selection": {
            "stableNodeSelector": "definitionId+instanceKey+screenId",
            "ambiguousSelectorPolicy": "REJECT",
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_deterministic_and_rejects_duplicate_names() {
        let tools = vec![json!({"name":"one","inputSchema":{"type":"object"}})];
        assert_eq!(manifest(&tools).unwrap(), manifest(&tools).unwrap());
        assert!(manifest(&[tools[0].clone(), tools[0].clone()]).is_err());
    }
}
