//! Read-only document health projection for a selected federation scope.

use anyhow::Result;
use homecli_proto::ProjectDocumentEntry;
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};

use crate::{
    project_document_architecture::analyze_knowledge_architecture,
    project_document_governance::DocumentSectionManifest,
    project_document_index::ProjectDocumentIndex,
    project_document_issue_workflow::list_filtered,
    project_document_maintenance::{health_status, weighted_score},
    project_document_quality::{analyze_document_quality, compact_report, filter_document_quality},
};

pub(crate) fn analyze_scoped_document_health(
    workspace: &Path,
    all_documents: &[ProjectDocumentEntry],
    documents: &[ProjectDocumentEntry],
    manifest: &DocumentSectionManifest,
    global_analysis: &Value,
    scope_id: &str,
) -> Result<Value> {
    let index = ProjectDocumentIndex::open(workspace)?;
    let global_quality = analyze_document_quality(workspace, all_documents, manifest, &index)?;
    let architecture = analyze_knowledge_architecture(documents, manifest);
    let federation_node = global_analysis
        .pointer("/federation/nodes")
        .and_then(Value::as_array)
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|node| node.get("id").and_then(Value::as_str) == Some(scope_id))
        })
        .cloned()
        .unwrap_or(Value::Null);
    let federation_score = federation_node
        .get("score")
        .and_then(Value::as_u64)
        .unwrap_or(architecture.score.into()) as u8;
    let paths = documents
        .iter()
        .map(|document| normalize(&document.path))
        .collect::<HashSet<_>>();
    let quality = filter_document_quality(&global_quality, &paths);
    let overall_score = weighted_score(architecture.score, quality.summary.score, federation_score);
    let workflow_issues = list_filtered(&index, &[], &[], &[], "", 0, 100_000)?
        .into_iter()
        .filter(|issue| {
            issue
                .get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| paths.contains(&normalize(path)))
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "version": 1,
        "source": "server",
        "projection": "federation_scope",
        "scope_id": scope_id,
        "overall": {
            "score": overall_score,
            "status": health_status(overall_score),
        },
        "architecture": architecture,
        "identity": global_analysis.get("identity").cloned().unwrap_or(Value::Null),
        "quality": compact_report(&quality, 40),
        "governance_workflow": {
            "version": 1,
            "summary": workflow_summary(&workflow_issues),
            "issues": workflow_issues.iter().take(100).collect::<Vec<_>>(),
            "returned_issues": workflow_issues.len().min(100),
            "total_issues": workflow_issues.len(),
            "read_only_projection": true,
        },
        "maintenance": global_analysis.get("maintenance").cloned().unwrap_or(Value::Null),
        "federation": {
            "scope_id": scope_id,
            "aggregated_score": federation_score,
            "selected_node": federation_node,
        },
    }))
}

fn workflow_summary(issues: &[Value]) -> Value {
    let count = |status: &str| {
        issues
            .iter()
            .filter(|issue| {
                issue
                    .pointer("/workflow/status")
                    .and_then(Value::as_str)
                    .unwrap_or("open")
                    == status
            })
            .count()
    };
    let ignored = count("ignored");
    let snoozed = count("snoozed");
    let resolved = count("resolved");
    json!({
        "open": count("open"),
        "assigned": count("assigned"),
        "snoozed": snoozed,
        "ignored": ignored,
        "resolved": resolved,
        "actionable": issues.len().saturating_sub(ignored + snoozed + resolved),
    })
}

fn normalize(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}
