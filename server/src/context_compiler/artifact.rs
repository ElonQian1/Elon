use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::Serialize;
use serde_json::json;

use super::{
    artifact_exports, config::ContextCompilerConfig, context_budget,
    directory_summary::DirectorySummary, model::RepoContextIndex,
    project_manifests::ProjectManifestReport, relevance::RelevantFile, repo_snapshot::RepoSnapshot,
    rust_project::RustProjectSummary, task_context_exports, validation::ValidationPlan,
};

#[derive(Debug, Clone)]
pub(crate) struct ContextPackArtifact {
    pub(crate) path: PathBuf,
    pub(crate) bundle_dir: PathBuf,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) bytes: usize,
}

pub(crate) struct ContextArtifactsInput<'a> {
    pub(crate) data_dir: &'a Path,
    pub(crate) config: &'a ContextCompilerConfig,
    pub(crate) trace_id: Option<&'a str>,
    pub(crate) user_id: &'a str,
    pub(crate) user_message: &'a str,
    pub(crate) pack: &'a str,
    pub(crate) llm_brief: Option<&'a str>,
    pub(crate) snapshot: &'a RepoSnapshot,
    pub(crate) rust_project: Option<&'a RustProjectSummary>,
    pub(crate) project_manifests: Option<&'a ProjectManifestReport>,
    pub(crate) directory_summaries: &'a [DirectorySummary],
    pub(crate) repo_index: Option<&'a RepoContextIndex>,
    pub(crate) relevant_files: &'a [RelevantFile],
    pub(crate) validation_plan: &'a ValidationPlan,
}

pub(crate) fn save_context_artifacts(
    input: ContextArtifactsInput<'_>,
) -> Option<ContextPackArtifact> {
    let config = input.config;
    if !config.save_pack_enabled {
        return None;
    }

    let now = Utc::now();
    let day = now.format("%Y%m%d").to_string();
    let stamp = now.format("%H%M%S%.3f").to_string().replace('.', "");
    let trace = input
        .trace_id
        .map(safe_component)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "no-trace".to_string());
    let user = safe_component(input.user_id);
    let stem = format!("{stamp}-{trace}-{user}");
    let dir = input.data_dir.join("context-compiler").join(day);
    let path = dir.join(format!("{stem}.md"));
    let bundle_dir = dir.join(&stem);
    fs::create_dir_all(&dir).ok()?;
    fs::create_dir_all(&bundle_dir).ok()?;

    let content = clamp_artifact(input.pack, config.artifact_max_bytes);
    let mut files = Vec::new();
    let mut bytes = 0usize;
    bytes += write_text(&path, &content, &mut files)?;
    bytes += write_text(&bundle_dir.join("context_pack.md"), &content, &mut files)?;
    bytes += task_context_exports::write_task_context_exports(
        task_context_exports::TaskContextExportsInput {
            bundle_dir: &bundle_dir,
            created_at: &now,
            trace_id: input.trace_id,
            user_id: input.user_id,
            user_message: input.user_message,
            pack: &content,
            snapshot: input.snapshot,
            repo_index: input.repo_index,
            relevant_files: input.relevant_files,
            validation_plan: input.validation_plan,
        },
        &mut files,
    )?;
    bytes += context_budget::write_context_budget_exports(&bundle_dir, &content, &mut files)?;
    bytes += write_text(
        &bundle_dir.join("brief.md"),
        &build_brief_markdown(input.user_message, input.llm_brief, input.relevant_files),
        &mut files,
    )?;
    bytes += write_text(
        &bundle_dir.join("validation.md"),
        &input.validation_plan.to_markdown(),
        &mut files,
    )?;
    bytes += write_text(
        &bundle_dir.join("agent_prompt.md"),
        &build_agent_prompt(input.user_message),
        &mut files,
    )?;
    bytes += write_json(
        &bundle_dir.join("repo_snapshot.json"),
        input.snapshot,
        &mut files,
    )?;
    bytes += write_json(
        &bundle_dir.join("relevant_files.json"),
        &input.relevant_files,
        &mut files,
    )?;
    bytes += write_json(
        &bundle_dir.join("validation_plan.json"),
        input.validation_plan,
        &mut files,
    )?;
    if let Some(rust_project) = input.rust_project {
        bytes += write_json(
            &bundle_dir.join("rust_project.json"),
            rust_project,
            &mut files,
        )?;
    }
    if let Some(project_manifests) = input.project_manifests {
        bytes += write_json(
            &bundle_dir.join("project_manifests.json"),
            project_manifests,
            &mut files,
        )?;
    }
    if !input.directory_summaries.is_empty() {
        bytes += write_json(
            &bundle_dir.join("directory_summaries.json"),
            input.directory_summaries,
            &mut files,
        )?;
    }
    if let Some(repo_index) = input.repo_index {
        bytes += write_json(
            &bundle_dir.join("repo_context_index.json"),
            repo_index,
            &mut files,
        )?;
    }
    bytes += artifact_exports::write_context_exports(
        &bundle_dir,
        input.repo_index,
        input.project_manifests,
        input.directory_summaries,
        input.validation_plan,
        &mut files,
    )?;
    bytes += write_json(
        &bundle_dir.join("manifest.json"),
        &json!({
            "version": 1,
            "source": "elon-context-compiler",
            "created_at": now.to_rfc3339(),
            "trace_id": input.trace_id,
            "user_id": input.user_id,
            "legacy_context_pack": path.file_name().and_then(|name| name.to_str()).unwrap_or_default(),
            "bundle_files": files
                .iter()
                .filter_map(|file| file.file_name().and_then(|name| name.to_str()))
                .collect::<Vec<_>>(),
            "bundle_file_paths": bundle_file_paths(&bundle_dir, &files),
        }),
        &mut files,
    )?;
    Some(ContextPackArtifact {
        path,
        bundle_dir,
        files,
        bytes,
    })
}

