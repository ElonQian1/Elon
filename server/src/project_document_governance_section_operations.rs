//! Reversible AI operations over the virtual OneNote-style section tree.

use anyhow::{bail, Result};
use std::collections::{BTreeMap, HashSet};

use crate::project_document_governance::{
    CustomDocumentSection, DocumentOrganizationAuditEntry, DocumentSectionManifest,
    SuggestedSectionOperation,
};

pub(crate) fn apply_section_operations(
    manifest: &mut DocumentSectionManifest,
    sections: &mut BTreeMap<String, CustomDocumentSection>,
    operations: &[SuggestedSectionOperation],
) -> Result<usize> {
    let mut applied = 0;
    for operation in operations {
        match operation.kind.as_str() {
            "create" => create_section(sections, operation)?,
            "rename" => rename_section(sections, operation)?,
            "move" => move_section(sections, operation)?,
            "merge" => merge_section(manifest, sections, operation)?,
            "delete" => delete_section_tree(manifest, sections, operation)?,
            _ => bail!("未知 AI 分区操作：{}", operation.kind),
        }
        append_audit(manifest, operation);
        applied += 1;
    }
    Ok(applied)
}

fn create_section(
    sections: &mut BTreeMap<String, CustomDocumentSection>,
    operation: &SuggestedSectionOperation,
) -> Result<()> {
    if sections.contains_key(&operation.section_id) {
        return Ok(());
    }
    if operation.label.is_empty() {
        bail!("新增分区 {} 缺少 label", operation.section_id);
    }
    let order = sections
        .values()
        .filter(|section| section.parent_id == operation.parent_id)
        .map(|section| section.order)
        .max()
        .unwrap_or_default()
        .saturating_add(10);
    sections.insert(
        operation.section_id.clone(),
        CustomDocumentSection {
            id: operation.section_id.clone(),
            label: operation.label.clone(),
            detail: operation.reason.clone(),
            color: "#7f8fb3".to_string(),
            parent_id: operation.parent_id.clone(),
            order,
            ..CustomDocumentSection::default()
        },
    );
    Ok(())
}

fn rename_section(
    sections: &mut BTreeMap<String, CustomDocumentSection>,
    operation: &SuggestedSectionOperation,
) -> Result<()> {
    let section = required_section_mut(sections, &operation.section_id)?;
    if operation.label.is_empty() {
        bail!("重命名分区 {} 缺少 label", operation.section_id);
    }
    section.label = operation.label.clone();
    Ok(())
}

fn move_section(
    sections: &mut BTreeMap<String, CustomDocumentSection>,
    operation: &SuggestedSectionOperation,
) -> Result<()> {
    let parent_id = if operation.parent_id.is_empty() {
        operation.target_section_id.clone()
    } else {
        operation.parent_id.clone()
    };
    if !parent_id.is_empty() && !sections.contains_key(&parent_id) {
        bail!("移动目标父分区不存在：{parent_id}");
    }
    required_section_mut(sections, &operation.section_id)?.parent_id = parent_id;
    Ok(())
}

fn merge_section(
    manifest: &mut DocumentSectionManifest,
    sections: &mut BTreeMap<String, CustomDocumentSection>,
    operation: &SuggestedSectionOperation,
) -> Result<()> {
    let source_id = operation.section_id.as_str();
    let target_id = operation.target_section_id.as_str();
    if target_id.is_empty() || source_id == target_id {
        bail!("合并分区必须指定不同的 target_section_id");
    }
    let source = sections
        .get(source_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("AI 分区操作引用了不存在的分区：{source_id}"))?;
    if !sections.contains_key(target_id) {
        bail!("合并目标分区不存在：{target_id}");
    }
    let source_key = custom_key(source_id);
    let target_key = custom_key(target_id);
    for value in manifest.assignments.values_mut() {
        if value == &source_key {
            *value = target_key.clone();
        }
    }
    for values in manifest.secondary_assignments.values_mut() {
        for value in values.iter_mut() {
            if value == &source_key {
                *value = target_key.clone();
            }
        }
        values.sort();
        values.dedup();
    }
    for section in sections.values_mut() {
        if section.parent_id == source_id {
            section.parent_id = target_id.to_string();
        }
    }
    if let Some(target) = sections.get_mut(target_id) {
        if target.entrypoint.is_empty() {
            target.entrypoint = source.entrypoint;
        }
    }
    sections.remove(source_id);
    Ok(())
}

fn delete_section_tree(
    manifest: &mut DocumentSectionManifest,
    sections: &mut BTreeMap<String, CustomDocumentSection>,
    operation: &SuggestedSectionOperation,
) -> Result<()> {
    if !sections.contains_key(&operation.section_id) {
        bail!("AI 分区操作引用了不存在的分区：{}", operation.section_id);
    }
    let mut removed = HashSet::from([operation.section_id.clone()]);
    loop {
        let before = removed.len();
        for section in sections.values() {
            if removed.contains(&section.parent_id) {
                removed.insert(section.id.clone());
            }
        }
        if before == removed.len() {
            break;
        }
    }
    sections.retain(|id, _| !removed.contains(id));
    manifest
        .assignments
        .retain(|_, value| !removed.contains(custom_id(value)));
    for values in manifest.secondary_assignments.values_mut() {
        values.retain(|value| !removed.contains(custom_id(value)));
    }
    manifest
        .secondary_assignments
        .retain(|_, values| !values.is_empty());
    Ok(())
}

fn required_section_mut<'a>(
    sections: &'a mut BTreeMap<String, CustomDocumentSection>,
    id: &str,
) -> Result<&'a mut CustomDocumentSection> {
    sections
        .get_mut(id)
        .ok_or_else(|| anyhow::anyhow!("AI 分区操作引用了不存在的分区：{id}"))
}

fn append_audit(manifest: &mut DocumentSectionManifest, operation: &SuggestedSectionOperation) {
    manifest.audit_log.push(DocumentOrganizationAuditEntry {
        id: operation.id.clone(),
        action: format!("ai.section.{}", operation.kind),
        target: operation.section_id.clone(),
        summary: format!("{}；影响：{}", operation.reason, operation.impact),
        at: chrono::Utc::now().to_rfc3339(),
    });
    let keep_from = manifest.audit_log.len().saturating_sub(100);
    manifest.audit_log.drain(..keep_from);
}

fn custom_key(id: &str) -> String {
    format!("custom:{id}")
}

fn custom_id(value: &str) -> &str {
    value.strip_prefix("custom:").unwrap_or(value)
}
