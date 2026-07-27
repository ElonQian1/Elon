//! Independent governance facets and cross-document relations.

use anyhow::{bail, Result};
use homecli_proto::ProjectDocumentEntry;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use crate::project_document_file_operation_model::normalize_document_path;

const RETRIEVAL_VALUES: &[&str] = &["required", "on_demand", "excluded"];
const LIFECYCLE_VALUES: &[&str] = &[
    "active",
    "accepted",
    "source_material",
    "draft",
    "deprecated",
    "superseded",
    "archived",
    "unclassified",
];
const AUTHORITY_VALUES: &[&str] = &[
    "binding",
    "authoritative",
    "guidance",
    "evidence",
    "proposal",
    "non_authoritative",
    "none",
    "unknown",
];
const RELATION_VALUES: &[&str] = &[
    "related",
    "supports",
    "depends_on",
    "implements",
    "evidence_for",
    "supersedes",
    "replaced_by",
    "see_also",
];
const VERSION_STATUS_VALUES: &[&str] =
    &["current", "draft", "deprecated", "superseded", "archived"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentGovernanceFacets {
    #[serde(default)]
    pub retrieval: String,
    #[serde(default)]
    pub lifecycle: String,
    #[serde(default)]
    pub authority: String,
    #[serde(default)]
    pub document_type: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentRelation {
    #[serde(default)]
    pub relation: String,
    #[serde(default)]
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentKnowledgeMetadata {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub doc_type: String,
    #[serde(default)]
    pub audience: Vec<String>,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub owners: Vec<String>,
    #[serde(default)]
    pub reviewed_at: String,
    #[serde(default = "default_review_interval_days")]
    pub review_interval_days: u16,
    #[serde(default)]
    pub implementation_refs: Vec<String>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub version_status: String,
    #[serde(default)]
    pub related: Vec<String>,
    #[serde(default)]
    pub supersedes: Vec<String>,
    #[serde(default)]
    pub relations: Vec<DocumentRelation>,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub pinned: bool,
}

impl Default for DocumentKnowledgeMetadata {
    fn default() -> Self {
        Self {
            id: String::new(),
            doc_type: String::new(),
            audience: Vec::new(),
            owner: String::new(),
            owners: Vec::new(),
            reviewed_at: String::new(),
            review_interval_days: default_review_interval_days(),
            implementation_refs: Vec::new(),
            version: String::new(),
            version_status: String::new(),
            related: Vec::new(),
            supersedes: Vec::new(),
            relations: Vec::new(),
            order: 0,
            pinned: false,
        }
    }
}

pub(crate) fn normalize_governance_facets(
    values: BTreeMap<String, DocumentGovernanceFacets>,
) -> Result<BTreeMap<String, DocumentGovernanceFacets>> {
    values
        .into_iter()
        .map(|(path, facets)| Ok((normalize_document_path(&path)?, sanitize_facets(facets)?)))
        .collect()
}

pub(crate) fn normalize_secondary_assignments(
    values: BTreeMap<String, Vec<String>>,
    valid_keys: &HashSet<String>,
) -> Result<BTreeMap<String, Vec<String>>> {
    values
        .into_iter()
        .map(|(path, topics)| {
            normalize_document_path(&path).map(|path| {
                let mut topics = topics
                    .into_iter()
                    .map(|topic| topic.trim().to_string())
                    .filter(|topic| valid_keys.contains(topic) && topic.starts_with("custom:"))
                    .take(12)
                    .collect::<Vec<_>>();
                topics.sort();
                topics.dedup();
                (path, topics)
            })
        })
        .collect::<Result<BTreeMap<_, _>>>()
        .map(|values| {
            values
                .into_iter()
                .filter(|(_, topics)| !topics.is_empty())
                .collect()
        })
}

pub(crate) fn sanitize_knowledge_metadata(
    mut metadata: DocumentKnowledgeMetadata,
) -> Result<DocumentKnowledgeMetadata> {
    metadata.id = truncate(metadata.id.trim(), 120);
    metadata.doc_type = type_identifier(&metadata.doc_type, 64);
    metadata.audience = bounded(metadata.audience, 12, 80);
    metadata.owner = truncate(metadata.owner.trim(), 80);
    metadata.owners = bounded(metadata.owners, 12, 80);
    metadata.reviewed_at = valid_date(&metadata.reviewed_at);
    metadata.review_interval_days = metadata.review_interval_days.clamp(1, 3650);
    metadata.implementation_refs = bounded(metadata.implementation_refs, 32, 500);
    metadata.version = truncate(metadata.version.trim(), 40);
    metadata.version_status =
        enum_value(&metadata.version_status, VERSION_STATUS_VALUES, "版本状态")?;
    metadata.related = normalize_paths(metadata.related, 24)?;
    metadata.supersedes = normalize_paths(metadata.supersedes, 24)?;
    metadata.relations = normalize_relations(metadata.relations)?;
    for path in &metadata.related {
        add_relation(&mut metadata.relations, "related", path);
    }
    for path in &metadata.supersedes {
        add_relation(&mut metadata.relations, "supersedes", path);
    }
    metadata.order = metadata.order.clamp(0, 999_999);
    Ok(metadata)
}

pub(crate) fn effective_facets(
    document: &ProjectDocumentEntry,
    configured: Option<&DocumentGovernanceFacets>,
) -> DocumentGovernanceFacets {
    let base = inferred_facets(document);
    let Some(configured) = configured else {
        return base;
    };
    DocumentGovernanceFacets {
        retrieval: clamp_retrieval(&base, &configured.retrieval),
        lifecycle: clamp_lifecycle(&base.lifecycle, &configured.lifecycle),
        authority: clamp_authority(&base.authority, &configured.authority),
        document_type: if configured.document_type.trim().is_empty() {
            base.document_type
        } else {
            identifier(&configured.document_type, 64)
        },
    }
}

pub(crate) fn effective_facets_with_metadata(
    document: &ProjectDocumentEntry,
    configured: Option<&DocumentGovernanceFacets>,
    metadata: Option<&DocumentKnowledgeMetadata>,
) -> DocumentGovernanceFacets {
    let mut facets = if manifest_can_classify_generic_document(document, configured, metadata) {
        curated_generic_facets(
            document,
            configured.expect("checked configured facets"),
            metadata,
        )
    } else {
        effective_facets(document, configured)
    };
    match metadata.map(|value| value.version_status.as_str()) {
        Some("draft") => {
            facets.lifecycle = "draft".to_string();
            if facets.retrieval == "required" {
                facets.retrieval = "on_demand".to_string();
            }
            if matches!(facets.authority.as_str(), "binding" | "authoritative") {
                facets.authority = "proposal".to_string();
            }
        }
        Some("deprecated" | "superseded" | "archived") => {
            facets.lifecycle = metadata
                .map(|value| value.version_status.clone())
                .unwrap_or_default();
            facets.retrieval = "excluded".to_string();
            facets.authority = "non_authoritative".to_string();
        }
        _ => {}
    }
    facets
}

/// A generic `docs/` path is not automatically authoritative, but it also is
/// not a permanent zero-authority ceiling. A versioned project manifest may
/// classify it when three independent curation signals agree: explicit
/// governance facets, an owner, and a review date. Negative path classes such
/// as drafts, reports, discussions, and archives are never eligible.
fn manifest_can_classify_generic_document(
    document: &ProjectDocumentEntry,
    configured: Option<&DocumentGovernanceFacets>,
    metadata: Option<&DocumentKnowledgeMetadata>,
) -> bool {
    let Some(configured) = configured else {
        return false;
    };
    let Some(metadata) = metadata else {
        return false;
    };
    let path = document.path.replace('\\', "/").to_ascii_lowercase();
    let generic_path_class = path.starts_with("docs/")
        && matches!(
            document.metadata.authority.as_str(),
            "unknown" | "informative"
        )
        && matches!(document.metadata.role.as_str(), "note" | "guide")
        && matches!(
            document.metadata.lifecycle.as_str(),
            "unclassified" | "active"
        );
    let curated = (!metadata.owner.trim().is_empty() || !metadata.owners.is_empty())
        && !metadata.reviewed_at.trim().is_empty()
        && !metadata.doc_type.trim().is_empty()
        && matches!(configured.lifecycle.as_str(), "active" | "accepted")
        && matches!(
            configured.authority.as_str(),
            "binding" | "authoritative" | "guidance" | "evidence" | "proposal"
        );
    generic_path_class && curated
}

fn curated_generic_facets(
    document: &ProjectDocumentEntry,
    configured: &DocumentGovernanceFacets,
    metadata: Option<&DocumentKnowledgeMetadata>,
) -> DocumentGovernanceFacets {
    let base = inferred_facets(document);
    let authority = match configured.authority.as_str() {
        // Repository/domain rules are the only documents allowed to be
        // binding. A curated ordinary document is capped at authoritative.
        "binding" => "authoritative",
        value => value,
    };
    DocumentGovernanceFacets {
        // Ordinary project documents are always task-routed. They cannot
        // become mandatory rules through the manifest.
        retrieval: match configured.retrieval.as_str() {
            "excluded" => "excluded",
            _ => "on_demand",
        }
        .to_string(),
        lifecycle: configured.lifecycle.clone(),
        authority: authority.to_string(),
        document_type: if configured.document_type.trim().is_empty() {
            metadata
                .map(|value| value.doc_type.clone())
                .filter(|value| !value.is_empty())
                .unwrap_or(base.document_type)
        } else {
            configured.document_type.clone()
        },
    }
}

pub(crate) fn quick_view(facets: &DocumentGovernanceFacets) -> &'static str {
    match facets.retrieval.as_str() {
        "required" => return "required",
        "on_demand"
            if matches!(
                facets.document_type.as_str(),
                "instruction" | "project_guide" | "provider_adapter" | "guide"
            ) =>
        {
            return "on-demand"
        }
        _ => {}
    }
    match facets.document_type.as_str() {
        "agent_definition" | "prompt_template" | "skill" | "project_template" => "customizations",
        "decision" => "decisions",
        "status" | "report" => "evidence",
        "archive" => "archive",
        _ if facets.lifecycle == "archived" => "archive",
        "discussion" | "note" => "drafts",
        _ if matches!(facets.lifecycle.as_str(), "draft" | "unclassified") => "drafts",
        _ if facets.authority == "unknown" => "unclassified",
        _ if facets.lifecycle == "active" || facets.lifecycle == "accepted" => "current",
        _ => "unclassified",
    }
}

fn sanitize_facets(mut facets: DocumentGovernanceFacets) -> Result<DocumentGovernanceFacets> {
    facets.retrieval = enum_value(&facets.retrieval, RETRIEVAL_VALUES, "检索策略")?;
    facets.lifecycle = enum_value(&facets.lifecycle, LIFECYCLE_VALUES, "生命周期")?;
    facets.authority = enum_value(&facets.authority, AUTHORITY_VALUES, "权威性")?;
    facets.document_type = identifier(&facets.document_type, 64);
    Ok(facets)
}

fn inferred_facets(document: &ProjectDocumentEntry) -> DocumentGovernanceFacets {
    let role = document.metadata.role.as_str();
    let lifecycle = document.metadata.lifecycle.trim();
    let excluded = matches!(
        lifecycle,
        "draft" | "deprecated" | "superseded" | "archived" | "unclassified"
    ) || matches!(
        role,
        "archive" | "discussion" | "note" | "status" | "report" | "project_template"
    );
    DocumentGovernanceFacets {
        retrieval: if matches!(role, "policy" | "router") {
            "required"
        } else if excluded {
            "excluded"
        } else {
            "on_demand"
        }
        .to_string(),
        lifecycle: if lifecycle.is_empty() {
            "unclassified"
        } else {
            lifecycle
        }
        .to_string(),
        authority: authority_level(&document.metadata.authority).to_string(),
        document_type: identifier(role, 64),
    }
}

fn clamp_retrieval(base: &DocumentGovernanceFacets, requested: &str) -> String {
    if requested.is_empty() {
        return base.retrieval.clone();
    }
    if base.retrieval == "excluded" || requested == "required" && base.retrieval != "required" {
        base.retrieval.clone()
    } else {
        requested.to_string()
    }
}

fn clamp_lifecycle(base: &str, requested: &str) -> String {
    if requested.is_empty() {
        return base.to_string();
    }
    if matches!(
        base,
        "draft" | "deprecated" | "superseded" | "archived" | "unclassified"
    ) && matches!(requested, "active" | "accepted")
    {
        base.to_string()
    } else {
        requested.to_string()
    }
}

fn clamp_authority(base: &str, requested: &str) -> String {
    if requested.is_empty() {
        return base.to_string();
    }
    let rank = |value: &str| match value {
        "binding" => 7,
        "authoritative" => 6,
        "guidance" => 5,
        "evidence" => 4,
        "proposal" => 3,
        "non_authoritative" => 2,
        "unknown" => 1,
        _ => 0,
    };
    if rank(requested) > rank(base) {
        base.to_string()
    } else {
        requested.to_string()
    }
}

fn authority_level(value: &str) -> &'static str {
    match value {
        "repository_policy" | "repository_routing" | "domain_policy" => "binding",
        "normative" | "approved" | "operational" | "decision_record" => "authoritative",
        "evidence" => "evidence",
        "proposal" => "proposal",
        "none" => "none",
        "historical" | "customization" => "non_authoritative",
        "provider_routing" | "project_guidance" | "informative" => "guidance",
        _ => "unknown",
    }
}

fn normalize_relations(values: Vec<DocumentRelation>) -> Result<Vec<DocumentRelation>> {
    let mut output = Vec::new();
    for relation in values.into_iter().take(48) {
        let kind = identifier(&relation.relation, 40);
        if !RELATION_VALUES.contains(&kind.as_str()) {
            bail!("未知文档关系：{kind}")
        }
        let target = normalize_document_path(&relation.target)?;
        add_relation(&mut output, &kind, &target);
    }
    Ok(output)
}

fn add_relation(output: &mut Vec<DocumentRelation>, relation: &str, target: &str) {
    if !output
        .iter()
        .any(|item| item.relation == relation && item.target.eq_ignore_ascii_case(target))
    {
        output.push(DocumentRelation {
            relation: relation.to_string(),
            target: target.to_string(),
        });
    }
}

fn normalize_paths(values: Vec<String>, limit: usize) -> Result<Vec<String>> {
    let mut values = values
        .into_iter()
        .take(limit)
        .map(|value| normalize_document_path(&value))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn enum_value(value: &str, allowed: &[&str], label: &str) -> Result<String> {
    let value = identifier(value, 64);
    if value.is_empty() || allowed.contains(&value.as_str()) {
        Ok(value)
    } else {
        bail!("未知{label}：{value}")
    }
}

fn identifier(value: &str, limit: usize) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('-', "_")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .take(limit)
        .collect()
}

fn type_identifier(value: &str, limit: usize) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(*ch, '_' | '-'))
        .take(limit)
        .collect()
}

