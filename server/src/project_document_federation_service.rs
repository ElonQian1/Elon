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
        "limits": {"indexed_nodes":4096,"transport_page":200,"legacy_section_limit_applies":false,"legacy_document_limit_applies":false},
        "projection": {"mode":request.projection,"detail":request.detail},
    }))
}

/// Catalog responses carry federation totals only; tree nodes have their own paged contract.
pub(crate) fn strip_catalog_nodes(analysis: &mut Value) {
    // `ProjectDocumentsSnapshot.analysis` is the document-health object itself,
    // while bounded MCP responses wrap the same object under `document_health`.
    // Accept both shapes so every catalog transport drops the full node array.
    let federation = if analysis
        .get("federation")
        .and_then(Value::as_object)
        .is_some()
    {
        analysis.get_mut("federation")
    } else {
        analysis
            .get_mut("document_health")
            .and_then(|health| health.get_mut("federation"))
    };
    let Some(federation) = federation.and_then(Value::as_object_mut) else {
        return;
    };
    federation.insert("nodes".to_string(), json!([]));
    federation.insert(
        "nodes_transport".to_string(),
        json!({
            "mode": "server_paged",
            "endpoint": "docs/federation",
            "page_limit": 200,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_docs_scan::{
        collect_project_documents_with_options, ProjectDocumentScanOptions,
    };
    use std::fs;

    #[test]
    fn parent_cursor_pages_are_independent_and_catalog_is_summary_only() {
        let root =
            std::env::temp_dir().join(format!("elon-federation-page-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join(".elon")).unwrap();
        fs::write(root.join("AGENTS.md"), "# Root").unwrap();
        fs::write(
            root.join(crate::project_document_federation::FEDERATION_CONFIG_PATH),
            r#"{
          "version":1,"nodes":[
            {"id":"root","label":"Root"},
            {"id":"apps","label":"Apps","parent_id":"root"},
            {"id":"docs","label":"Docs","parent_id":"root"},
            {"id":"android","label":"Android","parent_id":"apps"}
          ]
        }"#,
        )
        .unwrap();
        let first =
            ProjectionRequest::from_arguments(&json!({"projection":"page","limit":1})).unwrap();
        let root_page = get_federation_index(&root, None, None, &first).unwrap();
        assert_eq!(root_page["nodes"][0]["id"], "root");
        let children = get_federation_index(&root, Some("root"), None, &first).unwrap();
        assert_eq!(children["nodes"][0]["id"], "apps");
        assert_eq!(children["pagination"]["has_more"], true);
        let next = ProjectionRequest::from_arguments(&json!({
            "projection":"page","limit":1,"cursor":children["pagination"]["next_cursor"]
        }))
        .unwrap();
        let second_child = get_federation_index(&root, Some("root"), None, &next).unwrap();
        assert_eq!(second_child["nodes"][0]["id"], "docs");
        let grandchild = get_federation_index(&root, Some("apps"), None, &first).unwrap();
        assert_eq!(grandchild["nodes"][0]["id"], "android");

        let mut snapshot = collect_project_documents_with_options(
            &root,
            ProjectDocumentScanOptions {
                seed_missing_defaults: false,
                catalog_only: true,
                include_analysis: true,
            },
        )
        .unwrap();
        assert_eq!(
            snapshot.analysis["federation"]["nodes"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        strip_catalog_nodes(&mut snapshot.analysis);
        assert_eq!(snapshot.analysis["federation"]["nodes"], json!([]));
        assert_eq!(snapshot.analysis["federation"]["node_count"], 4);
        assert_eq!(
            snapshot.analysis["federation"]["nodes_transport"]["mode"],
            "server_paged"
        );

        let mut wrapped =
            json!({"document_health":{"federation":{"node_count":4,"nodes":[1,2,3,4]}}});
        strip_catalog_nodes(&mut wrapped);
        assert_eq!(wrapped["document_health"]["federation"]["nodes"], json!([]));
        let _ = fs::remove_dir_all(root);
    }
}
