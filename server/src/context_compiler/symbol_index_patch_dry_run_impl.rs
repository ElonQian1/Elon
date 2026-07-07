use super::*;

pub(super) fn inspect_workspace(workspace: &Path) -> PatchDryRunWorkspace {
    let requested_path = workspace.display().to_string();
    let mut report = PatchDryRunWorkspace {
        requested_path,
        resolved_path: None,
        git_root: None,
        head: None,
        clean: false,
        status_lines: Vec::new(),
        warnings: Vec::new(),
    };

    if !workspace.exists() {
        report.warnings.push("workspace_not_found".to_string());
        return report;
    }
    if !workspace.is_dir() {
        report.warnings.push("workspace_not_directory".to_string());
        return report;
    }

    let resolved = match workspace.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            report
                .warnings
                .push(format!("workspace_canonicalize_failed: {error}"));
            workspace.to_path_buf()
        }
    };
    report.resolved_path = Some(resolved.display().to_string());

    let root_output = run_git(&resolved, &["rev-parse", "--show-toplevel"]);
    if !root_output.success {
        report.warnings.push(format!(
            "git_root_unavailable: {}",
            first_non_empty(Some(root_output.stderr.as_str()))
                .or_else(|| first_non_empty(Some(root_output.stdout.as_str())))
                .unwrap_or("not a git repository")
        ));
        return report;
    }

    let Some(git_root_text) = first_non_empty(Some(root_output.stdout.as_str())) else {
        report.warnings.push("git_root_empty".to_string());
        return report;
    };
    let git_root = PathBuf::from(git_root_text);
    report.git_root = Some(git_root.display().to_string());

    let head_output = run_git(&git_root, &["rev-parse", "--short", "HEAD"]);
    if head_output.success {
        report.head = first_non_empty(Some(head_output.stdout.as_str())).map(ToOwned::to_owned);
    } else {
        report.warnings.push(format!(
            "git_head_unavailable: {}",
            first_non_empty(Some(head_output.stderr.as_str()))
                .or_else(|| first_non_empty(Some(head_output.stdout.as_str())))
                .unwrap_or("HEAD is unavailable")
        ));
    }

    let status_output = run_git(&git_root, &["status", "--short"]);
    if status_output.success {
        report.status_lines = status_output
            .stdout
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        report.clean = report.status_lines.is_empty();
        if !report.clean {
            report.warnings.push(format!(
                "dirty_worktree: {} status lines",
                report.status_lines.len()
            ));
        }
    } else {
        report.warnings.push(format!(
            "git_status_unavailable: {}",
            first_non_empty(Some(status_output.stderr.as_str()))
                .or_else(|| first_non_empty(Some(status_output.stdout.as_str())))
                .unwrap_or("status failed")
        ));
    }

    report
}

pub(super) fn run_git_apply_check(git_root: &Path, generated_diff: &str) -> PatchApplyCheckResult {
    let patch_file = temp_patch_file();
    if let Err(error) = fs::write(&patch_file, generated_diff) {
        return PatchApplyCheckResult {
            attempted: false,
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            command: None,
            patch_file: Some(patch_file.display().to_string()),
            error: Some(format!("write_patch_failed: {error}")),
        };
    }

    let command = format!(
        "git -C {} apply --check --whitespace=nowarn {}",
        quote_path(git_root),
        quote_path(&patch_file)
    );
    let output = Command::new("git")
        .arg("-C")
        .arg(git_root)
        .args(["apply", "--check", "--whitespace=nowarn"])
        .arg(&patch_file)
        .output();
    let _ = fs::remove_file(&patch_file);

    match output {
        Ok(output) => PatchApplyCheckResult {
            attempted: true,
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: output_text(&output.stdout),
            stderr: output_text(&output.stderr),
            command: Some(command),
            patch_file: Some(patch_file.display().to_string()),
            error: None,
        },
        Err(error) => PatchApplyCheckResult {
            attempted: true,
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            command: Some(command),
            patch_file: Some(patch_file.display().to_string()),
            error: Some(format!("git_apply_check_failed: {error}")),
        },
    }
}

