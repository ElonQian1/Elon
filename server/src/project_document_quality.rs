//! Deterministic document quality checks with evidence suitable for AI review.

use anyhow::Result;
use homecli_proto::ProjectDocumentEntry;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{
    project_document_governance::DocumentSectionManifest,
    project_document_index::ProjectDocumentIndex,
    project_document_quality_rules::{
        check_implementation_refs, eligible_for_governance, eligible_for_orphan_check, load_facts,
        manifest_entrypoints, normalize, resolve_link_target, review_is_overdue,
        DocumentQualityFacts,
    },
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DocumentQualityIssue {
    pub fingerprint: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub severity: String,
    pub path: String,
    pub message: String,
    pub evidence: String,
    pub suggested_action: String,
    pub confidence: u8,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct DocumentQualitySummary {
    pub score: u8,
    pub status: &'static str,
    pub total_issues: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub broken_links: usize,
    pub orphan_documents: usize,
    pub missing_owners: usize,
    pub missing_review_dates: usize,
    pub overdue_reviews: usize,
    pub implementation_conflicts: usize,
    pub external_links_checked: usize,
    pub external_links_pending: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DocumentQualityReport {
    pub summary: DocumentQualitySummary,
    pub issues: Vec<DocumentQualityIssue>,
    pub issue_types: Vec<&'static str>,
}

pub(crate) fn analyze_document_quality(
    workspace: &Path,
    documents: &[ProjectDocumentEntry],
    manifest: &DocumentSectionManifest,
    index: &ProjectDocumentIndex,
) -> Result<DocumentQualityReport> {
    let known = documents
        .iter()
        .map(|document| (normalize(&document.path).to_ascii_lowercase(), document))
        .collect::<HashMap<_, _>>();
    let mut facts = HashMap::new();
    for document in documents {
        facts.insert(
            normalize(&document.path),
            load_facts(workspace, document, index)?,
        );
    }

    let mut inbound = HashMap::<String, usize>::new();
    let mut issues = Vec::new();
    let mut external_urls = HashSet::new();
    for document in documents {
        let path = normalize(&document.path);
        let Some(document_facts) = facts.get(&path) else {
            continue;
        };
        external_urls.extend(document_facts.external_links.iter().cloned());
        for raw_target in &document_facts.local_links {
            let (target_path, anchor) = resolve_link_target(&path, raw_target);
            let target_key = target_path.to_ascii_lowercase();
            if target_path.is_empty() {
                continue;
            }
            let Some(target_document) = known.get(&target_key) else {
                issues.push(make_issue(
                    "broken_link",
                    "error",
                    &path,
                    format!("链接目标不存在：{raw_target}"),
                    format!("解析后的项目路径为 {target_path}"),
                    "更新链接或恢复目标文档",
                    100,
                ));
                continue;
            };
            *inbound.entry(normalize(&target_document.path)).or_default() += 1;
            if !anchor.is_empty() {
                let target_facts = facts.get(&normalize(&target_document.path));
                if !target_facts.is_some_and(|value| value.anchors.contains(&anchor)) {
                    issues.push(make_issue(
                        "broken_anchor",
                        "warning",
                        &path,
                        format!("标题锚点不存在：#{anchor}"),
                        format!("链接目标为 {}", target_document.path),
                        "更新标题锚点或目标文档标题",
                        100,
                    ));
                }
            }
        }
    }

    let entrypoints = manifest_entrypoints(manifest);
    let mut implementation_cache = HashMap::new();
    for document in documents {
        let path = normalize(&document.path);
        let metadata = manifest
            .document_metadata
            .get(&path)
            .cloned()
            .unwrap_or_default();
        if eligible_for_governance(document) {
            if metadata.owner.trim().is_empty() && metadata.owners.is_empty() {
                issues.push(make_issue(
                    "missing_owner",
                    "warning",
                    &path,
                    "当前知识入口没有维护负责人".to_string(),
                    format!(
                        "role={} lifecycle={}",
                        document.metadata.role, document.metadata.lifecycle
                    ),
                    "为文档设置用户或团队 owner",
                    100,
                ));
            }
            if metadata.reviewed_at.trim().is_empty() {
                issues.push(make_issue(
                    "missing_review_date",
                    "warning",
                    &path,
                    "当前知识入口没有复查日期".to_string(),
                    "无法判断实现变化后是否仍然有效".to_string(),
                    "设置 reviewed_at 和 review_interval_days",
                    100,
                ));
            } else if review_is_overdue(&metadata) {
                issues.push(make_issue(
                    "overdue_review",
                    "warning",
                    &path,
                    "文档已经超过复查周期".to_string(),
                    format!(
                        "reviewed_at={} interval={}天",
                        metadata.reviewed_at, metadata.review_interval_days
                    ),
                    "结合关联实现完成复查并更新日期",
                    100,
                ));
            }
        }
        if eligible_for_orphan_check(document)
            && inbound.get(&path).copied().unwrap_or_default() == 0
            && !entrypoints.contains(&path)
            && metadata.related.is_empty()
            && metadata.supersedes.is_empty()
        {
            issues.push(make_issue(
                "orphan_document",
                "info",
                &path,
                "文档没有知识入口、入链或显式关系".to_string(),
                "它可能无法从项目阅读地图被发现".to_string(),
                "加入主题入口、推荐阅读或建立 related 关系",
                95,
            ));
        }
        check_implementation_refs(
            workspace,
            &path,
            &metadata,
            &mut implementation_cache,
            &mut issues,
        );
    }

    for url in &external_urls {
        if let Some((status, error)) = index.external_link_status(url)? {
            if status.is_some_and(|value| value >= 400) || error.is_some() {
                issues.push(make_issue(
                    "broken_external_link",
                    "warning",
                    external_link_owner(&facts, url),
                    format!("外部链接不可用：{url}"),
                    error.unwrap_or_else(|| format!("HTTP {}", status.unwrap_or_default())),
                    "更新链接或标记为历史来源",
                    90,
                ));
            }
        }
    }

    issues.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.issue_type.cmp(&right.issue_type))
    });
    let summary = summarize(
        &issues,
        external_urls.len(),
        index.checked_external_links()?,
    );
    Ok(DocumentQualityReport {
        summary,
        issues,
        issue_types: vec![
            "broken_link",
            "broken_anchor",
            "broken_external_link",
            "orphan_document",
            "missing_owner",
            "missing_review_date",
            "overdue_review",
            "implementation_conflict",
            "implementation_drift",
        ],
    })
}

