//! Bounded CI-style health report for shared project-navigation memory.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    project_document_governance::{parse_manifest, SECTION_CONFIG_PATH},
    project_document_native_context::{validate_evidence_current, ProjectContextMemory},
    project_document_native_context_git::{relocation_candidates_from_index, relocation_index},
};

const MAX_HEALTH_MEMORIES: usize = 64;

pub(crate) fn shared_memory_health(workspace: &Path) -> Result<Value> {
    let manifest_path = workspace.join(SECTION_CONFIG_PATH);
    let manifest = if manifest_path.is_file() {
        let content = fs::read_to_string(&manifest_path)
            .with_context(|| format!("读取共享项目记忆失败：{}", manifest_path.display()))?;
        parse_manifest(Some(&content))?
    } else {
        parse_manifest(None)?
    };
    let mut report = memory_health_report(workspace, &manifest.context_memories);
    report["receipt_automation"] = json!({
        "node_policy_enabled": crate::node_agent_project_memory_hook_config::enabled(""),
        "trust_mode": "codex_non_managed_hook_review",
        "trust_bypass_enabled": false,
        "runtime_execution_observation_available": false,
        "status_rule": "Configured does not mean trusted or executed; verify with Codex /hooks and a dedicated runtime check."
    });
    Ok(report)
}

pub(crate) fn memory_health_report(workspace: &Path, memories: &[ProjectContextMemory]) -> Value {
    let mut current_count = 0usize;
    let mut drifted_count = 0usize;
    let mut relocation_count = 0usize;
    let relocation_index = relocation_index(workspace);
    let items = memories
        .iter()
        .take(MAX_HEALTH_MEMORIES)
        .map(|memory| {
            let mut drifted_paths = Vec::new();
            let mut relocations = BTreeSet::new();
            for evidence in &memory.evidence {
                if validate_evidence_current(workspace, evidence).is_err() {
                    drifted_paths.push(evidence.path.clone());
                    relocations.extend(relocation_candidates_from_index(
                        evidence,
                        &relocation_index,
                    ));
                }
            }
            let status = if drifted_paths.is_empty() {
                current_count += 1;
                "current"
            } else if !relocations.is_empty() {
                drifted_count += 1;
                relocation_count += 1;
                "relocation_suggested"
            } else {
                drifted_count += 1;
                "drifted"
            };
            json!({
                "candidate_id": memory.candidate_id,
                "status": status,
                "drifted_paths": drifted_paths,
                "relocation_candidates": relocations.into_iter().take(3).collect::<Vec<_>>(),
                "source_bodies_returned": 0,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema": "elon.project_context_memory_health.v1",
        "checked_count": items.len(),
        "current_count": current_count,
        "drifted_count": drifted_count,
        "relocation_suggested_count": relocation_count,
        "truncated": memories.len() > MAX_HEALTH_MEMORIES,
        "items": items,
        "authority": "diagnostic_only",
        "repair_policy": "Re-open current files with native tools and create a reviewed replacement; never rewrite evidence paths automatically.",
        "source_bodies_returned": 0,
    })
}
