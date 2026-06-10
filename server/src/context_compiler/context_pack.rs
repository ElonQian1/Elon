use super::{
    config::ContextCompilerConfig,
    model::{RepoContextIndex, RustAnalyzerReport, RustSymbol, SymbolGraphSummary},
    relevance::RelevantFile,
    repo_snapshot::RepoSnapshot,
    rust_project::RustProjectSummary,
};

pub(crate) fn build_context_pack(
    config: &ContextCompilerConfig,
    user_message: &str,
    snapshot: &RepoSnapshot,
    rust_project: Option<&RustProjectSummary>,
    repo_index: Option<&RepoContextIndex>,
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

    if let Some(rust) = rust_project {
        out.push_str("<rust_project>\n");
        out.push_str(&format!(
            "- workspace: {}\n- root_package: {}\n",
            rust.workspace,
            rust.root_package.as_deref().unwrap_or("none")
        ));
        if let Some(toolchain) = rust.toolchain.as_deref() {
            out.push_str(&format!("- toolchain: {toolchain}\n"));
        }
        if !rust.workspace_members.is_empty() {
            out.push_str("- workspace_members: ");
            out.push_str(&rust.workspace_members.join(", "));
            out.push('\n');
        }
        if !rust.manifests.is_empty() {
            out.push_str("- manifests:\n");
            for manifest in &rust.manifests {
                out.push_str(&format!(
                    "  - path={} package={} workspace={}\n",
                    manifest.path,
                    manifest.package_name.as_deref().unwrap_or("none"),
                    manifest.workspace
                ));
            }
        }
        out.push_str("</rust_project>\n\n");
    }

    if let Some(index) = repo_index {
        render_cargo_workspace(&mut out, index);
        render_repo_map(&mut out, &index.graph);
        render_symbol_graph(&mut out, &index.graph);
        render_rust_safety_context(&mut out, &index.rust.symbols);
        render_rust_analyzer(&mut out, &index.rust_analyzer);
    } else {
        out.push_str("<repo_map status=\"disabled\">\n");
        out.push_str("Rust repo map++ disabled by ELON_CONTEXT_COMPILER_RUST_ANALYSIS=false.\n");
        out.push_str("</repo_map>\n\n");
    }

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
        out.push_str("<retrieval_evidence>\n");
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
        out.push_str("</retrieval_evidence>\n\n");
    }

    out.push_str("<output_contract>\n");
    out.push_str("- 先用真实文件确认上下文包给出的 path/line/symbol，再修改。\n");
    out.push_str("- 优先编辑 ranked_files 与 retrieval_evidence 同时命中的模块。\n");
    out.push_str("- 涉及 Rust public API、trait impl、enum match、unsafe/await/Send/Sync/Drop 时同步检查调用方和测试。\n");
    out.push_str("- 修改 Rust 代码后优先对本次改动的 .rs 文件运行 rustfmt。\n");
    out.push_str("- 后端行为变化至少运行相关 cargo test 或 cargo check。\n");
    out.push_str("- Android/APK 变化先完成代码同步；只有用户明确要求发布时才运行发布脚本。\n");
    out.push_str("</output_contract>\n\n");
    out.push_str("</task_context_pack>");

    truncate_pack(out, config.max_pack_chars)
}

fn render_cargo_workspace(out: &mut String, index: &RepoContextIndex) {
    out.push_str("<cargo_workspace>\n");
    if let Some(manifest) = &index.cargo.manifest_path {
        out.push_str(&format!("- manifest: {}\n", markdown_escape(manifest)));
    }
    if let Some(root) = &index.cargo.workspace_root {
        out.push_str(&format!("- workspace_root: {}\n", markdown_escape(root)));
    }
    for package in index.cargo.packages.iter().take(12) {
        out.push_str(&format!(
            "- package {} v{} manifest={}\n",
            markdown_escape(&package.name),
            markdown_escape(&package.version),
            markdown_escape(&package.manifest_path)
        ));
        if !package.targets.is_empty() {
            out.push_str(&format!(
                "  targets: {}\n",
                markdown_escape(&package.targets.join(", "))
            ));
        }
        if !package.features.is_empty() {
            out.push_str(&format!(
                "  features: {}\n",
                markdown_escape(&package.features.join(", "))
            ));
        }
    }
    for warning in &index.cargo.warnings {
        out.push_str(&format!("- warning: {}\n", markdown_escape(warning)));
    }
    out.push_str("</cargo_workspace>\n\n");
}

fn render_repo_map(out: &mut String, graph: &SymbolGraphSummary) {
    out.push_str("<repo_map strategy=\"rust-native-aider-plus\">\n");
    for file in &graph.ranked_files {
        out.push_str(&format!(
            "<file path=\"{}\" score=\"{:.2}\" role=\"{}\" symbols=\"{}\">\n",
            xml_escape(&file.path),
            file.score,
            file.role,
            file.symbol_count
        ));
        if !file.top_symbols.is_empty() {
            out.push_str("top_symbols: ");
            out.push_str(&markdown_escape(&file.top_symbols.join(", ")));
            out.push('\n');
        }
        if !file.reasons.is_empty() {
            out.push_str("reason: ");
            out.push_str(&markdown_escape(&file.reasons.join("; ")));
            out.push('\n');
        }
        out.push_str("</file>\n");
    }
    for warning in &graph.warnings {
        out.push_str(&format!("- warning: {}\n", markdown_escape(warning)));
    }
    out.push_str("</repo_map>\n\n");
}