pub(super) fn make_issue(
    issue_type: &str,
    severity: &str,
    path: impl AsRef<str>,
    message: String,
    evidence: String,
    suggested_action: &str,
    confidence: u8,
) -> DocumentQualityIssue {
    let path = path.as_ref().to_string();
    let mut hasher = Sha256::new();
    hasher.update(issue_type.as_bytes());
    hasher.update([0]);
    hasher.update(path.as_bytes());
    hasher.update([0]);
    hasher.update(message.as_bytes());
    DocumentQualityIssue {
        fingerprint: format!("{:x}", hasher.finalize()),
        issue_type: issue_type.to_string(),
        severity: severity.to_string(),
        path,
        message,
        evidence,
        suggested_action: suggested_action.to_string(),
        confidence,
    }
}

fn summarize(
    issues: &[DocumentQualityIssue],
    external_total: usize,
    external_checked: usize,
) -> DocumentQualitySummary {
    let count = |kind: &str| {
        issues
            .iter()
            .filter(|issue| issue.issue_type == kind)
            .count()
    };
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .count();
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity == "warning")
        .count();
    let info = issues.len().saturating_sub(errors + warnings);
    let score = 100usize
        .saturating_sub(errors.saturating_mul(8))
        .saturating_sub(warnings.saturating_mul(3))
        .saturating_sub(info.min(10)) as u8;
    DocumentQualitySummary {
        score,
        status: if errors > 0 {
            "needs_attention"
        } else if warnings > 0 {
            "review"
        } else {
            "healthy"
        },
        total_issues: issues.len(),
        errors,
        warnings,
        info,
        broken_links: count("broken_link") + count("broken_anchor") + count("broken_external_link"),
        orphan_documents: count("orphan_document"),
        missing_owners: count("missing_owner"),
        missing_review_dates: count("missing_review_date"),
        overdue_reviews: count("overdue_review"),
        implementation_conflicts: count("implementation_conflict") + count("implementation_drift"),
        external_links_checked: external_checked.min(external_total),
        external_links_pending: external_total.saturating_sub(external_checked),
    }
}

fn external_link_owner<'a>(facts: &'a HashMap<String, DocumentQualityFacts>, url: &str) -> &'a str {
    facts
        .iter()
        .find(|(_, value)| value.external_links.iter().any(|link| link == url))
        .map(|(path, _)| path.as_str())
        .unwrap_or("external-link")
}

fn severity_rank(value: &str) -> u8 {
    match value {
        "error" => 0,
        "warning" => 1,
        _ => 2,
    }
}

pub(crate) fn compact_report(report: &DocumentQualityReport, limit: usize) -> Value {
    json!({
        "summary": report.summary,
        "issues": report.issues.iter().take(limit).collect::<Vec<_>>(),
        "returned_issues": report.issues.len().min(limit),
        "total_issues": report.issues.len(),
        "issue_types": report.issue_types,
    })
}

#[cfg(test)]
#[path = "project_document_quality_tests.rs"]
mod tests;