fn bundle_file_paths(bundle_dir: &Path, files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .filter_map(|file| {
            file.strip_prefix(bundle_dir)
                .ok()
                .and_then(|path| path.to_str())
                .map(|path| path.replace('\\', "/"))
        })
        .collect()
}

fn write_text(path: &Path, content: &str, files: &mut Vec<PathBuf>) -> Option<usize> {
    fs::write(path, content.as_bytes()).ok()?;
    files.push(path.to_path_buf());
    Some(content.len())
}

fn write_json<T: Serialize + ?Sized>(
    path: &Path,
    value: &T,
    files: &mut Vec<PathBuf>,
) -> Option<usize> {
    let content = serde_json::to_string_pretty(value).ok()?;
    write_text(path, &content, files)
}

fn build_brief_markdown(
    user_message: &str,
    llm_brief: Option<&str>,
    relevant_files: &[RelevantFile],
) -> String {
    let mut out = String::new();
    out.push_str("# Task Brief\n\n");
    out.push_str("## User Request\n\n");
    out.push_str(user_message.trim());
    out.push_str("\n\n");
    if let Some(brief) = llm_brief.filter(|value| !value.trim().is_empty()) {
        out.push_str("## LLM Brief\n\n");
        out.push_str(brief.trim());
        out.push_str("\n\n");
    }
    if !relevant_files.is_empty() {
        out.push_str("## Likely Relevant Files\n\n");
        for file in relevant_files {
            out.push_str(&format!(
                "- `{}` score={} role={} lines={}\n",
                file.path, file.score, file.role, file.lines
            ));
        }
    }
    out
}

fn build_agent_prompt(user_message: &str) -> String {
    format!(
        r#"You are the implementation agent working inside a real repository.

<role>
A local deterministic context compiler prepared this task bundle.
Use it as a navigation aid, not as ground truth.
Before editing, verify important facts by reading the actual files.
</role>

<user_request>
{}
</user_request>

<context_files>
- brief.md
- task_context_pack.md
- context_pack.md
- .ai/context/current-task.md and .ai/context/current-task.json
- context_budget.md / context_budget.json
- project_manifests.md / project_manifests.json
- directory_summaries.md / directory_summaries.json / directories.jsonl
- repo_snapshot.json
- repo_context_index.json
- repo_map.md / summaries.md / symbols.jsonl / symbol_index.jsonl / symbol_edges.jsonl / symbol_lookup.json / symbol_index.sqlite / edges.tsv / chunks.jsonl / tests.jsonl / lsp_locations.jsonl / semantic_facts.jsonl when Rust analysis ran
- relevant_files.json
- validation_plan.json
- validation.md
</context_files>

<important_rules>
- Do not rely solely on summaries.
- Read the actual source files before editing.
- Prefer minimal, high-confidence changes.
- Preserve public APIs unless the task requires changing them.
- Add or update tests when behavior changes.
- Do not touch secrets or production configuration.
</important_rules>

<recommended_workflow>
1. Read the task brief.
2. Inspect the listed files and symbols.
3. Make a short plan.
4. Edit code.
5. Run formatting and validation commands from validation_plan.json.
6. Review git diff.
7. Summarize changes and remaining risks.
</recommended_workflow>
"#,
        user_message.trim()
    )
}

fn clamp_artifact(pack: &str, max_bytes: usize) -> String {
    if pack.len() <= max_bytes {
        return pack.to_string();
    }
    let mut out = String::new();
    for ch in pack.chars() {
        if out.len() + ch.len_utf8() > max_bytes {
            break;
        }
        out.push(ch);
    }
    out.push_str(
        "\n\n<!-- context pack artifact truncated by ELON_CONTEXT_COMPILER_ARTIFACT_MAX_BYTES -->",
    );
    out
}

fn safe_component(value: &str) -> String {
    let mut out = value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                Some(ch)
            } else {
                None
            }
        })
        .take(64)
        .collect::<String>();
    if out.is_empty() {
        out.push_str("unknown");
    }
    out
}


#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;
