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

fn execute_in_isolated_worktree(
    dry_run: &SymbolPatchDryRunResponse,
    generated_diff: &str,
) -> PatchVerificationExecutionReport {
    let source_git_root = match dry_run.workspace.git_root.as_ref().map(PathBuf::from) {
        Some(path) => path,
        None => return blocked_report(dry_run, Path::new(&dry_run.workspace.requested_path)),
    };
    let run_workspace = temp_run_dir();
    let mut report = PatchVerificationExecutionReport::new(
        PatchVerificationExecutionStatus::SetupFailed,
        dry_run,
        Some(source_git_root.display().to_string()),
        Some(run_workspace.display().to_string()),
    );

    let setup = run_git_worktree_add(&source_git_root, &run_workspace);
    if !setup.success() {
        report
            .blocked_reasons
            .push("isolated_worktree_create_failed".to_string());
        report.warnings.push(setup.compact_error());
        return report;
    }

    let patch_file = run_workspace.join(".elon-symbol-generated.patch");
    report.patch_file = Some(patch_file.display().to_string());
    if let Err(error) = fs::write(&patch_file, generated_diff) {
        report
            .blocked_reasons
            .push("write_patch_file_failed".to_string());
        report.warnings.push(error.to_string());
        report.run_workspace_removed =
            cleanup_worktree(&source_git_root, &run_workspace, &mut report.warnings);
        return report;
    }

    let apply = run_fixed_command(
        &run_workspace,
        "git apply --whitespace=nowarn .elon-symbol-generated.patch",
        "git",
        &[
            "apply".to_string(),
            "--whitespace=nowarn".to_string(),
            ".elon-symbol-generated.patch".to_string(),
        ],
        "patch_apply",
        true,
        60,
    );
    report.command_results.push(apply.as_result_input());
    report.patch_applied = apply.exit_code == Some(0) && !apply.timed_out;
    report.executed_commands.push(apply);

    if !report.patch_applied {
        report.status = PatchVerificationExecutionStatus::ApplyFailed;
        report.next_steps = vec![
            "Use verification_repair_context.retry_prompt to regenerate an applicable diff."
                .to_string(),
            "Rerun patch dry-run before any apply attempt.".to_string(),
        ];
        report.run_workspace_removed =
            cleanup_worktree(&source_git_root, &run_workspace, &mut report.warnings);
        return report;
    }

    for command in &dry_run.verification_plan.commands {
        if !command.auto_runnable_after_apply {
            report.skipped_commands.push(skipped_command(
                command,
                "command_requires_manual_review_or_is_not_safe_to_auto_run",
            ));
            continue;
        }
        if let Err(reason) = validate_command(command) {
            report
                .skipped_commands
                .push(skipped_command(command, &reason));
            continue;
        }

        let executed = run_command(
            &run_workspace,
            &command.command,
            &command.category,
            command.required,
            command.timeout_seconds,
        );
        let failed = executed.exit_code != Some(0) || executed.timed_out;
        report.command_results.push(executed.as_result_input());
        report.executed_commands.push(executed);
        if failed {
            report.status = PatchVerificationExecutionStatus::VerificationFailed;
            report.next_steps = vec![
                "Use verification_repair_context.retry_prompt to generate an incremental repair diff.".to_string(),
                "Run patch dry-run against the repaired diff before another verification run.".to_string(),
            ];
            report.run_workspace_removed =
                cleanup_worktree(&source_git_root, &run_workspace, &mut report.warnings);
            return report;
        }
    }

    report.status = if report
        .skipped_commands
        .iter()
        .any(|command| command.required)
    {
        PatchVerificationExecutionStatus::ManualVerificationRequired
    } else {
        PatchVerificationExecutionStatus::Passed
    };
    report.next_steps = match report.status {
        PatchVerificationExecutionStatus::Passed => vec![
            "Verification commands passed in an isolated worktree.".to_string(),
            "The original workspace was not modified by this run.".to_string(),
        ],
        PatchVerificationExecutionStatus::ManualVerificationRequired => vec![
            "Auto-runnable commands passed, but manual verification commands remain.".to_string(),
            "Run skipped commands before treating the patch as fully verified.".to_string(),
        ],
        _ => Vec::new(),
    };
    report.run_workspace_removed =
        cleanup_worktree(&source_git_root, &run_workspace, &mut report.warnings);
    report
}

fn blocked_report(
    dry_run: &SymbolPatchDryRunResponse,
    workspace: &Path,
) -> PatchVerificationExecutionReport {
    let mut report = PatchVerificationExecutionReport::new(
        PatchVerificationExecutionStatus::Blocked,
        dry_run,
        dry_run.workspace.git_root.clone(),
        None,
    );
    report.source_workspace = workspace.display().to_string();
    report
        .blocked_reasons
        .extend(dry_run.apply_gate.blockers.clone());
    report
        .blocked_reasons
        .extend(dry_run.verification_plan.blocked_reasons.clone());
    report.blocked_reasons = dedupe(report.blocked_reasons);
    report.next_steps = vec![
        "Resolve apply_gate blockers before creating an isolated verification worktree."
            .to_string(),
    ];
    report
}

