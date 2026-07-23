//! Parent aggregation and explainable health scoring for project knowledge maps.

use homecli_proto::ProjectDocumentEntry;
use std::collections::HashMap;

use crate::{
    project_document_knowledge_graph::coverage_for_paths,
    project_document_knowledge_graph_model::{
        KnowledgeMapFinding, KnowledgeMapStats, ProjectKnowledgeMapNode,
    },
};

pub(crate) fn view_needs_implementation(view: &str) -> bool {
    matches!(view, "capabilities" | "architecture")
}

pub(crate) struct KnowledgeMapScore {
    pub structural: u8,
    pub finding: u8,
    pub documentation: u8,
    pub implementation: Option<u8>,
    pub formula: &'static str,
}

pub(crate) fn aggregate_child_knowledge(
    nodes: &mut [ProjectKnowledgeMapNode],
    by_path: &HashMap<String, &ProjectDocumentEntry>,
    view: &str,
) {
    let mut indexes = (0..nodes.len()).collect::<Vec<_>>();
    indexes.sort_by_key(|index| std::cmp::Reverse(nodes[*index].depth));
    for child_index in indexes {
        let parent_id = nodes[child_index].parent_id.clone();
        let Some(parent_index) = nodes.iter().position(|node| node.id == parent_id) else {
            continue;
        };
        let child_paths = nodes[child_index].document_paths.clone();
        let child_evidence = nodes[child_index].implementation_refs.clone();
        let child_entrypoint = nodes[child_index].entrypoint.clone();
        let parent = &mut nodes[parent_index];
        parent.document_paths.extend(child_paths);
        parent.document_paths.sort();
        parent.document_paths.dedup();
        for evidence in child_evidence {
            if !parent
                .implementation_refs
                .iter()
                .any(|item| item.reference == evidence.reference)
            {
                parent.implementation_refs.push(evidence);
            }
        }
        if parent.entrypoint.is_empty() && !child_entrypoint.is_empty() {
            parent.entrypoint = child_entrypoint;
            parent.entrypoint_source = "child_aggregate".to_string();
        }
        parent.document_count = parent.document_paths.len();
        parent.coverage = coverage_for_paths(
            &parent.document_paths,
            by_path,
            !parent.entrypoint.is_empty(),
        );
        parent.missing_coverage = parent
            .coverage
            .iter()
            .filter(|item| !item.covered)
            .map(|item| item.label.to_string())
            .collect();
        let covered = parent.coverage.iter().filter(|item| item.covered).count();
        parent.documentation_status = if parent.document_paths.is_empty() {
            "undocumented"
        } else if parent.entrypoint_source == "configured" || covered >= 3 {
            "documented"
        } else {
            "partial"
        }
        .to_string();
        parent.implementation_status = implementation_status(parent, view).to_string();
    }
}

pub(crate) fn append_status_findings(
    nodes: &[ProjectKnowledgeMapNode],
    findings: &mut Vec<KnowledgeMapFinding>,
) {
    for node in nodes {
        if node.documentation_status == "undocumented" {
            findings.push(finding(
                "undocumented_node",
                "warning",
                node,
                "节点及其子节点都没有关联当前 Markdown。",
                "关联现有权威文档，确有缺口时再新增文档。",
            ));
        }
        if node.entrypoint.is_empty() {
            findings.push(finding(
                "missing_entrypoint",
                "info",
                node,
                "节点及其子节点缺少推荐入口文档。",
                "从关联文档中指定唯一入口。",
            ));
        }
        if node.implementation_status == "missing" {
            let stale = node
                .implementation_refs
                .iter()
                .any(|item| item.verification == "missing");
            findings.push(finding(
                if stale {
                    "stale_implementation_evidence"
                } else {
                    "missing_implementation_evidence"
                },
                "warning",
                node,
                if stale {
                    "节点或子节点的部分实现引用已经失效。"
                } else {
                    "节点及其子节点没有代码、API 或测试证据。"
                },
                "添加或修正 file:/route:/symbol:/test: 引用。",
            ));
        }
    }
}

pub(crate) fn score_map(
    stats: &KnowledgeMapStats,
    findings: &[KnowledgeMapFinding],
    view: &str,
) -> KnowledgeMapScore {
    let finding = (100_i32
        - findings
            .iter()
            .map(|item| match item.severity {
                "error" => 15,
                "warning" => 8,
                _ => 2,
            })
            .sum::<i32>())
    .clamp(0, 100) as u8;
    let documentation = if stats.nodes == 0 {
        0
    } else {
        ((stats.documented * 100 + stats.partial * 50) / stats.nodes) as u8
    };
    let implementation = view_needs_implementation(view).then(|| {
        if stats.nodes == 0 {
            0
        } else {
            ((stats.implementation_verified * 100 + stats.implementation_declared * 60)
                / stats.nodes) as u8
        }
    });
    let (structural, formula) = implementation.map_or_else(
        || {
            (
                ((u16::from(finding) * 60 + u16::from(documentation) * 40) / 100) as u8,
                "finding*60% + documentation*40%",
            )
        },
        |implementation| {
            (
                ((u16::from(finding) * 35
                    + u16::from(documentation) * 30
                    + u16::from(implementation) * 35)
                    / 100) as u8,
                "finding*35% + documentation*30% + implementation*35%",
            )
        },
    );
    KnowledgeMapScore {
        structural,
        finding,
        documentation,
        implementation,
        formula,
    }
}

fn implementation_status(node: &ProjectKnowledgeMapNode, view: &str) -> &'static str {
    if !view_needs_implementation(view) {
        "not_applicable"
    } else if node.implementation_refs.is_empty()
        || node
            .implementation_refs
            .iter()
            .any(|item| item.verification == "missing")
    {
        "missing"
    } else if node
        .implementation_refs
        .iter()
        .any(|item| item.verification == "declared")
    {
        "declared"
    } else {
        "verified"
    }
}

fn finding(
    code: &'static str,
    severity: &'static str,
    node: &ProjectKnowledgeMapNode,
    message: &str,
    suggested_action: &'static str,
) -> KnowledgeMapFinding {
    KnowledgeMapFinding {
        code,
        severity,
        node_id: node.id.clone(),
        message: message.to_string(),
        suggested_action,
    }
}
