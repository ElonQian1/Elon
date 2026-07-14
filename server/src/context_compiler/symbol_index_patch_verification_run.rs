use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use super::{
    symbol_index_patch_dry_run::{dry_run_symbol_patch, SymbolPatchDryRunResponse},
    symbol_index_patch_generation_types::SymbolPatchGeneration,
    symbol_index_patch_verification::PatchVerificationCommand,
    symbol_index_patch_verification_repair::build_patch_verification_repair_context,
    symbol_index_patch_verification_run_types::{
        PatchVerificationExecutedCommand, PatchVerificationExecutionReport,
        PatchVerificationExecutionStatus, PatchVerificationSkippedCommand,
        SymbolPatchVerificationRunResponse,
    },
};

const COMMAND_OUTPUT_LIMIT: usize = 4_000;

pub(crate) fn run_symbol_patch_verification(
    generation: &SymbolPatchGeneration,
    generated_diff: &str,
    workspace: &Path,
) -> SymbolPatchVerificationRunResponse {
    let dry_run = dry_run_symbol_patch(generation, generated_diff, workspace);
    let execution = if dry_run.apply_gate.ready_to_apply {
        execute_in_isolated_worktree(&dry_run, generated_diff)
    } else {
        blocked_report(&dry_run, workspace)
    };
    let verification_repair_context = build_patch_verification_repair_context(
        generation,
        &dry_run,
        generated_diff,
        &execution.command_results,
    );

    SymbolPatchVerificationRunResponse {
        dry_run,
        execution,
        verification_repair_context,
    }
}

#[path = "symbol_index_patch_verification_run_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
