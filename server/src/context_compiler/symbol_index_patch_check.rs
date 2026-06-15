use std::collections::BTreeSet;

use serde::Serialize;

use super::symbol_index_patch_generation_types::{PatchGenerationMode, SymbolPatchGeneration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchDiffCheckStatus {
    AcceptedForApplyCheck,
    Rejected,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchDiffCheck {
    pub(crate) task: String,
    pub(crate) status: PatchDiffCheckStatus,
    pub(crate) accepted_for_apply_check: bool,
    pub(crate) touched_files: Vec<PatchTouchedFile>,
    pub(crate) allowed_files: Vec<String>,
    pub(crate) inspect_only_files: Vec<String>,
    pub(crate) violations: Vec<PatchDiffViolation>,
    pub(crate) warnings: Vec<PatchDiffWarning>,
    pub(crate) apply_check_command: Option<String>,
    pub(crate) next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchTouchedFile {
    pub(crate) file_path: String,
    pub(crate) old_path: Option<String>,
    pub(crate) new_path: Option<String>,
    pub(crate) change_kind: String,
    pub(crate) hunk_count: usize,
    pub(crate) allowed: bool,
    pub(crate) inspect_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchDiffViolation {
    pub(crate) code: String,
    pub(crate) severity: String,
    pub(crate) file_path: Option<String>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchDiffWarning {
    pub(crate) code: String,
    pub(crate) file_path: Option<String>,
    pub(crate) message: String,
}

#[derive(Debug, Default)]
struct DiffFileDraft {
    old_path: Option<String>,
    new_path: Option<String>,
    new_file: bool,
    deleted_file: bool,
    hunk_count: usize,
}

pub(crate) fn check_symbol_patch_diff(
    generation: &SymbolPatchGeneration,
    generated_diff: &str,
) -> SymbolPatchDiffCheck {
    let allowed_files = generation.diff_contract.allowed_files.clone();
    let inspect_only_files = generation.diff_contract.inspect_only_files.clone();
    let allowed = allowed_files.iter().cloned().collect::<BTreeSet<_>>();
    let inspect_only = inspect_only_files.iter().cloned().collect::<BTreeSet<_>>();
    let mut violations = base_violations(generation, generated_diff);
    let mut warnings = Vec::new();
    let touched_files = parse_unified_diff_files(generated_diff)
        .into_iter()
        .map(|draft| touched_file(draft, &allowed, &inspect_only))
        .collect::<Vec<_>>();

    if touched_files.is_empty() && !generated_diff.trim().is_empty() {
        violations.push(violation(
            "no_touched_files",
            None,
            "No touched file was parsed from the generated diff.",
        ));
    }
    for file in &touched_files {
        if file.inspect_only {
            violations.push(violation(
                "inspect_only_file_modified",
                Some(file.file_path.clone()),
                "Generated diff touches a file that the patch plan marked inspect-only.",
            ));
        }
        if !file.allowed {
            violations.push(violation(
                "file_not_allowed",
                Some(file.file_path.clone()),
                "Generated diff touches a file outside diff_contract.allowed_files.",
            ));
        }
        if looks_like_generated_or_lockfile(&file.file_path) && !file.allowed {
            violations.push(violation(
                "generated_or_lockfile_modified",
                Some(file.file_path.clone()),
                "Generated diff touches a generated, build, or lock file that is not explicitly allowed.",
            ));
        }
    }

    if has_prose_outside_unified_diff(generated_diff) {
        violations.push(violation(
            "prose_outside_unified_diff",
            None,
            "Generated output contains prose or fences outside the unified diff.",
        ));
    }
    if contains_binary_patch(generated_diff) {
        violations.push(violation(
            "binary_patch",
            None,
            "Binary patches are not accepted by the symbol patch diff checker.",
        ));
    }
    if !generation.diff_contract.required_tests.is_empty() {
        warnings.push(warning(
            "tests_required_after_apply",
            None,
            format!(
                "Run required tests after apply: {}",
                generation.diff_contract.required_tests.join("; ")
            ),
        ));
    }

    let accepted_for_apply_check = violations.is_empty()
        && generation.apply_readiness.can_run_apply_check
        && !touched_files.is_empty();
    let status = if accepted_for_apply_check {
        PatchDiffCheckStatus::AcceptedForApplyCheck
    } else if generation.mode == PatchGenerationMode::NoPatch {
        PatchDiffCheckStatus::NotApplicable
    } else {
        PatchDiffCheckStatus::Rejected
    };
    let apply_check_command =
        accepted_for_apply_check.then(|| "git apply --check <generated.patch>".to_string());

    SymbolPatchDiffCheck {
        task: generation.task.clone(),
        status,
        accepted_for_apply_check,
        touched_files,
        allowed_files,
        inspect_only_files,
        violations,
        warnings,
        apply_check_command,
        next_steps: next_steps(status, generation),
    }
}

fn base_violations(
    generation: &SymbolPatchGeneration,
    generated_diff: &str,
) -> Vec<PatchDiffViolation> {
    let mut violations = Vec::new();
    if generated_diff.trim().is_empty() {
        violations.push(violation("empty_diff", None, "Generated diff is empty."));
    }
    if generation.mode != PatchGenerationMode::GenerateDiff || !generation.ready_to_generate {
        violations.push(violation(
            "generation_not_ready",
            None,
            "Patch generation is not ready for a code diff for this task.",
        ));
    }
    if !generation.apply_readiness.can_run_apply_check {
        violations.push(violation(
            "apply_check_not_ready",
            None,
            "Patch apply readiness does not allow running git apply --check yet.",
        ));
    }
    violations
}

fn parse_unified_diff_files(diff: &str) -> Vec<DiffFileDraft> {
    let mut files = Vec::<DiffFileDraft>::new();
    let mut current = None::<DiffFileDraft>;
    for line in diff.lines() {
        if let Some((old_path, new_path)) = parse_diff_git_line(line) {
            if let Some(file) = current.take() {
                files.push(file);
            }
            current = Some(DiffFileDraft {
                old_path,
                new_path,
                ..Default::default()
            });
            continue;
        }
        let Some(file) = current.as_mut() else {
            continue;
        };
        if line == "new file mode" || line.starts_with("new file mode ") {
            file.new_file = true;
        } else if line == "deleted file mode" || line.starts_with("deleted file mode ") {
            file.deleted_file = true;
        } else if let Some(path) = line.strip_prefix("--- ") {
            file.old_path = normalize_diff_path(path);
        } else if let Some(path) = line.strip_prefix("+++ ") {
            file.new_path = normalize_diff_path(path);
        } else if line.starts_with("@@") {
            file.hunk_count += 1;
        }
    }
    if let Some(file) = current {
        files.push(file);
    }
    files
}

fn parse_diff_git_line(line: &str) -> Option<(Option<String>, Option<String>)> {
    let rest = line.strip_prefix("diff --git ")?;
    let mut parts = rest.split_whitespace();
    let old_path = parts.next().and_then(normalize_diff_path);
    let new_path = parts.next().and_then(normalize_diff_path);
    Some((old_path, new_path))
}

fn normalize_diff_path(raw: &str) -> Option<String> {
    let path = raw.trim().trim_matches('"');
    if path == "/dev/null" {
        return None;
    }
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path)
        .replace('\\', "/")
        .into()
}

fn touched_file(
    draft: DiffFileDraft,
    allowed: &BTreeSet<String>,
    inspect_only: &BTreeSet<String>,
) -> PatchTouchedFile {
    let file_path = draft
        .new_path
        .clone()
        .or_else(|| draft.old_path.clone())
        .unwrap_or_default();
    PatchTouchedFile {
        allowed: allowed.contains(&file_path),
        inspect_only: inspect_only.contains(&file_path),
        change_kind: change_kind(&draft).to_string(),
        file_path,
        old_path: draft.old_path,
        new_path: draft.new_path,
        hunk_count: draft.hunk_count,
    }
}

fn change_kind(file: &DiffFileDraft) -> &'static str {
    if file.new_file || file.old_path.is_none() {
        "added"
    } else if file.deleted_file || file.new_path.is_none() {
        "deleted"
    } else {
        "modified"
    }
}

fn has_prose_outside_unified_diff(diff: &str) -> bool {
    for line in diff.lines().map(str::trim).filter(|line| !line.is_empty()) {
        return !(line.starts_with("diff --git ")
            || line.starts_with("--- ")
            || line.starts_with("Index: "));
    }
    false
}

fn contains_binary_patch(diff: &str) -> bool {
    diff.lines().any(|line| {
        line.starts_with("GIT binary patch")
            || line.starts_with("Binary files ")
            || line.starts_with("literal ")
    })
}

fn looks_like_generated_or_lockfile(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with("cargo.lock")
        || lower.ends_with("package-lock.json")
        || lower.ends_with("pnpm-lock.yaml")
        || lower.ends_with("yarn.lock")
        || lower.contains("/target/")
        || lower.contains("/build/")
        || lower.contains("/generated/")
}

fn next_steps(status: PatchDiffCheckStatus, generation: &SymbolPatchGeneration) -> Vec<String> {
    match status {
        PatchDiffCheckStatus::AcceptedForApplyCheck => {
            let mut steps = vec![
                "Save generated diff to a patch file.".to_string(),
                "Run git apply --check <generated.patch>.".to_string(),
                "If apply check passes, apply the patch in a clean worktree.".to_string(),
            ];
            steps.extend(
                generation
                    .diff_contract
                    .verification_commands
                    .iter()
                    .cloned(),
            );
            steps
        }
        PatchDiffCheckStatus::Rejected => vec![
            "Regenerate the diff using only allowed_files.".to_string(),
            "Do not edit inspect_only_files without rerunning task-pack or adding evidence."
                .to_string(),
            "Return blocked instead of applying a rejected diff.".to_string(),
        ],
        PatchDiffCheckStatus::NotApplicable => vec![
            "Do not generate or apply a code patch for this context-only task.".to_string(),
            "Use the task pack to answer or inspect instead.".to_string(),
        ],
    }
}

fn violation(code: &str, file_path: Option<String>, message: &str) -> PatchDiffViolation {
    PatchDiffViolation {
        code: code.to_string(),
        severity: "error".to_string(),
        file_path,
        message: message.to_string(),
    }
}

fn warning(code: &str, file_path: Option<String>, message: String) -> PatchDiffWarning {
    PatchDiffWarning {
        code: code.to_string(),
        file_path,
        message,
    }
}