fn bounded(values: Vec<String>, count: usize, chars: usize) -> Vec<String> {
    values
        .into_iter()
        .take(count)
        .map(|value| truncate(value.trim(), chars))
        .filter(|value| !value.is_empty())
        .collect()
}

fn valid_date(value: &str) -> String {
    let value = value.trim();
    (value.len() == 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-'))
    .then(|| value.to_string())
    .unwrap_or_default()
}

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}
fn default_review_interval_days() -> u16 {
    180
}

#[cfg(test)]
mod tests {
    use super::*;
    use homecli_proto::ProjectDocumentMetadata;

    #[test]
    fn path_ceiling_prevents_virtual_promotion() {
        let document = ProjectDocumentEntry {
            path: "docs/archive/old.md".into(),
            title: "Old".into(),
            content: String::new(),
            truncated: false,
            byte_len: 0,
            source: "workspace".into(),
            metadata: ProjectDocumentMetadata {
                role: "archive".into(),
                lifecycle: "archived".into(),
                authority: "historical".into(),
                ..Default::default()
            },
        };
        let requested = DocumentGovernanceFacets {
            retrieval: "required".into(),
            lifecycle: "active".into(),
            authority: "binding".into(),
            document_type: "spec".into(),
        };
        let effective = effective_facets(&document, Some(&requested));
        assert_eq!(effective.retrieval, "excluded");
        assert_eq!(effective.lifecycle, "archived");
        assert_eq!(effective.authority, "non_authoritative");
    }