fn render_symbol_graph(out: &mut String, graph: &SymbolGraphSummary) {
    out.push_str("<symbol_graph>\n");
    out.push_str("<ranked_symbols>\n");
    for symbol in graph.ranked_symbols.iter().take(40) {
        out.push_str(&format!(
            "- {} {} {}:{}-{} score={:.2} id={}",
            symbol.kind.as_str(),
            markdown_escape(&symbol.name),
            markdown_escape(&symbol.path),
            symbol.line_start,
            symbol.line_end,
            symbol.score,
            markdown_escape(&symbol.id)
        ));
        if !symbol.reasons.is_empty() {
            out.push_str(" reason=");
            out.push_str(&markdown_escape(&symbol.reasons.join("; ")));
        }
        out.push('\n');
    }
    out.push_str("</ranked_symbols>\n");
    out.push_str("<relationships>\n");
    for relationship in graph.relationships.iter().take(50) {
        out.push_str(&format!(
            "- {} L{} -> {} [{}] at {} reason={}\n",
            markdown_escape(&relationship.from_path),
            relationship.line,
            markdown_escape(&relationship.to_symbol_name),
            relationship.kind.as_str(),
            markdown_escape(&relationship.to_path),
            markdown_escape(&relationship.reason)
        ));
    }
    out.push_str("</relationships>\n");
    out.push_str("</symbol_graph>\n\n");
}

fn render_rust_safety_context(out: &mut String, symbols: &[RustSymbol]) {
    let risky = symbols
        .iter()
        .filter(|symbol| !symbol.safety_notes.is_empty())
        .take(30)
        .collect::<Vec<_>>();
    if risky.is_empty() {
        return;
    }
    out.push_str("<rust_safety_context>\n");
    for symbol in risky {
        out.push_str(&format!(
            "- {} {} {}:{}-{} visibility={} notes={}\n",
            symbol.kind.as_str(),
            markdown_escape(&symbol.name),
            markdown_escape(&symbol.path),
            symbol.line_start,
            symbol.line_end,
            symbol.visibility.as_str(),
            markdown_escape(&symbol.safety_notes.join(", "))
        ));
    }
    out.push_str("</rust_safety_context>\n\n");
}

fn render_rust_analyzer(out: &mut String, report: &RustAnalyzerReport) {
    out.push_str("<rust_analyzer>\n");
    out.push_str(&format!("- available: {}\n", report.available));
    if let Some(version) = &report.version {
        out.push_str(&format!("- version: {}\n", markdown_escape(version)));
    }
    if !report.enhancement_targets.is_empty() {
        out.push_str(&format!(
            "- enhanced_files: {} / targets={}\n",
            report.files_enhanced,
            markdown_escape(&report.enhancement_targets.join(", "))
        ));
    }
    for symbol in report.symbols.iter().take(40) {
        out.push_str(&format!(
            "- {} {} {}:{}",
            markdown_escape(&symbol.kind),
            markdown_escape(&symbol.label),
            markdown_escape(&symbol.path),
            symbol.line
        ));
        if let Some(detail) = &symbol.detail {
            out.push_str(&format!(" detail={}", markdown_escape(detail)));
        }
        if let Some(parent) = &symbol.parent {
            out.push_str(&format!(" parent={}", markdown_escape(parent)));
        }
        out.push('\n');
    }
    for warning in &report.warnings {
        out.push_str(&format!("- warning: {}\n", markdown_escape(warning)));
    }
    out.push_str("</rust_analyzer>\n\n");
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
            rust_analysis_enabled: true,
            rust_analyzer_enabled: true,
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

        let pack = build_context_pack(
            &config,
            "实现 context compiler",
            &snapshot,
            None,
            None,
            &relevant,
            None,
        );

        assert!(pack.contains("只读预检产物"));
        assert!(pack.contains("server/src/context_compiler/mod.rs"));
        assert!(pack.contains("Cargo.toml"));
        assert!(pack.contains("<retrieval_evidence>"));
    }

    #[test]
    fn context_pack_includes_rust_project_summary() {
        let config = ContextCompilerConfig {
            enabled: true,
            mode: ContextCompilerMode::Inject,
            agent_name: "hunyuan".to_string(),
            llm_brief_enabled: false,
            rust_analysis_enabled: true,
            rust_analyzer_enabled: true,
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
            has_origin: true,
            top_level_entries: Vec::new(),
            instruction_docs: Vec::new(),
            manifests: vec!["Cargo.toml".to_string()],
            large_files: Vec::new(),
            source_file_count: 1,
        };
        let rust = RustProjectSummary {
            root_package: Some("elon-server".to_string()),
            workspace: true,
            workspace_members: vec!["server".to_string()],
            manifests: Vec::new(),
            toolchain: Some("stable".to_string()),
        };

        let pack = build_context_pack(&config, "任务", &snapshot, Some(&rust), None, &[], None);

        assert!(pack.contains("<rust_project>"));
        assert!(pack.contains("root_package: elon-server"));
    }
}