pub(super) fn run_git(workdir: &Path, args: &[&str]) -> GitRunOutput {
    match Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(args)
        .output()
    {
        Ok(output) => GitRunOutput {
            success: output.status.success(),
            stdout: output_text(&output.stdout),
            stderr: output_text(&output.stderr),
        },
        Err(error) => GitRunOutput {
            success: false,
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

pub(super) fn dry_run_next_steps(
    contract_check: &SymbolPatchDiffCheck,
    apply_check: &PatchApplyCheckResult,
    apply_gate: &PatchApplyGate,
) -> Vec<String> {
    if apply_gate.ready_to_apply {
        let mut steps = vec![
            format!(
                "Save the exact generated diff with sha256={} to a patch file.",
                apply_gate.patch_sha256
            ),
            "Run the safe apply command from apply_gate.safe_apply_command.".to_string(),
        ];
        steps.extend(
            apply_gate
                .verification_commands
                .iter()
                .map(|command| format!("Verify after apply: {command}")),
        );
        return steps;
    }
    if !contract_check.accepted_for_apply_check {
        return contract_check.next_steps.clone();
    }
    if !apply_check.attempted {
        return vec!["Fix the workspace or git environment, then rerun patch dry-run.".to_string()];
    }
    if apply_check.success {
        return vec![
            "Patch dry-run passed against the current git worktree.".to_string(),
            "Apply the same diff only in a clean worktree, then run required tests.".to_string(),
        ];
    }
    vec![
        "Regenerate the diff against the current file contents.".to_string(),
        "Inspect git apply --check stderr before attempting any apply step.".to_string(),
    ]
}

pub(super) fn build_apply_gate(
    generation: &SymbolPatchGeneration,
    generated_diff: &str,
    contract_check: &SymbolPatchDiffCheck,
    workspace: &PatchDryRunWorkspace,
    apply_check: &PatchApplyCheckResult,
) -> PatchApplyGate {
    let mut blockers = Vec::new();
    let mut warnings = workspace.warnings.clone();
    let patch_sha256 = sha256_hex(generated_diff);
    let touched_files = contract_check
        .touched_files
        .iter()
        .map(|file| file.file_path.clone())
        .collect::<Vec<_>>();

    if !contract_check.accepted_for_apply_check {
        blockers.push("diff_contract_not_accepted".to_string());
        blockers.extend(
            contract_check
                .violations
                .iter()
                .map(|violation| format!("contract_violation:{}", violation.code)),
        );
    }
    if workspace.git_root.is_none() {
        blockers.push("workspace_git_root_unavailable".to_string());
    }
    if workspace.head.is_none() {
        blockers.push("workspace_head_unavailable".to_string());
    }
    if !workspace.clean {
        blockers.push("workspace_not_clean".to_string());
    }
    if !apply_check.attempted {
        blockers.push("git_apply_check_not_attempted".to_string());
    } else if !apply_check.success {
        blockers.push("git_apply_check_failed".to_string());
    }

    let status = if generation.apply_readiness.requires_generated_diff
        || contract_check.accepted_for_apply_check
    {
        if blockers.is_empty() {
            PatchApplyGateStatus::Ready
        } else {
            PatchApplyGateStatus::Blocked
        }
    } else {
        PatchApplyGateStatus::NotApplicable
    };
    let ready_to_apply = status == PatchApplyGateStatus::Ready;
    if !ready_to_apply && workspace.clean && apply_check.success {
        warnings.push("dry_run_passed_but_apply_gate_blocked".to_string());
    }

    PatchApplyGate {
        status,
        ready_to_apply,
        patch_sha256,
        touched_files,
        required_actions: required_actions(&blockers, ready_to_apply),
        safe_apply_command: ready_to_apply
            .then(|| safe_apply_command(workspace.git_root.as_deref().unwrap_or_default())),
        verification_commands: generation.apply_readiness.post_apply_checks.clone(),
        rollback_hint: generation.apply_readiness.rollback_strategy.clone(),
        blockers: dedupe(blockers),
        warnings: dedupe(warnings),
    }
}

pub(super) fn required_actions(blockers: &[String], ready_to_apply: bool) -> Vec<String> {
    if ready_to_apply {
        return vec![
            "Apply only the exact diff matching patch_sha256.".to_string(),
            "Run every verification command after apply.".to_string(),
            "If verification fails, use rollback_hint and do not commit.".to_string(),
        ];
    }

    let mut actions = Vec::new();
    if blockers
        .iter()
        .any(|blocker| blocker == "diff_contract_not_accepted")
    {
        actions.push("Regenerate the diff so it satisfies the diff contract.".to_string());
    }
    if blockers
        .iter()
        .any(|blocker| blocker == "workspace_not_clean")
    {
        actions
            .push("Commit, stash, or move unrelated workspace changes before apply.".to_string());
    }
    if blockers
        .iter()
        .any(|blocker| blocker == "git_apply_check_failed")
    {
        actions.push("Regenerate the diff against the current file contents.".to_string());
    }
    if blockers.iter().any(|blocker| {
        blocker == "workspace_git_root_unavailable"
            || blocker == "workspace_head_unavailable"
            || blocker == "git_apply_check_not_attempted"
    }) {
        actions.push("Fix the git workspace, then rerun patch dry-run.".to_string());
    }
    if actions.is_empty() {
        actions.push("Rerun patch dry-run after resolving apply gate blockers.".to_string());
    }
    dedupe(actions)
}

pub(super) fn safe_apply_command(git_root: &str) -> String {
    format!(
        "git -C {} apply --whitespace=nowarn <generated.patch>",
        quote_path(Path::new(git_root))
    )
}

pub(super) fn temp_patch_file() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "elon-symbol-patch-dry-run-{}-{nanos}.patch",
        std::process::id()
    ))
}

pub(super) fn output_text(bytes: &[u8]) -> String {
    truncate_text(String::from_utf8_lossy(bytes).to_string())
}

pub(super) fn truncate_text(text: String) -> String {
    let mut out = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= COMMAND_OUTPUT_LIMIT {
            out.push_str("...<truncated>");
            return out;
        }
        out.push(ch);
    }
    out
}

pub(super) fn first_non_empty(text: Option<&str>) -> Option<&str> {
    text?.lines().map(str::trim).find(|line| !line.is_empty())
}

pub(super) fn quote_path(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

pub(super) fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

pub(super) fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

impl PatchApplyCheckResult {
    pub(super) fn not_attempted(message: &str) -> Self {
        Self {
            attempted: false,
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            command: None,
            patch_file: None,
            error: Some(message.to_string()),
        }
    }
}

pub(super) struct GitRunOutput {
    success: bool,
    stdout: String,
    stderr: String,
}
