use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    symbol_index_patch_check::{check_symbol_patch_diff, SymbolPatchDiffCheck},
    symbol_index_patch_generation_types::SymbolPatchGeneration,
    symbol_index_patch_repair::{build_patch_repair_context, PatchRepairContext},
    symbol_index_patch_verification::{build_patch_verification_plan, PatchVerificationPlan},
};

const COMMAND_OUTPUT_LIMIT: usize = 4_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchDryRunResponse {
    pub(crate) task: String,
    pub(crate) accepted_for_apply_check: bool,
    pub(crate) contract_check: SymbolPatchDiffCheck,
    pub(crate) workspace: PatchDryRunWorkspace,
    pub(crate) apply_check: PatchApplyCheckResult,
    pub(crate) apply_gate: PatchApplyGate,
    pub(crate) repair_context: PatchRepairContext,
    pub(crate) verification_plan: PatchVerificationPlan,
    pub(crate) next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchDryRunWorkspace {
    pub(crate) requested_path: String,
    pub(crate) resolved_path: Option<String>,
    pub(crate) git_root: Option<String>,
    pub(crate) head: Option<String>,
    pub(crate) clean: bool,
    pub(crate) status_lines: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchApplyCheckResult {
    pub(crate) attempted: bool,
    pub(crate) success: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) command: Option<String>,
    pub(crate) patch_file: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchApplyGate {
    pub(crate) status: PatchApplyGateStatus,
    pub(crate) ready_to_apply: bool,
    pub(crate) patch_sha256: String,
    pub(crate) touched_files: Vec<String>,
    pub(crate) blockers: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) required_actions: Vec<String>,
    pub(crate) safe_apply_command: Option<String>,
    pub(crate) verification_commands: Vec<String>,
    pub(crate) rollback_hint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchApplyGateStatus {
    Ready,
    Blocked,
    NotApplicable,
}

pub(crate) fn dry_run_symbol_patch(
    generation: &SymbolPatchGeneration,
    generated_diff: &str,
    workspace: &Path,
) -> SymbolPatchDryRunResponse {
    let contract_check = check_symbol_patch_diff(generation, generated_diff);
    let mut workspace = inspect_workspace(workspace);
    let apply_check = if contract_check.accepted_for_apply_check {
        if let Some(git_root) = workspace.git_root.as_ref().map(PathBuf::from) {
            run_git_apply_check(&git_root, generated_diff)
        } else {
            workspace
                .warnings
                .push("workspace_not_ready_for_apply_check".to_string());
            PatchApplyCheckResult::not_attempted("workspace is not a usable git repository")
        }
    } else {
        PatchApplyCheckResult::not_attempted("diff contract was not accepted")
    };
    let apply_gate = build_apply_gate(
        generation,
        generated_diff,
        &contract_check,
        &workspace,
        &apply_check,
    );
    let repair_context = build_patch_repair_context(
        generation,
        generated_diff,
        &contract_check,
        &workspace,
        &apply_check,
        &apply_gate,
    );
    let verification_plan = build_patch_verification_plan(
        generation,
        apply_gate.ready_to_apply,
        &apply_gate.blockers,
        workspace.git_root.as_deref(),
    );
    let next_steps = dry_run_next_steps(&contract_check, &apply_check, &apply_gate);

    SymbolPatchDryRunResponse {
        task: generation.task.clone(),
        accepted_for_apply_check: contract_check.accepted_for_apply_check,
        contract_check,
        workspace,
        apply_check,
        apply_gate,
        repair_context,
        verification_plan,
        next_steps,
    }
}


#[path = "symbol_index_patch_dry_run_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