    #[test]
    fn manifest_version_status_can_only_reduce_authority() {
        let document = ProjectDocumentEntry {
            path: "docs/current/specs/api.md".into(),
            title: "API".into(),
            content: String::new(),
            truncated: false,
            byte_len: 0,
            source: "workspace".into(),
            metadata: ProjectDocumentMetadata {
                role: "spec".into(),
                lifecycle: "active".into(),
                authority: "normative".into(),
                default_retrieval: true,
                ..Default::default()
            },
        };
        let metadata = DocumentKnowledgeMetadata {
            version_status: "superseded".into(),
            ..Default::default()
        };
        let effective = effective_facets_with_metadata(&document, None, Some(&metadata));
        assert_eq!(effective.lifecycle, "superseded");
        assert_eq!(effective.retrieval, "excluded");
        assert_eq!(effective.authority, "non_authoritative");
    }

    #[test]
    fn reviewed_manifest_can_classify_generic_docs_without_promoting_negative_paths() {
        let generic = ProjectDocumentEntry {
            path: "docs/system-guide.md".into(),
            title: "Guide".into(),
            content: String::new(),
            truncated: false,
            byte_len: 0,
            source: "workspace".into(),
            metadata: ProjectDocumentMetadata {
                role: "note".into(),
                lifecycle: "unclassified".into(),
                authority: "unknown".into(),
                ambiguous: true,
                ..Default::default()
            },
        };
        let requested = DocumentGovernanceFacets {
            retrieval: "required".into(),
            lifecycle: "active".into(),
            authority: "binding".into(),
            document_type: "architecture".into(),
        };
        let reviewed = DocumentKnowledgeMetadata {
            owner: "architecture-team".into(),
            reviewed_at: "2026-07-20".into(),
            doc_type: "architecture".into(),
            ..Default::default()
        };
        let effective = effective_facets_with_metadata(&generic, Some(&requested), Some(&reviewed));
        assert_eq!(effective.retrieval, "on_demand");
        assert_eq!(effective.lifecycle, "active");
        assert_eq!(effective.authority, "authoritative");

        let archived = ProjectDocumentEntry {
            path: "docs/archive/system-guide.md".into(),
            metadata: ProjectDocumentMetadata {
                role: "archive".into(),
                lifecycle: "archived".into(),
                authority: "historical".into(),
                ..Default::default()
            },
            ..generic
        };
        let effective =
            effective_facets_with_metadata(&archived, Some(&requested), Some(&reviewed));
        assert_eq!(effective.retrieval, "excluded");
        assert_eq!(effective.lifecycle, "archived");
        assert_eq!(effective.authority, "non_authoritative");
    }

    #[test]
    fn imported_conversation_keeps_explicit_zero_authority() {
        let document = ProjectDocumentEntry {
            path: "docs/inbox/conversations/chat.md".into(),
            title: "Chat".into(),
            content: String::new(),
            truncated: false,
            byte_len: 0,
            source: "workspace".into(),
            metadata: ProjectDocumentMetadata {
                role: "discussion".into(),
                lifecycle: "source_material".into(),
                authority: "none".into(),
                default_retrieval: false,
                ambiguous: false,
                ..Default::default()
            },
        };

        let effective = effective_facets(&document, None);
        assert_eq!(effective.retrieval, "excluded");
        assert_eq!(effective.lifecycle, "source_material");
        assert_eq!(effective.authority, "none");
        assert_eq!(quick_view(&effective), "drafts");
    }
}
