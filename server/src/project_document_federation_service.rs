//! Pageable, lazy federation index shared by MCP and future UI transports.

use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;

use crate::{
    project_document_governance_service::analyze_workspace,
    project_document_response::{pagination, ProjectionRequest},
};

pub(crate) fn get_federation_index(
    workspace: &Path,
    parent_id: Option<&str>,
    query: Option<&str>,
    request: &ProjectionRequest,
) -> Result<Value> {
    let analysis = analyze_workspace(workspace, 0, 1, false)?;
    let health = analysis
        .get("document_health")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let federation = health
        .get("federation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let all_nodes = federation
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let normalized_parent = parent_id.unwrap_or_default().trim();
    let query = query.unwrap_or_default().trim().to_ascii_lowercase();
    let filtered = all_nodes
        .iter()
        .filter(|node| {
            if !query.is_empty() {
                return format!(
                    "{} {} {} {}",
                    node["id"].as_str().unwrap_or_default(),
                    node["label"].as_str().unwrap_or_default(),
                    node["scope_path"].as_str().unwrap_or_default(),
                    node["owner"].as_str().unwrap_or_default(),
                )
                .to_ascii_lowercase()
                .contains(&query);
            }
            node["parent_id"].as_str().unwrap_or_default() == normalized_parent
        })
        .cloned()
        .collect::<Vec<_>>();
    let page = if request.projection == "summary" {
        Vec::new()
    } else {
        filtered
            .iter()
            .skip(request.offset)
            .take(request.limit)
            .cloned()
            .collect::<Vec<_>>()
    };
    let returned = page.len();
    Ok(json!({
        "catalog_revision": analysis["catalog_revision"],
        "identity": health["identity"],
        "federation": {
            "enabled": federation["enabled"], "source": federation["source"],
            "root_id": federation["root_id"], "node_count": federation["node_count"],
            "aggregated_score": federation["aggregated_score"],
            "unhealthy_nodes": federation["unhealthy_nodes"], "max_depth": federation["max_depth"],
        },
        "selection": {"parent_id":normalized_parent,"query":query,"lazy":true},
        "nodes": page,
        "pagination": pagination(request.offset, request.limit, filtered.len(), returned),
        "limits": {"manifest_nodes":256,"transport_page":200,"legacy_section_limit_applies":false,"legacy_document_limit_applies":false},
        "projection": {"mode":request.projection,"detail":request.detail},
    }))
}
