//! Deterministic, metadata-only project knowledge architecture diagnostics.

use homecli_proto::ProjectDocumentEntry;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::{
    project_document_governance::DocumentSectionManifest,
    project_document_governance_facets::effective_facets_with_metadata,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RecommendedKnowledgeSection {
    pub id: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct FoundationCoverage {
    pub doc_type: &'static str,
    pub label: &'static str,
    pub covered: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct KnowledgeArchitectureHealth {
    pub profile: String,
    pub profile_label: &'static str,
    pub profile_source: &'static str,
    pub score: u8,
    pub status: &'static str,
    pub topic_sections: usize,
    pub topic_assigned_documents: usize,
    pub topic_unassigned_documents: usize,
    pub ambiguous_documents: usize,
    pub outdated_documents: usize,
    pub duplicate_titles: usize,
    pub home_configured: bool,
    pub foundation_coverage: Vec<FoundationCoverage>,
    pub missing_document_types: Vec<&'static str>,
    pub findings: Vec<String>,
    pub recommended_sections: Vec<RecommendedKnowledgeSection>,
}

pub(crate) fn analyze_knowledge_architecture(
    documents: &[ProjectDocumentEntry],
    manifest: &DocumentSectionManifest,
) -> KnowledgeArchitectureHealth {
    let (profile, profile_source) = if manifest.profile != "auto" {
        (manifest.profile.clone(), "manifest")
    } else {
        (infer_profile(documents), "metadata")
    };
    let foundations = foundation_specs(&profile);
    let foundation_coverage = foundations
        .iter()
        .map(|(doc_type, label, aliases)| FoundationCoverage {
            doc_type,
            label,
            covered: foundation_is_covered(documents, manifest, doc_type, aliases),
        })
        .collect::<Vec<_>>();
    let missing_document_types = foundation_coverage
        .iter()
        .filter(|item| !item.covered)
        .map(|item| item.doc_type)
        .collect::<Vec<_>>();
    let known_paths = documents
        .iter()
        .map(|document| normalize(&document.path))
        .collect::<HashSet<_>>();
    let explicitly_assigned_documents = manifest
        .assignments
        .iter()
        .filter(|(path, section)| {
            section.starts_with("custom:") && known_paths.contains(&normalize(path))
        })
        .count();
    // Every catalog entry receives a deterministic template topic. Explicit
    // assignments are pins, not a prerequisite for being part of the tree.
    let topic_assigned_documents = documents.len();
    let topic_unassigned_documents = 0;
    let ambiguous_documents = documents
        .iter()
        .filter(|document| {
            let path = document.path.replace('\\', "/");
            let facets = effective_facets_with_metadata(
                document,
                manifest.governance_facets.get(&path),
                manifest.document_metadata.get(&path),
            );
            facets.lifecycle == "unclassified" || facets.authority == "unknown"
        })
        .count();
    let outdated_documents = documents
        .iter()
        .filter(|document| {
            matches!(
                document.metadata.lifecycle.as_str(),
                "deprecated" | "superseded"
            )
        })
        .count();
    let duplicate_titles = duplicate_title_count(documents, manifest);
    let home_configured = !manifest.home.title.is_empty()
        && !manifest.home.summary.is_empty()
        && (!manifest.home.entrypoint.is_empty() || !manifest.home.start_here.is_empty());
    let score = health_score(
        documents.len(),
        topic_assigned_documents,
        ambiguous_documents,
        home_configured,
        foundation_coverage
            .iter()
            .filter(|item| item.covered)
            .count(),
        foundation_coverage.len(),
        profile_source,
    );
    let mut findings = Vec::new();
    if profile_source == "metadata" {
        findings.push(format!(
            "项目类型由路径和标题推断为“{}”，尚未固化。",
            profile_label(&profile)
        ));
    }
    if !home_configured {
        findings.push("缺少面向读者的项目知识首页、摘要或推荐阅读入口。".to_string());
    }
    if !missing_document_types.is_empty() {
        findings.push(format!(
            "缺少 {} 类基础文档：{}。",
            missing_document_types.len(),
            missing_document_types.join("、")
        ));
    }
    if explicitly_assigned_documents < documents.len() && manifest.sections.is_empty() {
        findings.push("当前使用项目模板自动构建主题树；可将核心入口固定到自定义分区。".to_string());
    }
    if duplicate_titles > 0 {
        findings.push(format!(
            "发现 {duplicate_titles} 组重复或近似标题，需要确认唯一入口。"
        ));
    }
    if ambiguous_documents > 0 {
        findings.push(format!(
            "仍有 {ambiguous_documents} 份文档无法从路径确定权威性。"
        ));
    }

    KnowledgeArchitectureHealth {
        profile: profile.clone(),
        profile_label: profile_label(&profile),
        profile_source,
        score,
        status: if score >= 85 {
            "healthy"
        } else if score >= 60 {
            "needs_attention"
        } else {
            "needs_structure"
        },
        topic_sections: manifest.sections.len(),
        topic_assigned_documents,
        topic_unassigned_documents,
        ambiguous_documents,
        outdated_documents,
        duplicate_titles,
        home_configured,
        foundation_coverage,
        missing_document_types,
        findings,
        recommended_sections: recommended_sections(&profile),
    }
}

fn infer_profile(documents: &[ProjectDocumentEntry]) -> String {
    let haystack = documents
        .iter()
        .flat_map(|document| {
            std::iter::once(document.path.as_str())
                .chain(std::iter::once(document.title.as_str()))
                .chain(document.metadata.headings.iter().map(String::as_str))
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let has = |terms: &[&str]| {
        terms
            .iter()
            .filter(|term| haystack.contains(**term))
            .count()
    };
    if has(&["android", "pc-frontend", "node-agent", "server/src"]) >= 3 {
        "software-platform".to_string()
    } else if has(&["api", "openapi", "endpoint", "sdk", "接口"]) >= 2 {
        "software-api".to_string()
    } else if has(&[
        "research",
        "experiment",
        "dataset",
        "literature",
        "实验",
        "研究",
    ]) >= 2
    {
        "research".to_string()
    } else if has(&[
        "runbook",
        "incident",
        "monitor",
        "deployment",
        "运维",
        "故障",
    ]) >= 2
    {
        "operations".to_string()
    } else if has(&[
        "roadmap",
        "requirement",
        "persona",
        "metric",
        "需求",
        "用户",
    ]) >= 2
    {
        "product".to_string()
    } else {
        "personal-knowledge".to_string()
    }
}

fn foundation_is_covered(
    documents: &[ProjectDocumentEntry],
    manifest: &DocumentSectionManifest,
    doc_type: &str,
    aliases: &[&str],
) -> bool {
    if manifest.document_metadata.iter().any(|(path, metadata)| {
        metadata.doc_type == doc_type
            && documents
                .iter()
                .find(|document| normalize(&document.path) == normalize(path))
                .is_some_and(|document| current_for_architecture(document, manifest))
    }) {
        return true;
    }
    if manifest.sections.iter().any(|section| {
        !section.entrypoint.is_empty()
            && aliases.iter().any(|alias| {
                section.id.contains(alias)
                    || section.label.to_ascii_lowercase().contains(alias)
                    || section.entrypoint.to_ascii_lowercase().contains(alias)
            })
            && documents
                .iter()
                .find(|document| normalize(&document.path) == normalize(&section.entrypoint))
                .is_some_and(|document| current_for_architecture(document, manifest))
    }) {
        return true;
    }
    documents
        .iter()
        .filter(|document| current_for_architecture(document, manifest))
        .any(|document| {
            let searchable = format!("{} {}", document.path, document.title).to_ascii_lowercase();
            aliases.iter().any(|alias| searchable.contains(alias))
                || match doc_type {
                    "architecture" => document.metadata.role == "architecture",
                    "operations" => document.metadata.role == "runbook",
                    "requirements" => document.metadata.role == "requirement",
                    "decisions" => document.metadata.role == "decision",
                    _ => false,
                }
        })
}

fn current_for_architecture(
    document: &ProjectDocumentEntry,
    manifest: &DocumentSectionManifest,
) -> bool {
    let path = document.path.replace('\\', "/");
    let facets = effective_facets_with_metadata(
        document,
        manifest.governance_facets.get(&path),
        manifest.document_metadata.get(&path),
    );
    facets.retrieval != "excluded"
        && matches!(facets.lifecycle.as_str(), "active" | "accepted" | "current")
}

fn duplicate_title_count(
    documents: &[ProjectDocumentEntry],
    manifest: &DocumentSectionManifest,
) -> usize {
    let mut counts = HashMap::new();
    for document in documents {
        let path = document.path.replace('\\', "/");
        let facets = effective_facets_with_metadata(
            document,
            manifest.governance_facets.get(&path),
            manifest.document_metadata.get(&path),
        );
        if facets.retrieval == "excluded"
            || !matches!(facets.lifecycle.as_str(), "active" | "accepted" | "current")
        {
            continue;
        }
        let title = document
            .title
            .to_ascii_lowercase()
            .replace([' ', '-', '_', '（', '）', '(', ')'], "");
        if !title.is_empty() {
            *counts.entry(title).or_insert(0usize) += 1;
        }
    }
    counts.values().filter(|count| **count > 1).count()
}

fn health_score(
    document_count: usize,
    assigned: usize,
    ambiguous: usize,
    home_configured: bool,
    covered_foundations: usize,
    foundation_count: usize,
    profile_source: &str,
) -> u8 {
    let profile_score = if profile_source == "manifest" { 15 } else { 8 };
    let home_score = if home_configured { 15 } else { 0 };
    let foundation_score = if foundation_count == 0 {
        30
    } else {
        (covered_foundations * 30 / foundation_count) as u8
    };
    let assignment_score = if document_count == 0 {
        30
    } else {
        (assigned * 30 / document_count) as u8
    };
    let authority_score = if document_count == 0 {
        10
    } else {
        (10usize.saturating_sub(ambiguous * 10 / document_count)) as u8
    };
    profile_score + home_score + foundation_score + assignment_score + authority_score
}

fn profile_label(profile: &str) -> &'static str {
    match profile {
        "software-platform" => "软件平台",
        "software-api" => "API / SDK",
        "product" => "产品与业务",
        "research" => "研究项目",
        "operations" => "运维项目",
        _ => "个人知识库",
    }
}

fn foundation_specs(profile: &str) -> Vec<(&'static str, &'static str, &'static [&'static str])> {
    match profile {
        "software-platform" => vec![
            (
                "overview",
                "项目总览",
                &["overview", "project", "readme", "总览"],
            ),
            ("architecture", "总体架构", &["architecture", "架构"]),
            ("api-reference", "后端与 API", &["api", "backend", "接口"]),
            (
                "client-guides",
                "客户端指南",
                &["android", "pc", "client", "客户端"],
            ),
            (
                "operations",
                "部署与运维",
                &["runbook", "deploy", "release", "发布", "运维"],
            ),
        ],
        "software-api" => vec![
            ("overview", "项目总览", &["overview", "readme", "总览"]),
            (
                "quickstart",
                "快速开始",
                &["quickstart", "getting-started", "快速开始"],
            ),
            ("architecture", "架构", &["architecture", "架构"]),
            (
                "api-reference",
                "API 参考",
                &["api", "openapi", "endpoint", "接口"],
            ),
            (
                "data-model",
                "数据模型",
                &["data-model", "schema", "数据模型"],
            ),
            ("operations", "部署运维", &["runbook", "deploy", "运维"]),
        ],
        "product" => vec![
            ("overview", "产品总览", &["overview", "readme", "总览"]),
            ("users", "用户与场景", &["persona", "user", "用户"]),
            ("requirements", "需求", &["requirement", "spec", "需求"]),
            ("roadmap", "路线图", &["roadmap", "路线图"]),
            ("metrics", "指标", &["metric", "analytics", "指标"]),
        ],
        "research" => vec![
            ("overview", "研究总览", &["overview", "readme", "总览"]),
            ("literature", "文献", &["literature", "文献"]),
            ("methods", "方法", &["method", "方法"]),
            ("experiments", "实验", &["experiment", "实验"]),
            ("results", "结论", &["result", "conclusion", "结论"]),
        ],
        "operations" => vec![
            ("overview", "服务总览", &["overview", "service", "总览"]),
            ("operations", "运行手册", &["runbook", "sop", "手册"]),
            ("monitoring", "监控", &["monitor", "alert", "监控"]),
            (
                "incidents",
                "事故与恢复",
                &["incident", "recovery", "故障", "恢复"],
            ),
            (
                "security",
                "安全",
                &["security", "permission", "安全", "权限"],
            ),
        ],
        _ => vec![
            ("overview", "知识库总览", &["overview", "readme", "总览"]),
            ("topics", "核心主题", &["topic", "主题"]),
            ("guides", "方法与指南", &["guide", "how-to", "指南"]),
            (
                "sources",
                "来源与参考",
                &["reference", "source", "参考", "来源"],
            ),
        ],
    }
}

fn recommended_sections(profile: &str) -> Vec<RecommendedKnowledgeSection> {
    foundation_specs(profile)
        .into_iter()
        .map(|(id, label, _)| RecommendedKnowledgeSection {
            id,
            label,
            detail: match id {
                "overview" => "项目定位、范围和推荐阅读入口",
                "quickstart" => "最短可执行上手路径",
                "architecture" => "系统边界、模块和关键数据流",
                "api-reference" => "接口、参数、错误码和示例",
                "data-model" => "实体、字段和关系",
                "operations" => "部署、发布、维护和故障恢复",
                "requirements" => "已批准需求和验收标准",
                "decisions" => "已接受决策及其原因",
                _ => "该项目类型需要的核心知识主题",
            },
        })
        .collect()
}

fn normalize(value: &str) -> String {
    value.trim().replace('\\', "/").to_ascii_lowercase()
}

#[cfg(test)]
#[path = "project_document_architecture_tests.rs"]
mod tests;
