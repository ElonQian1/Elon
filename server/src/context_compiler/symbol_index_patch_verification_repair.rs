use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    symbol_index_patch_dry_run::{SymbolPatchDryRunResponse, dry_run_symbol_patch},
    symbol_index_patch_generation_types::SymbolPatchGeneration,
    symbol_index_patch_verification::PatchVerificationCommand,
};

const OUTPUT_EXCERPT_LIMIT: usize = 2_000;
const DIFF_EXCERPT_LIMIT: usize = 4_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationCommandResultInput {
    pub(crate) command: String,
    #[serde(alias = "exitCode")]
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: Option<String>,
    pub(crate) stderr: Option<String>,
    #[serde(alias = "elapsedMs")]
    pub(crate) elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchVerificationRepairResponse {
    pub(crate) dry_run: SymbolPatchDryRunResponse,
    pub(crate) verification_repair_context: PatchVerificationRepairContext,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationRepairContext {
    pub(crate) task: String,
    pub(crate) status: PatchVerificationRepairStatus,
    pub(crate) model_repair_required: bool,
    pub(crate) patch_sha256: String,
    pub(crate) allowed_files: Vec<String>,
    pub(crate) failed_commands: Vec<PatchVerificationCommandFailure>,
    pub(crate) passed_commands: Vec<PatchVerificationCommandPass>,
    pub(crate) ignored_results: Vec<String>,
    pub(crate) blocked_reasons: Vec<String>,
    pub(crate) generated_diff_excerpt: String,
    pub(crate) repair_instructions: Vec<String>,
    pub(crate) retry_prompt: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchVerificationRepairStatus {
    ReadyForRepair,
    VerificationPassed,
    VerificationNotReady,
    NoResults,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationCommandFailure {
    pub(crate) command: String,
    pub(crate) category: String,
    pub(crate) failure_kind: String,
    pub(crate) required: bool,
    pub(crate) matched_plan: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) elapsed_ms: Option<u64>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) repair_hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationCommandPass {
    pub(crate) command: String,
    pub(crate) category: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) elapsed_ms: Option<u64>,
}

pub(crate) fn build_symbol_patch_verification_repair_response(
    generation: &SymbolPatchGeneration,
    generated_diff: &str,
    workspace: &Path,
    results: &[PatchVerificationCommandResultInput],
) -> SymbolPatchVerificationRepairResponse {
    let dry_run = dry_run_symbol_patch(generation, generated_diff, workspace);
    let verification_repair_context =
        build_patch_verification_repair_context(generation, &dry_run, generated_diff, results);

    SymbolPatchVerificationRepairResponse {
        dry_run,
        verification_repair_context,
    }
}

pub(crate) fn build_patch_verification_repair_context(
    generation: &SymbolPatchGeneration,
    dry_run: &SymbolPatchDryRunResponse,
    generated_diff: &str,
    results: &[PatchVerificationCommandResultInput],
) -> PatchVerificationRepairContext {
    let mut failed_commands = Vec::new();
    let mut passed_commands = Vec::new();
    let mut ignored_results = Vec::new();

    for result in results {
        let normalized = result.command.trim();
        if normalized.is_empty() {
            ignored_results.push("empty_command".to_string());
            continue;
        }
        let plan_command = find_plan_command(&dry_run.verification_plan.commands, normalized);
        if result.exit_code == Some(0) {
            passed_commands.push(pass_from_result(result, normalized, plan_command));
        } else {
            failed_commands.push(failure_from_result(result, normalized, plan_command));
        }
    }

    let mut blocked_reasons = Vec::new();
    let status = if !dry_run.apply_gate.ready_to_apply {
        blocked_reasons.extend(dry_run.apply_gate.blockers.clone());
        blocked_reasons.extend(dry_run.verification_plan.blocked_reasons.clone());
        PatchVerificationRepairStatus::VerificationNotReady
    } else if results.is_empty() {
        blocked_reasons.push("verification_results_empty".to_string());
        PatchVerificationRepairStatus::NoResults
    } else if failed_commands.is_empty() {
        PatchVerificationRepairStatus::VerificationPassed
    } else {
        PatchVerificationRepairStatus::ReadyForRepair
    };
    blocked_reasons = dedupe(blocked_reasons);

    let repair_instructions = repair_instructions(generation);
    let model_repair_required = status == PatchVerificationRepairStatus::ReadyForRepair;
    let retry_prompt = model_repair_required.then(|| {
        build_retry_prompt(
            generation,
            dry_run,
            generated_diff,
            &failed_commands,
            &repair_instructions,
        )
    });

    PatchVerificationRepairContext {
        task: generation.task.clone(),
        status,
        model_repair_required,
        patch_sha256: dry_run.apply_gate.patch_sha256.clone(),
        allowed_files: generation.diff_contract.allowed_files.clone(),
        failed_commands,
        passed_commands,
        ignored_results,
        blocked_reasons,
        generated_diff_excerpt: truncate_text(generated_diff, DIFF_EXCERPT_LIMIT),
        repair_instructions,
        retry_prompt,
    }
}

fn find_plan_command<'a>(
    commands: &'a [PatchVerificationCommand],
    command: &str,
) -> Option<&'a PatchVerificationCommand> {
    commands
        .iter()
        .find(|candidate| candidate.command.trim() == command)
}

