use super::{
    directory_summary::DirectorySummary,
    model::{
        ContextEvidence, RepoContextIndex, RustAnalyzerReport, RustSymbol, SymbolGraphSummary,
        TaskProfile,
    },
    project_manifests::ProjectManifestReport,
    relevance::RelevantFile,
    repo_snapshot::RepoSnapshot,
    rust_project::RustProjectSummary,
    validation::ValidationPlan,
};

pub(crate) fn render_task_profile(out: &mut String, task: &TaskProfile) {
    out.push_str("<task_understanding>\n");
    if !task.keywords.is_empty() {
        out.push_str(&format!(
            "- keywords: {}\n",
            markdown_escape(&task.keywords.join(", "))
        ));
    }
    if !task.likely_domains.is_empty() {
        out.push_str(&format!(
            "- likely_domains: {}\n",
            markdown_escape(&task.likely_domains.join(", "))
        ));
    }
    if !task.suspected_symbols.is_empty() {
        out.push_str(&format!(
            "- suspected_symbols: {}\n",
            markdown_escape(&task.suspected_symbols.join(", "))
        ));
    }
    if !task.suspected_files.is_empty() {
        out.push_str(&format!(
            "- suspected_files: {}\n",
            markdown_escape(&task.suspected_files.join(", "))
        ));
    }
    for hint in &task.action_hints {
        out.push_str(&format!("- action_hint: {}\n", markdown_escape(hint)));
    }
    out.push_str("</task_understanding>\n\n");
}

pub(crate) fn render_repo_snapshot(out: &mut String, snapshot: &RepoSnapshot) {
    out.push_str("<repo_snapshot>\n");
    out.push_str(&format!(
        "- git_head: {}\n- git_branch: {}\n- has_origin: {}\n- source_file_count: {}\n",
        snapshot.git_head.as_deref().unwrap_or("unknown"),
        snapshot.git_branch.as_deref().unwrap_or("unknown"),
        snapshot.has_origin,
        snapshot.source_file_count
    ));
    out.push_str(&format!("- git_dirty: {}\n", snapshot.git_dirty));
    if !snapshot.git_status_short.is_empty() {
        out.push_str("- git_status_short:\n");
        for line in &snapshot.git_status_short {
            out.push_str(&format!("  - {}\n", markdown_escape(line)));
        }
    }
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
}

pub(crate) fn render_project_manifests(out: &mut String, report: Option<&ProjectManifestReport>) {
    let Some(report) = report else {
        return;
    };
    if report.readmes.is_empty() && report.manifests.is_empty() {
        return;
    }
    out.push_str("<project_manifests>\n");
    for readme in report.readmes.iter().take(8) {
        out.push_str(&format!(
            "- readme {} title={}\n",
            markdown_escape(&readme.path),
            markdown_escape(readme.title.as_deref().unwrap_or("none"))
        ));
        if !readme.headings.is_empty() {
            out.push_str(&format!(
                "  headings: {}\n",
                markdown_escape(&readme.headings.join(", "))
            ));
        }
        if let Some(preview) = readme.preview.as_deref() {
            out.push_str(&format!("  preview: {}\n", markdown_escape(preview)));
        }
    }
    for manifest in report.manifests.iter().take(16) {
        out.push_str(&format!(
            "- manifest {} kind={} name={} version={}\n",
            markdown_escape(&manifest.path),
            manifest.kind,
            markdown_escape(manifest.name.as_deref().unwrap_or("unknown")),
            markdown_escape(manifest.version.as_deref().unwrap_or("unknown"))
        ));
        if let Some(description) = manifest.description.as_deref() {
            out.push_str(&format!(
                "  description: {}\n",
                markdown_escape(description)
            ));
        }
        push_compact_list(out, "scripts", &manifest.scripts);
        push_compact_list(out, "dependencies", &manifest.dependencies);
        push_compact_list(out, "features", &manifest.features);
    }
    for warning in &report.warnings {
        out.push_str(&format!("- warning: {}\n", markdown_escape(warning)));
    }
    out.push_str("</project_manifests>\n\n");
}

pub(crate) fn render_directory_summaries(out: &mut String, summaries: &[DirectorySummary]) {
    if summaries.is_empty() {
        return;
    }
    out.push_str("<directory_summaries>\n");
    for summary in summaries.iter().take(30) {
        out.push_str(&format!(
            "- {} direct_files={} subtree_source_files={} subtree_lines={}\n",
            markdown_escape(&summary.path),
            summary.direct_files,
            summary.subtree_source_files,
            summary.subtree_lines
        ));
        if !summary.role_counts.is_empty() {
            let roles = summary
                .role_counts
                .iter()
                .take(6)
                .map(|item| format!("{}:{}", item.role, item.files))
                .collect::<Vec<_>>();
            out.push_str(&format!(
                "  roles: {}\n",
                markdown_escape(&roles.join(", "))
            ));
        }
        push_compact_list(out, "key_files", &summary.key_files);
        push_compact_list(out, "child_directories", &summary.child_directories);
    }
    out.push_str("</directory_summaries>\n\n");
}

