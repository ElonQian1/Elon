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

pub(crate) const SECTION_CONFIG_PATH: &str = ".elon/document-sections.json";
pub(crate) const SUGGESTIONS_CONFIG_PATH: &str = ".elon/document-organization-suggestions.json";
pub(crate) const MAX_PROPOSED_SECTIONS: usize = 8;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CustomDocumentSection {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default = "default_section_color")]
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentSectionManifest {
    #[serde(default = "schema_version")]
    pub version: u8,
    #[serde(default)]
    pub sections: Vec<CustomDocumentSection>,
    #[serde(default)]
    pub assignments: BTreeMap<String, String>,
}

impl Default for DocumentSectionManifest {
    fn default() -> Self {
        Self {
            version: schema_version(),
            sections: Vec::new(),
            assignments: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SuggestedAssignment {
    pub path: String,
    pub section_id: String,
    #[serde(default)]
    pub reason: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocumentOrganizationSuggestions {
    #[serde(default = "schema_version")]
    pub version: u8,
    #[serde(default)]
    pub status: OrganizationStatus,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub proposed_sections: Vec<CustomDocumentSection>,
    #[serde(default)]
    pub assignments: Vec<SuggestedAssignment>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub move_suggestions: Vec<String>,
    #[serde(default)]
    pub file_operations: Vec<SuggestedFileOperation>,
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
    if manifest.sections.len() > 64 || manifest.assignments.len() > 5_000 {
        bail!("项目文档分区配置超过安全上限");
    }
    manifest.sections = unique_sections(manifest.sections, 64)?;
    let valid_keys = valid_section_keys(&manifest.sections);
    let mut assignments = BTreeMap::new();
    for (path, section) in manifest.assignments {
        let path = normalize_document_path(&path)?;
        let section = normalize_section_key(&section, &manifest.sections);
        if !valid_keys.contains(&section) {
            bail!("分区配置引用了未知分区：{section}");
        }
        assignments.insert(path, section);
    }
    manifest.assignments = assignments;
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
        || suggestions.file_operations.len() > MAX_SUGGESTED_FILE_OPERATIONS
    {
        bail!("AI 文档整理建议超过安全上限");
    }
    suggestions.summary = truncate_chars(suggestions.summary.trim(), 4_000);
    suggestions.proposed_sections =
        unique_sections(suggestions.proposed_sections, MAX_PROPOSED_SECTIONS)?;
    suggestions.assignments = suggestions
        .assignments
        .into_iter()
        .map(|item| {
            Ok(SuggestedAssignment {
                path: normalize_document_path(&item.path)?,
                section_id: truncate_chars(item.section_id.trim(), 64),
                reason: truncate_chars(item.reason.trim(), 500),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    suggestions.conflicts = bounded_strings(suggestions.conflicts, 100, 1_000);
    suggestions.move_suggestions = bounded_strings(suggestions.move_suggestions, 100, 1_000);
    suggestions.file_operations = normalize_file_operations(suggestions.file_operations)?;
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
    manifest.sections = sections.into_values().collect();
    let valid_keys = valid_section_keys(&manifest.sections);
    let mut applied = 0usize;
    let mut skipped = 0usize;
    for assignment in &suggestions.assignments {
        let section = normalize_section_key(&assignment.section_id, &manifest.sections);
        if known_paths.contains(&assignment.path.to_ascii_lowercase())
            && valid_keys.contains(&section)
        {
            manifest
                .assignments
                .insert(assignment.path.clone(), section);
            applied += 1;
        } else {
            skipped += 1;
        }
    }
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
    let metadata = &document.metadata;
    if metadata.lifecycle == "archived" || metadata.role == "archive" {
        "archive"
    } else if matches!(metadata.role.as_str(), "policy" | "router") {
        "required"
    } else if matches!(
        metadata.role.as_str(),
        "agent_definition" | "prompt_template" | "skill"
    ) {
        "customizations"
    } else if matches!(
        metadata.role.as_str(),
        "instruction" | "project_guide" | "provider_adapter" | "guide"
    ) {
        "on-demand"
    } else if matches!(
        metadata.role.as_str(),
        "spec" | "architecture" | "requirement" | "runbook"
    ) && metadata.lifecycle == "active"
    {
        "current"
    } else if metadata.role == "decision" {
        "decisions"
    } else if matches!(metadata.role.as_str(), "status" | "report") {
        "evidence"
    } else if metadata.ambiguous || metadata.lifecycle == "unclassified" {
        "unclassified"
    } else if metadata.lifecycle == "draft"
        || matches!(metadata.role.as_str(), "discussion" | "note")
    {
        "drafts"
    } else {
        "unclassified"
    }
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
    if section.id.is_empty() || section.label.is_empty() {
        bail!("自定义分区必须包含有效 id 和 label");
    }
    if !is_hex_color(&section.color) {
        section.color = default_section_color();
    }
    Ok(section)
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