fn pass_from_result(
    result: &PatchVerificationCommandResultInput,
    command: &str,
    plan_command: Option<&PatchVerificationCommand>,
) -> PatchVerificationCommandPass {
    PatchVerificationCommandPass {
        command: command.to_string(),
        category: plan_command
            .map(|command| command.category.clone())
            .unwrap_or_else(|| "manual".to_string()),
        exit_code: result.exit_code,
        elapsed_ms: result.elapsed_ms,
    }
}

fn failure_from_result(
    result: &PatchVerificationCommandResultInput,
    command: &str,
    plan_command: Option<&PatchVerificationCommand>,
) -> PatchVerificationCommandFailure {
    PatchVerificationCommandFailure {
        command: command.to_string(),
        category: plan_command
            .map(|command| command.category.clone())
            .unwrap_or_else(|| "manual".to_string()),
        failure_kind: plan_command
            .map(|command| command.failure_kind.clone())
            .unwrap_or_else(|| "manual_verification_failed".to_string()),
        required: plan_command.map(|command| command.required).unwrap_or(true),
        matched_plan: plan_command.is_some(),
        exit_code: result.exit_code,
        elapsed_ms: result.elapsed_ms,
        stdout: result
            .stdout
            .as_deref()
            .map(|text| truncate_text(text, OUTPUT_EXCERPT_LIMIT))
            .unwrap_or_default(),
        stderr: result
            .stderr
            .as_deref()
            .map(|text| truncate_text(text, OUTPUT_EXCERPT_LIMIT))
            .unwrap_or_default(),
        repair_hint: plan_command
            .map(|command| command.repair_hint.clone())
            .unwrap_or_else(|| {
                "Treat this as a manual verification failure and repair the minimal allowed files."
                    .to_string()
            }),
    }
}

fn repair_instructions(generation: &SymbolPatchGeneration) -> Vec<String> {
    let mut instructions = vec![
        "Return a unified diff only; do not include prose outside the diff.".to_string(),
        "Touch only files listed in allowed_files.".to_string(),
        "Address the failed verification commands directly.".to_string(),
        "Keep the repair incremental relative to the previous generated diff.".to_string(),
        "Rerun patch dry-run after producing the repair diff.".to_string(),
        "Stop after at most two repair attempts unless an operator explicitly continues."
            .to_string(),
    ];
    instructions.extend(
        generation
            .diff_contract
            .forbidden_patterns
            .iter()
            .map(|pattern| format!("Do not emit forbidden pattern: {pattern}")),
    );
    dedupe(instructions)
}

fn build_retry_prompt(
    generation: &SymbolPatchGeneration,
    dry_run: &SymbolPatchDryRunResponse,
    generated_diff: &str,
    failed_commands: &[PatchVerificationCommandFailure],
    repair_instructions: &[String],
) -> String {
    let allowed_files = bullet_list(&generation.diff_contract.allowed_files);
    let failures = failed_commands
        .iter()
        .map(render_failed_command)
        .collect::<Vec<_>>()
        .join("\n");
    let instructions = bullet_list(repair_instructions);
    let diff_excerpt = truncate_text(generated_diff, DIFF_EXCERPT_LIMIT);

    format!(
        "<patch_verification_repair_task>\n\
Task: {task}\n\
Patch sha256: {patch_sha256}\n\
Allowed files:\n{allowed_files}\n\
Failed verification commands:\n{failures}\n\
Previous generated diff excerpt:\n```diff\n{diff_excerpt}\n```\n\
Repair rules:\n{instructions}\n\
</patch_verification_repair_task>",
        task = generation.task,
        patch_sha256 = dry_run.apply_gate.patch_sha256,
    )
}

fn render_failed_command(command: &PatchVerificationCommandFailure) -> String {
    format!(
        "<failed_command>\n\
command: {command}\n\
category: {category}\n\
failure_kind: {failure_kind}\n\
exit_code: {exit_code}\n\
stdout:\n{stdout}\n\
stderr:\n{stderr}\n\
repair_hint: {repair_hint}\n\
</failed_command>",
        command = command.command,
        category = command.category,
        failure_kind = command.failure_kind,
        exit_code = command
            .exit_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        stdout = command.stdout,
        stderr = command.stderr,
        repair_hint = command.repair_hint,
    )
}

fn bullet_list(values: &[String]) -> String {
    if values.is_empty() {
        return "- none".to_string();
    }
    values
        .iter()
        .map(|value| format!("- {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_text(text: &str, limit: usize) -> String {
    let mut out = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= limit {
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