pub(crate) fn render_rust_project(out: &mut String, rust_project: Option<&RustProjectSummary>) {
    let Some(rust) = rust_project else {
        return;
    };
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

pub(crate) fn render_repo_index(out: &mut String, index: Option<&RepoContextIndex>) {
    if let Some(index) = index {
        render_cargo_workspace(out, index);
        render_repo_map(out, &index.graph);
        render_symbol_graph(out, &index.graph);
        render_rust_safety_context(out, &index.rust.symbols);
        render_rust_analyzer(out, &index.rust_analyzer);
    } else {
        out.push_str("<repo_map status=\"disabled\">\n");
        out.push_str("Rust repo map++ disabled by ELON_CONTEXT_COMPILER_RUST_ANALYSIS=false.\n");
        out.push_str("</repo_map>\n\n");
    }
}

pub(crate) fn render_source_size_risks(out: &mut String, snapshot: &RepoSnapshot) {
    if snapshot.large_files.is_empty() {
        return;
    }
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

pub(crate) fn render_context_evidence(out: &mut String, evidence: Option<&ContextEvidence>) {
    let Some(evidence) = evidence else {
        return;
    };
    render_relevant_files(out, evidence);
    render_neighbor_summaries(out, evidence);
    render_invariants(out, evidence);
    render_tests(out, evidence);
    render_build_commands(out, evidence);
    render_missing_context_policy(out, evidence);
    render_recommended_actions(out, evidence);
}

pub(crate) fn render_retrieval_evidence(out: &mut String, relevant_files: &[RelevantFile]) {
    if relevant_files.is_empty() {
        return;
    }
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

pub(crate) fn render_validation_guidance(out: &mut String, validation_plan: &ValidationPlan) {
    out.push_str("<validation_guidance>\n");
    for command in &validation_plan.commands {
        let required = if command.required {
            "required"
        } else {
            "recommended"
        };
        out.push_str(&format!(
            "- `{}` ({}) - {}\n",
            markdown_escape(&command.command),
            required,
            markdown_escape(&command.reason)
        ));
    }
    for note in &validation_plan.notes {
        out.push_str(&format!("- note: {}\n", markdown_escape(note)));
    }
    out.push_str("</validation_guidance>\n\n");
}

pub(crate) fn render_output_contract(out: &mut String) {
    out.push_str("<output_contract>\n");
    out.push_str("- 先用真实文件确认上下文包给出的 path/line/symbol/hash，再修改。\n");
    out.push_str("- 优先编辑 relevant_files 中 role=edit-target 的片段；没有源码片段的文件先读取真实文件。\n");
    out.push_str("- 涉及 Rust public API、trait impl、enum match、unsafe/await/Send/Sync/Drop 时同步检查调用方和测试。\n");
    out.push_str("- 如果上下文不足，先列 missing_context，不要猜测未提供源码的行为。\n");
    out.push_str("- 修改 Rust 代码后优先对本次改动的 .rs 文件运行 rustfmt。\n");
    out.push_str("- 后端行为变化至少运行相关 cargo test 或 cargo check。\n");
    out.push_str("- Android/APK 变化先完成代码同步；只有用户明确要求发布时才运行发布脚本。\n");
    out.push_str("</output_contract>\n\n");
    out.push_str("<final_instructions>\n");
    out.push_str(
        "请基于上面的 context 完成任务；所有项目现状判断尽量引用 path:line 或 symbol id。\n",
    );
    out.push_str("</final_instructions>\n\n");
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
        if !package.target_paths.is_empty() {
            out.push_str(&format!(
                "  target_paths: {}\n",
                markdown_escape(&package.target_paths.join(", "))
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
    out.push_str("<symbol_graph>\n<ranked_symbols>\n");
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
    out.push_str("</ranked_symbols>\n<relationships>\n");
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
    out.push_str("</relationships>\n</symbol_graph>\n\n");
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

fn render_relevant_files(out: &mut String, evidence: &ContextEvidence) {
    if evidence.snippets.is_empty() {
        return;
    }
    out.push_str("<relevant_files>\n");
    for snippet in &evidence.snippets {
        out.push_str(&format!(
            "<file id=\"{}\" path=\"{}\" role=\"{}\" symbols=\"{}\" lines=\"{}-{}\" sha256=\"{}\">\n",
            xml_escape(&snippet.id),
            xml_escape(&snippet.path),
            snippet.role,
            xml_escape(&snippet.symbols.join(",")),
            snippet.line_start,
            snippet.line_end,
            xml_escape(&snippet.sha256)
        ));
        if !snippet.reason.is_empty() {
            out.push_str(&format!("reason: {}\n", markdown_escape(&snippet.reason)));
        }
        out.push_str("```rust\n");
        out.push_str(&snippet.content);
        out.push_str("\n```\n</file>\n");
    }
    out.push_str("</relevant_files>\n\n");
}

fn render_neighbor_summaries(out: &mut String, evidence: &ContextEvidence) {
    if evidence.neighbor_summaries.is_empty() {
        return;
    }
    out.push_str("<neighbor_summaries>\n");
    for neighbor in &evidence.neighbor_summaries {
        out.push_str(&format!(
            "- {} relationship={} symbols={} reason={} needed_if={}\n",
            markdown_escape(&neighbor.path),
            neighbor.relationship.as_str(),
            markdown_escape(&neighbor.symbols.join(",")),
            markdown_escape(&neighbor.reason),
            markdown_escape(&neighbor.needed_if)
        ));
    }
    out.push_str("</neighbor_summaries>\n\n");
}

fn render_invariants(out: &mut String, evidence: &ContextEvidence) {
    render_facts(out, "invariants", &evidence.invariants);
    render_facts(out, "public_api_contracts", &evidence.public_api_contracts);
    render_facts(out, "unsafe_boundaries", &evidence.unsafe_boundaries);
    if evidence.feature_flags.is_empty() {
        return;
    }
    out.push_str("<feature_flags>\n");
    for flag in &evidence.feature_flags {
        out.push_str(&format!(
            "- package={} feature={} manifest={}\n",
            markdown_escape(&flag.package),
            markdown_escape(&flag.feature),
            markdown_escape(&flag.manifest_path)
        ));
    }
    out.push_str("</feature_flags>\n\n");
}

fn render_tests(out: &mut String, evidence: &ContextEvidence) {
    if evidence.test_targets.is_empty() {
        return;
    }
    out.push_str("<tests>\n");
    for test in &evidence.test_targets {
        out.push_str(&format!(
            "- {} reason={}\n",
            markdown_escape(&test.path),
            markdown_escape(&test.reason)
        ));
    }
    out.push_str("</tests>\n\n");
}

fn render_build_commands(out: &mut String, evidence: &ContextEvidence) {
    if evidence.build_commands.is_empty() {
        return;
    }
    out.push_str("<build_commands>\n");
    for command in &evidence.build_commands {
        out.push_str(&format!(
            "- `{}` reason={}\n",
            markdown_escape(&command.command),
            markdown_escape(&command.reason)
        ));
    }
    out.push_str("</build_commands>\n\n");
}

fn render_missing_context_policy(out: &mut String, evidence: &ContextEvidence) {
    out.push_str("<missing_context_policy>\n");
    out.push_str("- 不要修改没有真实源码证据的文件；先读取文件或列入 missing_context。\n");
    for item in &evidence.missing_context {
        out.push_str(&format!("- {}\n", markdown_escape(item)));
    }
    out.push_str("</missing_context_policy>\n\n");
}

fn render_recommended_actions(out: &mut String, evidence: &ContextEvidence) {
    if evidence.recommended_actions.is_empty() {
        return;
    }
    out.push_str("<recommended_agent_actions>\n");
    for action in &evidence.recommended_actions {
        out.push_str(&format!("- {}\n", markdown_escape(action)));
    }
    out.push_str("</recommended_agent_actions>\n\n");
}

fn render_facts(out: &mut String, tag: &str, facts: &[super::model::ContextFact]) {
    if facts.is_empty() {
        return;
    }
    out.push_str(&format!("<{tag}>\n"));
    for fact in facts {
        out.push_str(&format!(
            "- {} {}:{}-{} detail={}\n",
            markdown_escape(&fact.subject),
            markdown_escape(&fact.path),
            fact.line_start,
            fact.line_end,
            markdown_escape(&fact.detail)
        ));
    }
    out.push_str(&format!("</{tag}>\n\n"));
}

fn push_compact_list(out: &mut String, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    out.push_str(&format!(
        "  {}: {}\n",
        label,
        markdown_escape(
            &values
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    ));
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
