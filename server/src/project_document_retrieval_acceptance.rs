//! Repeatable retrieval acceptance cases for AI-facing project documentation.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::Path};

use crate::project_document_knowledge_graph_service::plan_context;

pub(crate) const RETRIEVAL_CASES_PATH: &str = ".elon/document-retrieval-cases.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct RetrievalAcceptanceCase {
    pub id: String,
    pub query: String,
    #[serde(default)]
    pub node_id: Option<String>,
    pub expected_paths: Vec<String>,
    #[serde(default)]
    pub forbidden_paths: Vec<String>,
    #[serde(default)]
    pub require_first: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RetrievalAcceptanceManifest {
    #[serde(default)]
    version: u8,
    cases: Vec<RetrievalAcceptanceCase>,
}

pub(crate) fn test_document_retrieval(
    workspace: &Path,
    inline_cases: Option<Vec<RetrievalAcceptanceCase>>,
    max_tokens: u64,
    max_documents: usize,
) -> Result<Value> {
    let (source, cases) = match inline_cases {
        Some(cases) if !cases.is_empty() => ("inline", cases),
        _ => {
            let path = workspace.join(RETRIEVAL_CASES_PATH);
            let content = fs::read_to_string(&path)
                .with_context(|| format!("读取 {} 失败", RETRIEVAL_CASES_PATH))?;
            let manifest: RetrievalAcceptanceManifest = serde_json::from_str(&content)
                .with_context(|| format!("解析 {} 失败", RETRIEVAL_CASES_PATH))?;
            if manifest.version != 1 {
                bail!("{} version 必须为 1", RETRIEVAL_CASES_PATH);
            }
            (RETRIEVAL_CASES_PATH, manifest.cases)
        }
    };
    if cases.is_empty() || cases.len() > 20 {
        bail!("检索验收用例数量必须为 1 到 20");
    }
    let mut results = Vec::new();
    for case in cases {
        if case.id.trim().is_empty()
            || case.query.trim().is_empty()
            || case.expected_paths.is_empty()
        {
            bail!("每个检索验收用例必须包含 id、query 和 expected_paths");
        }
        let plan = plan_context(
            workspace,
            &case.query,
            case.node_id.as_deref(),
            max_tokens,
            max_documents,
            2_000,
        )?;
        let selected_documents = plan["relevant_documents"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| {
                item.pointer("/document/path")
                    .and_then(Value::as_str)
                    .map(|path| {
                        json!({
                            "path": normalize(path),
                            "score": item["score"],
                            "reason": item["reason"],
                            "read_mode": item.pointer("/read_plan/mode"),
                        })
                    })
            })
            .chain(
                plan["mandatory_rules"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|item| {
                        item.pointer("/document/path")
                            .and_then(Value::as_str)
                            .map(|path| {
                                json!({
                                    "path": normalize(path),
                                    "reason": item["reason"],
                                    "mandatory": true,
                                })
                            })
                    }),
            )
            .collect::<Vec<_>>();
        let selected_paths = selected_documents
            .iter()
            .filter_map(|item| item["path"].as_str().map(str::to_string))
            .collect::<Vec<_>>();
        let missing_paths = case
            .expected_paths
            .iter()
            .filter(|path| !selected_paths.contains(&normalize(path)))
            .cloned()
            .collect::<Vec<_>>();
        let forbidden_hits = case
            .forbidden_paths
            .iter()
            .filter(|path| selected_paths.contains(&normalize(path)))
            .cloned()
            .collect::<Vec<_>>();
        let first_mismatch = case
            .require_first
            .as_ref()
            .filter(|path| selected_paths.first() != Some(&normalize(path)))
            .cloned();
        let passed =
            missing_paths.is_empty() && forbidden_hits.is_empty() && first_mismatch.is_none();
        results.push(json!({
            "id": case.id,
            "passed": passed,
            "query": case.query,
            "expected_paths": case.expected_paths,
            "selected_paths": selected_paths,
            "selected_documents": selected_documents,
            "matched_nodes": plan["matched_nodes"],
            "missing_paths": missing_paths,
            "forbidden_hits": forbidden_hits,
            "first_mismatch": first_mismatch,
        }));
    }
    let passed = results
        .iter()
        .filter(|result| result["passed"] == true)
        .count();
    Ok(json!({
        "source": source,
        "summary": {
            "total": results.len(),
            "passed": passed,
            "failed": results.len().saturating_sub(passed),
            "success": passed == results.len(),
        },
        "results": results,
        "budget": {
            "classification_model_tokens": 0,
            "markdown_bodies_read": 0,
            "max_tokens_per_case": max_tokens.clamp(200, 12_000),
            "max_documents_per_case": max_documents.clamp(1, 24),
        },
    }))
}

fn normalize(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .trim()
        .replace('\\', "/")
        .to_ascii_lowercase()
}

#[cfg(test)]
#[path = "project_document_retrieval_acceptance_tests.rs"]
mod tests;
