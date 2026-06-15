use serde::Serialize;

use super::{
    symbol_index_patch_check::SymbolPatchDiffCheck,
    symbol_index_patch_dry_run::{PatchApplyCheckResult, PatchApplyGate, PatchDryRunWorkspace},
    symbol_index_patch_generation_types::SymbolPatchGeneration,
};

const DIFF_EXCERPT_LIMIT: usize = 3_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchRepairContext {
    pub(crate) model_repair_required: bool,
    pub(crate) failure_kind: String,
    pub(crate) patch_sha256: String,
    pub(crate) task: String,
    pub(crate) allowed_files: Vec<String>,
    pub(crate) inspect_only_files: Vec<String>,
    pub(crate) touched_files: Vec<String>,
    pub(crate) blockers: Vec<String>,
    pub(crate) contract_violations: Vec<PatchRepairIssue>,
    pub(crate) workspace_head: Option<String>,
    pub(crate) workspace_status_lines: Vec<String>,
    pub(crate) failed_command: Option<String>,
    pub(crate) failure_stdout: String,
    pub(crate) failure_stderr: String,
    pub(crate) generated_diff_excerpt: String,
    pub(crate) repair_instructions: Vec<String>,
    pub(crate) retry_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchRepairIssue {
    pub(crate) code: String,
    pub(crate) severity: String,
    pub(crate) file_path: Option<String>,
    pub(crate) message: String,
}

pub(crate) fn build_patch_repair_context(
    generation: &SymbolPatchGeneration,
    generated_diff: &str,
    contract_check: &SymbolPatchDiffCheck,
    workspace: &PatchDryRunWorkspace,
    apply_check: &PatchApplyCheckResult,
    apply_gate: &PatchApplyGate,
) -> PatchRepairContext {
    let failure_kind = failure_kind(contract_check, workspace, apply_check, apply_gate);
    let model_repair_required = matches!(
        failure_kind.as_str(),
        "diff_contract_rejected" | "git_apply_check_failed"
    );
    let repair_instructions = repair_instructions(
        generation,
        &failure_kind,
        contract_check,
        workspace,
        apply_check,
        apply_gate,
    );
    let retry_prompt = model_repair_required.then(|| {
        retry_prompt(
            generation,
            &failure_kind,
            contract_check,
            apply_check,
            generated_diff,
        )
    });

    PatchRepairContext {
        model_repair_required,
        failure_kind,
        patch_sha256: apply_gate.patch_sha256.clone(),
        task: generation.task.clone(),
        allowed_files: contract_check.allowed_files.clone(),
        inspect_only_files: contract_check.inspect_only_files.clone(),
        touched_files: apply_gate.touched_files.clone(),
        blockers: apply_gate.blockers.clone(),
        contract_violations: contract_check
            .violations
            .iter()
            .map(|violation| PatchRepairIssue {
                code: violation.code.clone(),
                severity: violation.severity.clone(),
                file_path: violation.file_path.clone(),
                message: violation.message.clone(),
            })
            .collect(),
        workspace_head: workspace.head.clone(),
        workspace_status_lines: workspace.status_lines.clone(),
        failed_command: apply_check.command.clone(),
        failure_stdout: apply_check.stdout.clone(),
        failure_stderr: apply_check.stderr.clone(),
        generated_diff_excerpt: excerpt(generated_diff),
        repair_instructions,
        retry_prompt,
    }
}

fn failure_kind(
    contract_check: &SymbolPatchDiffCheck,
    workspace: &PatchDryRunWorkspace,
    apply_check: &PatchApplyCheckResult,
    apply_gate: &PatchApplyGate,
) -> String {
    if apply_gate.ready_to_apply {
        "none".to_string()
    } else if !contract_check.accepted_for_apply_check {
        "diff_contract_rejected".to_string()
    } else if apply_check.attempted && !apply_check.success {
        "git_apply_check_failed".to_string()
    } else if workspace.git_root.is_some() && !workspace.clean {
        "workspace_not_clean".to_string()
    } else if workspace.git_root.is_none() || workspace.head.is_none() {
        "workspace_unavailable".to_string()
    } else if !apply_check.attempted {
        "git_apply_check_not_attempted".to_string()
    } else {
        "apply_gate_blocked".to_string()
    }
}

fn repair_instructions(
    generation: &SymbolPatchGeneration,
    failure_kind: &str,
    contract_check: &SymbolPatchDiffCheck,
    workspace: &PatchDryRunWorkspace,
    apply_check: &PatchApplyCheckResult,
    apply_gate: &PatchApplyGate,
) -> Vec<String> {
    let mut instructions = Vec::new();
    match failure_kind {
        "none" => {
            instructions.push(
                "No repair is needed; apply only the exact patch_sha256 in a clean worktree."
                    .to_string(),
            );
        }
        "diff_contract_rejected" => {
            instructions
                .push("Regenerate a unified diff that only touches allowed_files.".to_string());
            instructions.push(
                "Do not include prose, Markdown fences, binary patches, or inspect_only files."
                    .to_string(),
            );
        }
        "git_apply_check_failed" => {
            instructions.push(
                "Regenerate the diff against the current workspace file contents.".to_string(),
            );
            instructions
                .push("Use git apply --check stderr as the primary failure evidence.".to_string());
        }
        "workspace_not_clean" => {
            instructions.push("Do not ask the model for a new diff yet; first make the workspace clean or use an isolated worktree.".to_string());
        }
        "workspace_unavailable" | "git_apply_check_not_attempted" => {
            instructions.push(
                "Fix the git workspace path, then rerun patch dry-run with the same diff."
                    .to_string(),
            );
        }
        _ => {
            instructions.push("Resolve apply gate blockers, then rerun patch dry-run.".to_string())
        }
    }
    instructions.extend(apply_gate.required_actions.iter().cloned());
    instructions.extend(contract_check.next_steps.iter().cloned());
    if let Some(error) = apply_check
        .error
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        instructions.push(format!("Apply check error: {error}"));
    }
    if !workspace.status_lines.is_empty() {
        instructions.push(format!(
            "Workspace has {} status lines; avoid mixing unrelated changes into repair.",
            workspace.status_lines.len()
        ));
    }
    instructions.extend(
        generation
            .apply_readiness
            .source_requirements
            .iter()
            .take(4)
            .map(|item| format!("Before regenerating, {item}.")),
    );
    dedupe(instructions)
}

