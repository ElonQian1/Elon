use super::{config::ContextCompilerConfig, relevance::RelevantFile, repo_snapshot::RepoSnapshot};

pub(crate) fn build_context_pack(
    config: &ContextCompilerConfig,
    user_message: &str,
    snapshot: &RepoSnapshot,
    relevant_files: &[RelevantFile],
    llm_brief: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str("<task_context_pack version=\"0.1\" source=\"elon-context-compiler\">\n\n");
    out.push_str("<instructions>\n");
    out.push_str("这份上下文包是只读预检产物，只能作为导航证据，不是真实代码的替代品。\n");
    out.push_str("执行修改前必须读取真实文件确认；如果上下文不足，请继续用当前 CLI 工具查询。\n");
    out.push_str("</instructions>\n\n");
    out.push_str("<task>\n");
    out.push_str("用户原始请求：\n");
    out.push_str(user_message.trim());
    out.push_str("\n</task>\n\n");

    if let Some(brief) = llm_brief.filter(|value| !value.trim().is_empty()) {
        out.push_str("<llm_brief model_role=\"context-compressor\">\n");
        out.push_str(brief.trim());
        out.push_str("\n</llm_brief>\n\n");
    }

    out.push_str("<repo_snapshot>\n");
    out.push_str(&format!(
        "- git_head: {}\n- git_branch: {}\n- has_origin: {}\n- source_file_count: {}\n",
        snapshot.git_head.as_deref().unwrap_or("unknown"),
        snapshot.git_branch.as_deref().unwrap_or("unknown"),
        snapshot.has_origin,
        snapshot.source_file_count
    ));
    if !snapshot.top_level_entries.is_empty() {
        out.push_str("- top_level_entries: ");
        out.push_str(&snapshot.top_level_entries.join(", "));
        out.push('\n');
    }
    if !snapshot.manifests.is_empty() {
        out.push_str("- manifests: ");
        out.push_str(&snapshot.manifests.join(", "));
        out.push('\n');
    }
    if !snapshot.instruction_docs.is_empty() {
        out.push_str("- instruction_docs: ");
        out.push_str(&snapshot.instruction_docs.join(", "));
        out.push('\n');
    }
    out.push_str("</repo_snapshot>\n\n");

    if !snapshot.large_files.is_empty() {
        out.push_str("<source_size_risks>\n");
        out.push_str("新增逻辑优先放入独立 focused module，避免扩大红区文件。\n");
        for file in &snapshot.large_files {
            out.push_str(&format!(
                "- {}: {} lines, role={}\n",
                file.path, file.lines, file.role
            ));
        }
        out.push_str("</source_size_risks>\n\n");
    }

    if !relevant_files.is_empty() {
        out.push_str("<relevant_files>\n");
        for file in relevant_files {
            out.push_str(&format!(
                "<file path=\"{}\" score=\"{}\" lines=\"{}\" role=\"{}\">\n",
                xml_escape(&file.path),
                file.score,
                file.lines,
                file.role
            ));
            if !file.reasons.is_empty() {
                out.push_str("reason: ");
                out.push_str(&file.reasons.join("; "));
                out.push('\n');
            }
            for line_match in &file.matches {
                out.push_str(&format!(
                    "- L{}: {}\n",
                    line_match.line,
                    markdown_escape(&line_match.text)
                ));
            }
            out.push_str("</file>\n");
        }
        out.push_str("</relevant_files>\n\n");
    }

    out.push_str("<validation_guidance>\n");
    out.push_str("- 修改 Rust 代码后优先对本次改动的 .rs 文件运行 rustfmt。\n");
    out.push_str("- 后端行为变化至少运行相关 cargo test 或 cargo check。\n");
    out.push_str("- Android/APK 变化先完成代码同步；只有用户明确要求发布时才运行发布脚本。\n");
    out.push_str("</validation_guidance>\n\n");
    out.push_str("</task_context_pack>");

    truncate_pack(out, config.max_pack_chars)
}

fn truncate_pack(mut pack: String, max_chars: usize) -> String {
    if pack.chars().count() <= max_chars {
        return pack;
    }
    let mut truncated = pack.chars().take(max_chars).collect::<String>();
    truncated.push_str("\n\n<!-- context pack truncated by ELON_CONTEXT_COMPILER_MAX_CHARS -->");
    pack.clear();
    truncated
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn markdown_escape(value: &str) -> String {
    value.replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_compiler::config::ContextCompilerMode;

    #[test]
    fn context_pack_contains_navigation_warning_and_relevant_file() {
        let config = ContextCompilerConfig {
            enabled: true,
            mode: ContextCompilerMode::Inject,
            agent_name: "hunyuan".to_string(),
            llm_brief_enabled: false,
            max_relevant_files: 4,
            max_pack_chars: 20_000,
        };
        let snapshot = RepoSnapshot {
            git_head: Some("abc123".to_string()),
            git_branch: Some("main".to_string()),
            has_origin: true,
            top_level_entries: vec!["server/".to_string()],
            instruction_docs: vec!["AGENTS.md".to_string()],
            manifests: vec!["Cargo.toml".to_string()],
            large_files: Vec::new(),
            source_file_count: 1,
        };
        let relevant = vec![RelevantFile {
            path: "server/src/context_compiler/mod.rs".to_string(),
            score: 9,
            lines: 120,
            role: "source",
            reasons: vec!["path contains `context`".to_string()],
            matches: Vec::new(),
        }];

        let pack = build_context_pack(&config, "实现 context compiler", &snapshot, &relevant, None);

        assert!(pack.contains("只读预检产物"));
        assert!(pack.contains("server/src/context_compiler/mod.rs"));
        assert!(pack.contains("Cargo.toml"));
    }
}
