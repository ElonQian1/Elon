//! Vendor-neutral project document organization contract and pure domain rules.

use anyhow::{bail, Result};
use homecli_proto::ProjectDocumentEntry;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::project_document_file_operation_model::{
    normalize_document_path, normalize_file_operations, validate_file_operations,
};
pub(crate) use crate::project_document_file_operation_model::{
    SuggestedFileOperation, SuggestedFileOperationKind, SuggestedFileOperationStatus,
};
use crate::project_document_governance_facets::{
    effective_facets, normalize_governance_facets, normalize_secondary_assignments, quick_view,
    sanitize_knowledge_metadata,
};
pub(crate) use crate::project_document_governance_facets::{
    DocumentGovernanceFacets, DocumentKnowledgeMetadata,
};
use crate::project_document_knowledge_graph_model::{
    merge_graph_config, normalize_graph_config, validate_graph_document_paths,
    ProjectKnowledgeGraphConfig,
};

pub(crate) const SECTION_CONFIG_PATH: &str = ".elon/document-sections.json";
pub(crate) const SUGGESTIONS_CONFIG_PATH: &str = ".elon/document-organization-suggestions.json";
pub(crate) const MAX_PROPOSED_SECTIONS: usize = 16;
pub(crate) const MAX_SUGGESTED_ASSIGNMENTS: usize = 500;
pub(crate) const MAX_SUGGESTED_FILE_OPERATIONS: usize = 100;

