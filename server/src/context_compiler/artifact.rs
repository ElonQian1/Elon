use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::Serialize;
use serde_json::json;

use super::{
    config::ContextCompilerConfig, model::RepoContextIndex, relevance::RelevantFile,
    repo_snapshot::RepoSnapshot, rust_project::RustProjectSummary, validation::ValidationPlan,
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
    if let Some(repo_index) = input.repo_index {
        bytes += write_json(
            &bundle_dir.join("repo_context_index.json"),
            repo_index,
            &mut files,
        )?;
    }
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

fn write_text(path: &Path, content: &str, files: &mut Vec<PathBuf>) -> Option<usize> {
    fs::write(path, content.as_bytes()).ok()?;
    files.push(path.to_path_buf());
    Some(content.len())
}

fn write_json<T: Serialize>(path: &Path, value: &T, files: &mut Vec<PathBuf>) -> Option<usize> {
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
- context_pack.md
- repo_snapshot.json
- repo_context_index.json
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
mod tests {
    use super::*;
    use crate::context_compiler::config::ContextCompilerMode;
    use crate::context_compiler::repo_snapshot::RepoSnapshot;
    use crate::context_compiler::validation::ValidationPlan;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn saves_pack_and_bundle_under_data_dir() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "elon_context_artifact_{}_{}",
            std::process::id(),
            nonce
        ));
        let config = ContextCompilerConfig {
            enabled: true,
            mode: ContextCompilerMode::Shadow,
            agent_name: "hunyuan".to_string(),
            llm_brief_enabled: false,
            rust_analysis_enabled: true,
            rust_analyzer_enabled: true,
            rust_analyzer_probe_enabled: false,
            rust_analyzer_probe_timeout_ms: 4_000,
            max_relevant_files: 4,
            max_rust_files: 40,
            max_symbols: 20,
            max_relationships: 20,
            max_rust_analyzer_files: 2,
            max_pack_chars: 20_000,
            save_pack_enabled: true,
            artifact_max_bytes: 100_000,
            rust_probe_enabled: true,
        };
        let snapshot = RepoSnapshot {
            git_head: Some("abc123".to_string()),
            git_branch: Some("main".to_string()),
            git_dirty: false,
            git_status_short: Vec::new(),
            has_origin: true,
            top_level_entries: Vec::new(),
            instruction_docs: Vec::new(),
            manifests: Vec::new(),
            large_files: Vec::new(),
            source_file_count: 0,
        };
        let validation = ValidationPlan {
            commands: Vec::new(),
            notes: Vec::new(),
        };

        let artifact = save_context_artifacts(ContextArtifactsInput {
            data_dir: &dir,
            config: &config,
            trace_id: Some("trace/1"),
            user_id: "user@example",
            user_message: "hello task",
            pack: "hello",
            llm_brief: None,
            snapshot: &snapshot,
            rust_project: None,
            repo_index: None,
            relevant_files: &[],
            validation_plan: &validation,
        })
        .unwrap();

        assert!(artifact.path.starts_with(&dir));
        assert_eq!(fs::read_to_string(&artifact.path).unwrap(), "hello");
        assert!(artifact.path.to_string_lossy().contains("trace1"));
        assert!(artifact.bundle_dir.starts_with(&dir));
        assert!(artifact.bundle_dir.join("repo_snapshot.json").is_file());
        assert!(artifact.bundle_dir.join("agent_prompt.md").is_file());
        assert!(artifact.files.len() >= 8);

        fs::remove_dir_all(dir).unwrap();
    }
}
