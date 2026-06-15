use serde::Serialize;

use super::{
    symbol_index_patch_dry_run::SymbolPatchDryRunResponse,
    symbol_index_patch_verification_repair::{
        PatchVerificationCommandResultInput, PatchVerificationRepairContext,
    },
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchVerificationRunResponse {
    pub(crate) dry_run: SymbolPatchDryRunResponse,
    pub(crate) execution: PatchVerificationExecutionReport,
    pub(crate) verification_repair_context: PatchVerificationRepairContext,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationExecutionReport {
    pub(crate) status: PatchVerificationExecutionStatus,
    pub(crate) source_workspace: String,
    pub(crate) source_git_root: Option<String>,
    pub(crate) run_workspace: Option<String>,
    pub(crate) run_workspace_removed: bool,
    pub(crate) patch_file: Option<String>,
    pub(crate) patch_applied: bool,
    pub(crate) command_results: Vec<PatchVerificationCommandResultInput>,
    pub(crate) executed_commands: Vec<PatchVerificationExecutedCommand>,
    pub(crate) skipped_commands: Vec<PatchVerificationSkippedCommand>,
    pub(crate) blocked_reasons: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) next_steps: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchVerificationExecutionStatus {
    Blocked,
    SetupFailed,
    ApplyFailed,
    VerificationFailed,
    ManualVerificationRequired,
    Passed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationExecutedCommand {
    pub(crate) command: String,
    pub(crate) category: String,
    pub(crate) required: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) elapsed_ms: u64,
    pub(crate) timed_out: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationSkippedCommand {
    pub(crate) command: String,
    pub(crate) category: String,
    pub(crate) required: bool,
    pub(crate) reason: String,
}

impl PatchVerificationExecutionReport {
    pub(super) fn new(
        status: PatchVerificationExecutionStatus,
        dry_run: &SymbolPatchDryRunResponse,
        source_git_root: Option<String>,
        run_workspace: Option<String>,
    ) -> Self {
        Self {
            status,
            source_workspace: dry_run.workspace.requested_path.clone(),
            source_git_root,
            run_workspace,
            run_workspace_removed: false,
            patch_file: None,
            patch_applied: false,
            command_results: Vec::new(),
            executed_commands: Vec::new(),
            skipped_commands: Vec::new(),
            blocked_reasons: Vec::new(),
            warnings: Vec::new(),
            next_steps: Vec::new(),
        }
    }
}

impl PatchVerificationExecutedCommand {
    pub(super) fn as_result_input(&self) -> PatchVerificationCommandResultInput {
        PatchVerificationCommandResultInput {
            command: self.command.clone(),
            exit_code: self.exit_code,
            stdout: Some(self.stdout.clone()),
            stderr: Some(self.stderr.clone()),
            elapsed_ms: Some(self.elapsed_ms),
        }
    }
}