const SYSTEM_SECTION_KEYS: &[&str] = &[
    "required",
    "on-demand",
    "current",
    "customizations",
    "drafts",
    "evidence",
    "decisions",
    "archive",
    "unclassified",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CustomDocumentSection {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default = "default_section_color")]
    pub color: String,
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub entrypoint: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentKnowledgeHome {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub entrypoint: String,
    #[serde(default)]
    pub start_here: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentOrganizationAuditEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentSectionManifest {
    #[serde(default = "schema_version")]
    pub version: u8,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub home: DocumentKnowledgeHome,
    #[serde(default)]
    pub sections: Vec<CustomDocumentSection>,
    #[serde(default)]
    pub assignments: BTreeMap<String, String>,
    #[serde(default)]
    pub secondary_assignments: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub governance_overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub governance_facets: BTreeMap<String, DocumentGovernanceFacets>,
    #[serde(default)]
    pub document_metadata: BTreeMap<String, DocumentKnowledgeMetadata>,
    #[serde(default)]
    pub audit_log: Vec<DocumentOrganizationAuditEntry>,
    #[serde(default)]
    pub knowledge_graph: ProjectKnowledgeGraphConfig,
}

impl Default for DocumentSectionManifest {
    fn default() -> Self {
        Self {
            version: schema_version(),
            profile: "auto".to_string(),
            home: DocumentKnowledgeHome::default(),
            sections: Vec::new(),
            assignments: BTreeMap::new(),
            secondary_assignments: BTreeMap::new(),
            governance_overrides: BTreeMap::new(),
            governance_facets: BTreeMap::new(),
            document_metadata: BTreeMap::new(),
            audit_log: Vec::new(),
            knowledge_graph: ProjectKnowledgeGraphConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SuggestedAssignment {
    pub path: String,
    pub section_id: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub secondary: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OrganizationStatus {
    Requested,
    Ready,
    Applied,
}

impl Default for OrganizationStatus {
    fn default() -> Self {
        Self::Requested
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentOrganizationSuggestions {
    #[serde(default = "schema_version")]
    pub version: u8,
    #[serde(default)]
    pub status: OrganizationStatus,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub proposed_profile: String,
    #[serde(default)]
    pub proposed_home: Option<DocumentKnowledgeHome>,
    #[serde(default)]
    pub proposed_sections: Vec<CustomDocumentSection>,
    #[serde(default)]
    pub assignments: Vec<SuggestedAssignment>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub move_suggestions: Vec<String>,
    #[serde(default)]
    pub architecture_findings: Vec<String>,
    #[serde(default)]
    pub missing_document_types: Vec<String>,
    #[serde(default)]
    pub document_metadata: BTreeMap<String, DocumentKnowledgeMetadata>,
    #[serde(default)]
    pub governance_facets: BTreeMap<String, DocumentGovernanceFacets>,
    #[serde(default)]
    pub file_operations: Vec<SuggestedFileOperation>,
    #[serde(default)]
    pub proposed_knowledge_graph: ProjectKnowledgeGraphConfig,
    #[serde(default)]
    pub documents_read: u64,
    #[serde(default)]
    pub estimated_tokens_used: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ApplySuggestionsResult {
    pub manifest: DocumentSectionManifest,
    pub suggestions: DocumentOrganizationSuggestions,
    pub applied_assignments: usize,
    pub skipped_assignments: usize,
    pub already_applied: bool,
}

pub(crate) fn parse_manifest(content: Option<&str>) -> Result<DocumentSectionManifest> {
    let Some(content) = content.filter(|value| !value.trim().is_empty()) else {
        return Ok(DocumentSectionManifest::default());
    };
    normalize_manifest(serde_json::from_str(content)?)
}

pub(crate) fn parse_suggestions(
    content: Option<&str>,
) -> Result<Option<DocumentOrganizationSuggestions>> {
    let Some(content) = content.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    Ok(Some(normalize_suggestions(serde_json::from_str(content)?)?))
}

pub(crate) fn normalize_manifest(
    mut manifest: DocumentSectionManifest,
) -> Result<DocumentSectionManifest> {
    if manifest.version != schema_version() {
        bail!("document-sections.json 仅支持 version=1");
    }
    if manifest.sections.len() > 64
        || manifest.assignments.len() > 5_000
        || manifest.secondary_assignments.len() > 5_000
        || manifest.governance_overrides.len() > 5_000
        || manifest.governance_facets.len() > 5_000
        || manifest.document_metadata.len() > 5_000
        || manifest.audit_log.len() > 100
    {
        bail!("项目文档分区配置超过安全上限");
    }
    manifest.profile = sanitize_profile(&manifest.profile);
    manifest.home = sanitize_home(manifest.home)?;
    manifest.sections = unique_sections(manifest.sections, 64)?;
    validate_section_tree(&manifest.sections)?;
    let valid_keys = valid_section_keys(&manifest.sections);
    let mut assignments = BTreeMap::new();
    let mut governance_overrides = manifest
        .governance_overrides
        .into_iter()
        .map(|(path, section)| {
            let path = normalize_document_path(&path)?;
            let section = section.trim().to_string();
            if !SYSTEM_SECTION_KEYS.contains(&section.as_str()) {
                bail!("治理覆盖引用了未知治理分区：{section}");
            }
            Ok((path, section))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    for (path, section) in manifest.assignments {
        let path = normalize_document_path(&path)?;
        let section = normalize_section_key(&section, &manifest.sections);
        if !valid_keys.contains(&section) {
            bail!("分区配置引用了未知分区：{section}");
        }
        if SYSTEM_SECTION_KEYS.contains(&section.as_str()) {
            // Backward compatibility: manifests written before knowledge/governance
            // separation stored both facets in `assignments`.
            governance_overrides.insert(path, section);
        } else {
            assignments.insert(path, section);
        }
    }
    manifest.assignments = assignments;
    manifest.secondary_assignments =
        normalize_secondary_assignments(manifest.secondary_assignments, &valid_keys)?;
    for (path, topics) in &mut manifest.secondary_assignments {
        if let Some(primary) = manifest.assignments.get(path) {
            topics.retain(|topic| topic != primary);
        }
    }
    manifest
        .secondary_assignments
        .retain(|_, topics| !topics.is_empty());
    manifest.governance_overrides = governance_overrides;
    manifest.governance_facets = normalize_governance_facets(manifest.governance_facets)?;
    manifest.document_metadata = manifest
        .document_metadata
        .into_iter()
        .map(|(path, metadata)| {
            Ok((
                normalize_document_path(&path)?,
                sanitize_knowledge_metadata(metadata)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    manifest.audit_log = manifest
        .audit_log
        .into_iter()
        .rev()
        .take(100)
        .map(sanitize_audit_entry)
        .filter(|entry| !entry.id.is_empty() && !entry.action.is_empty())
        .collect::<Vec<_>>();
    manifest.audit_log.reverse();
    manifest.knowledge_graph = normalize_graph_config(manifest.knowledge_graph)?;
    Ok(manifest)
}

pub(crate) fn normalize_suggestions(
    mut suggestions: DocumentOrganizationSuggestions,
) -> Result<DocumentOrganizationSuggestions> {
    if suggestions.version != schema_version() {
        bail!("document-organization-suggestions.json 仅支持 version=1");
    }
    if suggestions.proposed_sections.len() > MAX_PROPOSED_SECTIONS
        || suggestions.assignments.len() > MAX_SUGGESTED_ASSIGNMENTS
        || suggestions.conflicts.len() > 100
        || suggestions.move_suggestions.len() > 100
        || suggestions.architecture_findings.len() > 100
        || suggestions.missing_document_types.len() > 100
        || suggestions.document_metadata.len() > MAX_SUGGESTED_ASSIGNMENTS
        || suggestions.governance_facets.len() > MAX_SUGGESTED_ASSIGNMENTS
        || suggestions.file_operations.len() > MAX_SUGGESTED_FILE_OPERATIONS
    {
        bail!("AI 文档整理建议超过安全上限");
    }
    suggestions.summary = truncate_chars(suggestions.summary.trim(), 4_000);
    suggestions.proposed_profile = sanitize_profile(&suggestions.proposed_profile);
    suggestions.proposed_home = suggestions.proposed_home.map(sanitize_home).transpose()?;
    suggestions.proposed_sections =
        unique_sections(suggestions.proposed_sections, MAX_PROPOSED_SECTIONS)?;
    validate_section_tree(&suggestions.proposed_sections)?;
    suggestions.assignments = suggestions
        .assignments
        .into_iter()
        .map(|item| {
            Ok(SuggestedAssignment {
                path: normalize_document_path(&item.path)?,
                section_id: truncate_chars(item.section_id.trim(), 64),
                reason: truncate_chars(item.reason.trim(), 500),
                secondary: item.secondary,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    suggestions.conflicts = bounded_strings(suggestions.conflicts, 100, 1_000);
    suggestions.move_suggestions = bounded_strings(suggestions.move_suggestions, 100, 1_000);
    suggestions.architecture_findings =
        bounded_strings(suggestions.architecture_findings, 100, 1_000);
    suggestions.missing_document_types =
        bounded_strings(suggestions.missing_document_types, 100, 120);
    suggestions.document_metadata = suggestions
        .document_metadata
        .into_iter()
        .map(|(path, metadata)| {
            Ok((
                normalize_document_path(&path)?,
                sanitize_knowledge_metadata(metadata)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    suggestions.governance_facets = normalize_governance_facets(suggestions.governance_facets)?;
    suggestions.file_operations = normalize_file_operations(suggestions.file_operations)?;
    suggestions.proposed_knowledge_graph =
        normalize_graph_config(suggestions.proposed_knowledge_graph)?;
    Ok(suggestions)
}

pub(crate) fn validate_ready_suggestions(
    suggestions: DocumentOrganizationSuggestions,
    documents: &[ProjectDocumentEntry],
) -> Result<DocumentOrganizationSuggestions> {
    let suggestions = normalize_suggestions(suggestions)?;
    if suggestions.status != OrganizationStatus::Ready {
        bail!("保存 AI 建议时 status 必须为 ready");
    }
    let known_paths = document_paths(documents);
    let known_sections = valid_section_keys(&suggestions.proposed_sections);
    for assignment in &suggestions.assignments {
        if !known_paths.contains(&assignment.path.to_ascii_lowercase()) {
            bail!("AI 建议引用了目录中不存在的文档：{}", assignment.path);
        }
        let section = normalize_section_key(&assignment.section_id, &suggestions.proposed_sections);
        if !known_sections.contains(&section) {
            bail!("AI 建议引用了未知分区：{}", assignment.section_id);
        }
    }
    validate_suggested_paths(&suggestions, &known_paths)?;
    validate_graph_document_paths(&suggestions.proposed_knowledge_graph, &known_paths)?;
    validate_file_operations(&suggestions.file_operations, documents, &known_paths)?;
    Ok(suggestions)
}

pub(crate) fn apply_suggestions(
    manifest: DocumentSectionManifest,
    suggestions: DocumentOrganizationSuggestions,
    documents: &[ProjectDocumentEntry],
) -> Result<ApplySuggestionsResult> {
    let mut manifest = normalize_manifest(manifest)?;
    let mut suggestions = normalize_suggestions(suggestions)?;
    if suggestions.status == OrganizationStatus::Applied {
        return Ok(ApplySuggestionsResult {
            manifest,
            suggestions,
            applied_assignments: 0,
            skipped_assignments: 0,
            already_applied: true,
        });
    }
    if suggestions.status != OrganizationStatus::Ready {
        bail!("只有 ready 状态的 AI 建议可以应用");
    }
    let known_paths = document_paths(documents);
    let mut sections = manifest
        .sections
        .into_iter()
        .map(|section| (section.id.clone(), section))
        .collect::<BTreeMap<_, _>>();
    for section in suggestions.proposed_sections.iter().cloned() {
        sections.insert(section.id.clone(), section);
    }
    if suggestions.proposed_profile != "auto" {
        manifest.profile = suggestions.proposed_profile.clone();
    }
    if let Some(home) = suggestions.proposed_home.clone() {
        manifest.home = home;
    }
    manifest
        .document_metadata
        .extend(suggestions.document_metadata.clone());
    manifest
        .governance_facets
        .extend(suggestions.governance_facets.clone());
    manifest.knowledge_graph = merge_graph_config(
        manifest.knowledge_graph,
        suggestions.proposed_knowledge_graph.clone(),
    )?;
    manifest.sections = sections.into_values().collect();
    let valid_keys = valid_section_keys(&manifest.sections);
    let mut applied = 0usize;
    let mut skipped = 0usize;
    for assignment in &suggestions.assignments {
        let section = normalize_section_key(&assignment.section_id, &manifest.sections);
        if known_paths.contains(&assignment.path.to_ascii_lowercase())
            && valid_keys.contains(&section)
        {
            if assignment.secondary
                && section.starts_with("custom:")
                && manifest.assignments.get(&assignment.path) != Some(&section)
            {
                let topics = manifest
                    .secondary_assignments
                    .entry(assignment.path.clone())
                    .or_default();
                if !topics.contains(&section) {
                    topics.push(section);
                }
            } else if SYSTEM_SECTION_KEYS.contains(&section.as_str()) {
                manifest
                    .governance_overrides
                    .insert(assignment.path.clone(), section);
            } else {
                manifest
                    .assignments
                    .insert(assignment.path.clone(), section.clone());
                if let Some(topics) = manifest.secondary_assignments.get_mut(&assignment.path) {
                    topics.retain(|topic| topic != &section);
                }
            }
            applied += 1;
        } else {
            skipped += 1;
        }
    }
    manifest
        .secondary_assignments
        .retain(|_, topics| !topics.is_empty());
    suggestions.status = OrganizationStatus::Applied;
    Ok(ApplySuggestionsResult {
        manifest,
        suggestions,
        applied_assignments: applied,
        skipped_assignments: skipped,
        already_applied: false,
    })
}

pub(crate) fn automatic_section(document: &ProjectDocumentEntry) -> &'static str {
    quick_view(&effective_facets(document, None))
}

pub(crate) fn effective_section(
    document: &ProjectDocumentEntry,
    manifest: &DocumentSectionManifest,
) -> String {
    manifest
        .assignments
        .get(&document.path.replace('\\', "/"))
        .cloned()
        .unwrap_or_else(|| automatic_section(document).to_string())
}

pub(crate) fn to_pretty_json<T: Serialize>(value: &T) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(value)?))
}

fn unique_sections(
    sections: Vec<CustomDocumentSection>,
    limit: usize,
) -> Result<Vec<CustomDocumentSection>> {
    let mut output = Vec::new();
    let mut indexes = HashMap::new();
    for section in sections.into_iter().take(limit) {
        let section = sanitize_section(section)?;
        if let Some(index) = indexes.get(&section.id).copied() {
            output[index] = section;
        } else {
            indexes.insert(section.id.clone(), output.len());
            output.push(section);
        }
    }
    Ok(output)
}

fn sanitize_section(mut section: CustomDocumentSection) -> Result<CustomDocumentSection> {
    section.id = sanitize_section_id(&section.id);
    section.label = truncate_chars(section.label.trim(), 40);
    section.detail = truncate_chars(section.detail.trim(), 120);
    section.color = section.color.trim().to_string();
    section.parent_id = sanitize_section_id(&section.parent_id);
    section.order = section.order.clamp(0, 9_999);
    section.icon = truncate_chars(section.icon.trim(), 32);
    section.entrypoint = if section.entrypoint.trim().is_empty() {
        String::new()
    } else {
        normalize_document_path(&section.entrypoint)?
    };
    if section.id.is_empty() || section.label.is_empty() {
        bail!("自定义分区必须包含有效 id 和 label");
    }
    if !is_hex_color(&section.color) {
        section.color = default_section_color();
    }
    Ok(section)
}

fn validate_section_tree(sections: &[CustomDocumentSection]) -> Result<()> {
    let parents = sections
        .iter()
        .map(|section| (section.id.as_str(), section.parent_id.as_str()))
        .collect::<HashMap<_, _>>();
    for section in sections {
        if !section.parent_id.is_empty() && !parents.contains_key(section.parent_id.as_str()) {
            bail!("知识分区引用了不存在的父分区：{}", section.parent_id);
        }
        let mut cursor = section.id.as_str();
        let mut visited = HashSet::new();
        let mut depth = 0usize;
        loop {
            if !visited.insert(cursor) {
                bail!("知识分区层级存在循环：{}", section.id);
            }
            let parent = parents.get(cursor).copied().unwrap_or_default();
            if parent.is_empty() {
                break;
            }
            depth += 1;
            if depth >= 4 {
                bail!("知识分区层级最多支持 4 层：{}", section.id);
            }
            cursor = parent;
        }
    }
    Ok(())
}

fn sanitize_profile(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "software-platform" | "software-api" | "product" | "research" | "operations"
        | "personal-knowledge" => value.trim().to_ascii_lowercase(),
        _ => "auto".to_string(),
    }
}

fn sanitize_home(mut home: DocumentKnowledgeHome) -> Result<DocumentKnowledgeHome> {
    home.title = truncate_chars(home.title.trim(), 80);
    home.summary = truncate_chars(home.summary.trim(), 1_000);
    home.entrypoint = if home.entrypoint.trim().is_empty() {
        String::new()
    } else {
        normalize_document_path(&home.entrypoint)?
    };
    home.start_here = home
        .start_here
        .into_iter()
        .take(12)
        .map(|path| normalize_document_path(&path))
        .collect::<Result<Vec<_>>>()?;
    home.start_here.sort();
    home.start_here.dedup();
    Ok(home)
}

fn sanitize_audit_entry(
    mut entry: DocumentOrganizationAuditEntry,
) -> DocumentOrganizationAuditEntry {
    entry.id = truncate_chars(entry.id.trim(), 80);
    entry.action = truncate_chars(entry.action.trim(), 64);
    entry.target = truncate_chars(entry.target.trim(), 500);
    entry.summary = truncate_chars(entry.summary.trim(), 500);
    entry.at = truncate_chars(entry.at.trim(), 40);
    entry
}

fn validate_suggested_paths(
    suggestions: &DocumentOrganizationSuggestions,
    known_paths: &HashSet<String>,
) -> Result<()> {
    let validate = |path: &str| {
        if known_paths.contains(&path.to_ascii_lowercase()) {
            Ok(())
        } else {
            bail!("知识架构建议引用了目录中不存在的文档：{path}")
        }
    };
    if let Some(home) = &suggestions.proposed_home {
        if !home.entrypoint.is_empty() {
            validate(&home.entrypoint)?;
        }
        for path in &home.start_here {
            validate(path)?;
        }
    }
    for (path, metadata) in &suggestions.document_metadata {
        validate(path)?;
        for related in metadata
            .related
            .iter()
            .chain(&metadata.supersedes)
            .chain(metadata.relations.iter().map(|relation| &relation.target))
        {
            validate(related)?;
        }
    }
    for path in suggestions.governance_facets.keys() {
        validate(path)?;
    }
    Ok(())
}

fn valid_section_keys(sections: &[CustomDocumentSection]) -> HashSet<String> {
    SYSTEM_SECTION_KEYS
        .iter()
        .map(|value| value.to_string())
        .chain(
            sections
                .iter()
                .map(|section| format!("custom:{}", section.id)),
        )
        .collect()
}

fn normalize_section_key(value: &str, sections: &[CustomDocumentSection]) -> String {
    let value = value.trim();
    if SYSTEM_SECTION_KEYS.contains(&value) {
        value.to_string()
    } else if let Some(id) = value.strip_prefix("custom:") {
        format!("custom:{}", sanitize_section_id(id))
    } else if sections.iter().any(|section| section.id == value) {
        format!("custom:{value}")
    } else {
        value.to_string()
    }
}

fn document_paths(documents: &[ProjectDocumentEntry]) -> HashSet<String> {
    documents
        .iter()
        .map(|document| document.path.replace('\\', "/").to_ascii_lowercase())
        .collect()
}

fn sanitize_section_id(value: &str) -> String {
    let mut output = String::new();
    for ch in value.trim().to_lowercase().chars() {
        let valid = ch.is_ascii_alphanumeric()
            || matches!(ch, '_' | '-')
            || ('\u{4e00}'..='\u{9fff}').contains(&ch);
        if valid {
            output.push(ch);
        } else if !output.is_empty() && !output.ends_with('-') {
            output.push('-');
        }
        if output.chars().count() >= 48 {
            break;
        }
    }
    output.trim_matches('-').to_string()
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

fn bounded_strings(values: Vec<String>, count: usize, chars: usize) -> Vec<String> {
    values
        .into_iter()
        .take(count)
        .map(|value| truncate_chars(value.trim(), chars))
        .filter(|value| !value.is_empty())
        .collect()
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn schema_version() -> u8 {
    1
}

fn default_section_color() -> String {
    "#7f8fb3".to_string()
}