fn retry_prompt(
    generation: &SymbolPatchGeneration,
    failure_kind: &str,
    contract_check: &SymbolPatchDiffCheck,
    apply_check: &PatchApplyCheckResult,
    generated_diff: &str,
) -> String {
    let allowed = list_or_none(&contract_check.allowed_files);
    let violations = contract_check
        .violations
        .iter()
        .map(|violation| {
            format!(
                "- {} {} {}",
                violation.code,
                violation.file_path.as_deref().unwrap_or("-"),
                violation.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<patch_repair_task>\nTask:\n{}\n\nFailure kind: {}\n\nAllowed files:\n{}\n\nContract violations:\n{}\n\nGit apply stderr:\n{}\n\nPrevious diff excerpt:\n{}\n\nRules:\n- Output a corrected unified diff only.\n- Touch only allowed files.\n- Preserve unrelated code.\n- Regenerate against the current file contents.\n</patch_repair_task>",
        generation.task,
        failure_kind,
        allowed,
        if violations.is_empty() {
            "- none".to_string()
        } else {
            violations
        },
        empty_dash(&apply_check.stderr),
        excerpt(generated_diff)
    )
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "- none".to_string()
    } else {
        values
            .iter()
            .map(|value| format!("- {value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn empty_dash(value: &str) -> String {
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        value.trim().to_string()
    }
}

fn excerpt(value: &str) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= DIFF_EXCERPT_LIMIT {
            out.push_str("...<truncated>");
            return out;
        }
        out.push(ch);
    }
    out
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}