fn run_git_worktree_add(source_git_root: &Path, run_workspace: &Path) -> ProcessOutput {
    run_process(
        source_git_root,
        "git",
        &[
            "worktree".to_string(),
            "add".to_string(),
            "--detach".to_string(),
            run_workspace.display().to_string(),
            "HEAD".to_string(),
        ],
        60,
    )
}

fn cleanup_worktree(
    source_git_root: &Path,
    run_workspace: &Path,
    warnings: &mut Vec<String>,
) -> bool {
    let output = run_process(
        source_git_root,
        "git",
        &[
            "worktree".to_string(),
            "remove".to_string(),
            "--force".to_string(),
            run_workspace.display().to_string(),
        ],
        60,
    );
    if output.success() {
        true
    } else {
        warnings.push(format!(
            "isolated_worktree_cleanup_failed: {}",
            output.compact_error()
        ));
        false
    }
}

fn run_command(
    workdir: &Path,
    command: &str,
    category: &str,
    required: bool,
    timeout_seconds: u64,
) -> PatchVerificationExecutedCommand {
    let started = Instant::now();
    let parsed = parse_command(command);
    let output = match parsed {
        Ok((program, args)) => run_process(workdir, &program, &args, timeout_seconds),
        Err(error) => ProcessOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: error,
            timed_out: false,
        },
    };
    PatchVerificationExecutedCommand {
        command: command.to_string(),
        category: category.to_string(),
        required,
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        elapsed_ms: started.elapsed().as_millis() as u64,
        timed_out: output.timed_out,
    }
}

fn run_fixed_command(
    workdir: &Path,
    display_command: &str,
    program: &str,
    args: &[String],
    category: &str,
    required: bool,
    timeout_seconds: u64,
) -> PatchVerificationExecutedCommand {
    let started = Instant::now();
    let output = run_process(workdir, program, args, timeout_seconds);
    PatchVerificationExecutedCommand {
        command: display_command.to_string(),
        category: category.to_string(),
        required,
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        elapsed_ms: started.elapsed().as_millis() as u64,
        timed_out: output.timed_out,
    }
}

fn validate_command(command: &PatchVerificationCommand) -> Result<(), String> {
    parse_command(&command.command).map(|_| ())
}

fn parse_command(command: &str) -> Result<(String, Vec<String>), String> {
    if command
        .chars()
        .any(|ch| matches!(ch, '&' | '|' | ';' | '<' | '>' | '`' | '"' | '\''))
    {
        return Err("command_contains_shell_or_quote_metacharacter".to_string());
    }
    let parts = command
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let Some((program, args)) = parts.split_first() else {
        return Err("command_empty".to_string());
    };
    if !is_allowed_program_args(program, args) {
        return Err("command_not_allowed_for_auto_verification".to_string());
    }
    Ok((program.clone(), args.to_vec()))
}

fn is_allowed_program_args(program: &str, args: &[String]) -> bool {
    match program {
        "git" => {
            matches!(args, [a, b] if a == "diff" && b == "--check")
                || matches!(args, [a, b] if a == "status" && b == "--short")
        }
        "cargo" => {
            args.first()
                .is_some_and(|arg| matches!(arg.as_str(), "check" | "test" | "clippy"))
                || matches!(args, [a, b] if a == "fmt" && b == "--check")
        }
        _ => false,
    }
}

fn run_process(
    workdir: &Path,
    program: &str,
    args: &[String],
    timeout_seconds: u64,
) -> ProcessOutput {
    let mut child = match Command::new(program)
        .current_dir(workdir)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return ProcessOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: error.to_string(),
                timed_out: false,
            };
        }
    };

    let deadline = Instant::now() + Duration::from_secs(timeout_seconds.max(1));
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break process_output(child, false),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                break process_output(child, true);
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = child.kill();
                break ProcessOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: error.to_string(),
                    timed_out: false,
                };
            }
        }
    }
}

fn process_output(child: std::process::Child, timed_out: bool) -> ProcessOutput {
    match child.wait_with_output() {
        Ok(output) => ProcessOutput {
            exit_code: output.status.code(),
            stdout: output_text(&output.stdout),
            stderr: output_text(&output.stderr),
            timed_out,
        },
        Err(error) => ProcessOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
            timed_out,
        },
    }
}

fn skipped_command(
    command: &PatchVerificationCommand,
    reason: &str,
) -> PatchVerificationSkippedCommand {
    PatchVerificationSkippedCommand {
        command: command.command.clone(),
        category: command.category.clone(),
        required: command.required,
        reason: reason.to_string(),
    }
}

fn temp_run_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "elon-symbol-patch-verify-run-{}-{nanos}",
        std::process::id()
    ))
}

fn output_text(bytes: &[u8]) -> String {
    truncate_text(String::from_utf8_lossy(bytes).to_string())
}

fn truncate_text(text: String) -> String {
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

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        if !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

struct ProcessOutput {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

impl ProcessOutput {
    fn success(&self) -> bool {
        self.exit_code == Some(0) && !self.timed_out
    }

    fn compact_error(&self) -> String {
        let text = if self.stderr.trim().is_empty() {
            self.stdout.trim()
        } else {
            self.stderr.trim()
        };
        if text.is_empty() {
            "unknown process failure".to_string()
        } else {
            text.to_string()
        }
    }
}
