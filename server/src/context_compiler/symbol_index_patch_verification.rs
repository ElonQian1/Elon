use serde::Serialize;

use super::symbol_index_patch_generation_types::SymbolPatchGeneration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationPlan {
    pub(crate) status: PatchVerificationStatus,
    pub(crate) ready_to_verify_after_apply: bool,
    pub(crate) workspace_root: Option<String>,
    pub(crate) commands: Vec<PatchVerificationCommand>,
    pub(crate) blocked_reasons: Vec<String>,
    pub(crate) failure_categories: Vec<String>,
    pub(crate) repair_policy: PatchVerificationRepairPolicy,
    pub(crate) next_steps: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchVerificationStatus {
    ReadyAfterApply,
    Blocked,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationCommand {
    pub(crate) order: usize,
    pub(crate) command: String,
    pub(crate) category: String,
    pub(crate) required: bool,
    pub(crate) auto_runnable_after_apply: bool,
    pub(crate) requires_clean_worktree: bool,
    pub(crate) timeout_seconds: u64,
    pub(crate) failure_kind: String,
    pub(crate) repair_hint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchVerificationRepairPolicy {
    pub(crate) max_repair_attempts: usize,
    pub(crate) model_repair_on_failure: bool,
    pub(crate) rerun_patch_dry_run_after_repair: bool,
    pub(crate) keep_within_allowed_files: Vec<String>,
    pub(crate) collect_outputs: Vec<String>,
    pub(crate) stop_conditions: Vec<String>,
}

pub(crate) fn build_patch_verification_plan(
    generation: &SymbolPatchGeneration,
    ready_to_apply: bool,
    blockers: &[String],
    workspace_root: Option<&str>,
) -> PatchVerificationPlan {
    let commands = verification_commands(generation, ready_to_apply);
    let status = if !generation.apply_readiness.requires_generated_diff {
        PatchVerificationStatus::NotApplicable
    } else if ready_to_apply {
        PatchVerificationStatus::ReadyAfterApply
    } else {
        PatchVerificationStatus::Blocked
    };
    let ready_to_verify_after_apply = status == PatchVerificationStatus::ReadyAfterApply;
    let blocked_reasons = if ready_to_verify_after_apply {
        Vec::new()
    } else if status == PatchVerificationStatus::NotApplicable {
        generation.blocked_reasons.clone()
    } else {
        blockers.to_vec()
    };
    let failure_categories = commands
        .iter()
        .map(|command| command.failure_kind.clone())
        .collect::<Vec<_>>();

    PatchVerificationPlan {
        status,
        ready_to_verify_after_apply,
        workspace_root: workspace_root.map(ToOwned::to_owned),
        repair_policy: repair_policy(generation, ready_to_verify_after_apply),
        next_steps: next_steps(status, &commands, &blocked_reasons),
        commands,
        blocked_reasons: dedupe(blocked_reasons),
        failure_categories: dedupe(failure_categories),
    }
}

fn verification_commands(
    generation: &SymbolPatchGeneration,
    ready_to_apply: bool,
) -> Vec<PatchVerificationCommand> {
    generation
        .apply_readiness
        .post_apply_checks
        .iter()
        .enumerate()
        .map(|(index, command)| classify_command(index + 1, command, ready_to_apply))
        .collect()
}

fn classify_command(order: usize, command: &str, ready_to_apply: bool) -> PatchVerificationCommand {
    let normalized = command.trim();
    let lower = normalized.to_ascii_lowercase();
    let (category, timeout_seconds, failure_kind, repair_hint) = if lower == "git diff --check" {
        (
            "diff_hygiene",
            30,
            "diff_check_failed",
            "Fix whitespace, conflict markers, or malformed diff output before commit.",
        )
    } else if lower == "git status --short" {
        (
            "workspace_status",
            15,
            "workspace_dirty_after_apply",
            "Inspect status and keep only files allowed by the patch plan.",
        )
    } else if lower == "cargo fmt --check" {
        (
            "format",
            120,
            "format_failed",
            "Collect rustfmt diagnostics and repair formatting within allowed_files.",
        )
    } else if lower.starts_with("cargo test") {
        (
            "test",
            300,
            "targeted_tests_failed",
            "Collect the failing assertion and build a repair diff within allowed_files.",
        )
    } else if lower.starts_with("cargo check") {
        (
            "compile_check",
            300,
            "cargo_check_failed",
            "Collect compiler diagnostics and repair the minimal affected code.",
        )
    } else if lower.starts_with("cargo clippy") {
        (
            "lint",
            300,
            "clippy_failed",
            "Collect lint diagnostics and repair only task-owned files.",
        )
    } else {
        (
            "manual",
            120,
            "manual_verification_failed",
            "Run manually and attach the output before requesting a repair diff.",
        )
    };

    PatchVerificationCommand {
        order,
        command: normalized.to_string(),
        category: category.to_string(),
        required: true,
        auto_runnable_after_apply: ready_to_apply && is_safe_post_apply_command(&lower),
        requires_clean_worktree: true,
        timeout_seconds,
        failure_kind: failure_kind.to_string(),
        repair_hint: repair_hint.to_string(),
    }
}

fn is_safe_post_apply_command(lower: &str) -> bool {
    !has_shell_metachar(lower)
        && (lower == "git diff --check"
            || lower == "git status --short"
            || lower == "cargo fmt --check"
            || lower.starts_with("cargo test")
            || lower.starts_with("cargo check")
            || lower.starts_with("cargo clippy"))
}

fn has_shell_metachar(value: &str) -> bool {
    value
        .chars()
        .any(|ch| matches!(ch, '&' | '|' | ';' | '<' | '>' | '`'))
}

fn repair_policy(
    generation: &SymbolPatchGeneration,
    ready_to_verify_after_apply: bool,
) -> PatchVerificationRepairPolicy {
    PatchVerificationRepairPolicy {
        max_repair_attempts: if ready_to_verify_after_apply { 2 } else { 0 },
        model_repair_on_failure: ready_to_verify_after_apply,
        rerun_patch_dry_run_after_repair: ready_to_verify_after_apply,
        keep_within_allowed_files: generation.diff_contract.allowed_files.clone(),
        collect_outputs: vec![
            "command".to_string(),
            "exit_code".to_string(),
            "stdout".to_string(),
            "stderr".to_string(),
            "git_status_short".to_string(),
        ],
        stop_conditions: vec![
            "verification passes".to_string(),
            "repair attempts exhausted".to_string(),
            "repair would touch a file outside allowed_files".to_string(),
            "workspace contains unrelated changes".to_string(),
        ],
    }
}

fn next_steps(
    status: PatchVerificationStatus,
    commands: &[PatchVerificationCommand],
    blocked_reasons: &[String],
) -> Vec<String> {
    match status {
        PatchVerificationStatus::ReadyAfterApply => {
            let mut steps = vec![
                "Apply the exact patch_sha256 in a clean worktree.".to_string(),
                "Run verification commands in order and capture stdout/stderr.".to_string(),
            ];
            if commands.iter().any(|command| command.category == "manual") {
                steps.push(
                    "Manual commands are present; do not auto-run them without operator review."
                        .to_string(),
                );
            }
            steps.push(
                "If verification fails, request a repair diff using command output and rerun patch dry-run."
                    .to_string(),
            );
            steps
        }
        PatchVerificationStatus::Blocked => vec![format!(
            "Do not verify yet; resolve apply gate blockers first: {}",
            if blocked_reasons.is_empty() {
                "unknown".to_string()
            } else {
                blocked_reasons.join(", ")
            }
        )],
        PatchVerificationStatus::NotApplicable => {
            vec![
                "No post-apply verification is needed because no patch should be applied."
                    .to_string(),
            ]
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_cargo_and_git_commands_are_auto_runnable_only_when_ready() {
        let ready = classify_command(1, "cargo test auth_login", true);
        assert_eq!(ready.category, "test");
        assert_eq!(ready.failure_kind, "targeted_tests_failed");
        assert!(ready.auto_runnable_after_apply);

        let fmt = classify_command(1, "cargo fmt --check", true);
        assert_eq!(fmt.category, "format");
        assert_eq!(fmt.failure_kind, "format_failed");
        assert!(fmt.auto_runnable_after_apply);

        let blocked = classify_command(1, "git diff --check", false);
        assert_eq!(blocked.category, "diff_hygiene");
        assert!(!blocked.auto_runnable_after_apply);
    }

    #[test]
    fn shell_metachar_commands_require_manual_review() {
        let command = classify_command(1, "cargo test auth_login && cargo check", true);
        assert_eq!(command.category, "test");
        assert!(!command.auto_runnable_after_apply);
    }
}
