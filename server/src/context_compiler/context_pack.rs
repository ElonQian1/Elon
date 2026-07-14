use super::{
    config::ContextCompilerConfig, context_pack_render, context_quality_render,
    directory_summary::DirectorySummary, impact_render, model::RepoContextIndex,
    project_manifests::ProjectManifestReport, relevance::RelevantFile, repo_map_tags_render,
    repo_snapshot::RepoSnapshot, rust_analyzer_lsp_render, rust_analyzer_probe_render,
    rust_project::RustProjectSummary, semantic_query_plan_render, validation::ValidationPlan,
};

pub(crate) fn build_context_pack(
    config: &ContextCompilerConfig,
    user_message: &str,
    snapshot: &RepoSnapshot,
    rust_project: Option<&RustProjectSummary>,
    project_manifests: Option<&ProjectManifestReport>,
    directory_summaries: &[DirectorySummary],
    repo_index: Option<&RepoContextIndex>,
    relevant_files: &[RelevantFile],
    validation_plan: &ValidationPlan,
    llm_brief: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str("<task_context_pack version=\"0.2\" source=\"elon-context-compiler\">\n\n");
    out.push_str("<instructions>\n");
    out.push_str("这份上下文包是只读预检产物，只能作为导航证据，不是真实代码的替代品。\n");
    out.push_str("执行修改前必须读取真实文件确认；如果上下文不足，请继续用当前 CLI 工具查询。\n");
    out.push_str("</instructions>\n\n");
    out.push_str("<task>\n");
    out.push_str("用户原始请求：\n");
    out.push_str(user_message.trim());
    out.push_str("\n</task>\n\n");

    if let Some(index) = repo_index {
        context_pack_render::render_task_profile(&mut out, &index.task);
    }

    if let Some(brief) = llm_brief.filter(|value| !value.trim().is_empty()) {
        out.push_str("<llm_brief model_role=\"context-compressor\">\n");
        out.push_str(brief.trim());
        out.push_str("\n</llm_brief>\n\n");
    }

    context_pack_render::render_repo_snapshot(&mut out, snapshot);
    context_pack_render::render_project_manifests(&mut out, project_manifests);
    context_pack_render::render_directory_summaries(&mut out, directory_summaries);
    context_pack_render::render_rust_project(&mut out, rust_project);
    context_pack_render::render_repo_index(&mut out, repo_index);
    repo_map_tags_render::render_repo_map_tags(
        &mut out,
        repo_index.map(|index| &index.graph.repo_map_tags),
    );
    rust_analyzer_probe_render::render_rust_analyzer_probes(
        &mut out,
        repo_index.map(|index| &index.rust_analyzer.probes),
    );
    semantic_query_plan_render::render_semantic_query_plan(
        &mut out,
        repo_index.map(|index| &index.semantic_plan),
    );
    rust_analyzer_lsp_render::render_rust_analyzer_lsp(
        &mut out,
        repo_index.map(|index| &index.rust_analyzer.lsp),
    );
    context_quality_render::render_context_quality(
        &mut out,
        repo_index.map(|index| &index.quality),
    );
    context_pack_render::render_source_size_risks(&mut out, snapshot);
    impact_render::render_impact_analysis(&mut out, repo_index.map(|index| &index.impact));
    context_pack_render::render_context_evidence(&mut out, repo_index.map(|index| &index.evidence));
    context_pack_render::render_retrieval_evidence(&mut out, relevant_files);
    context_pack_render::render_validation_guidance(&mut out, validation_plan);
    context_pack_render::render_output_contract(&mut out);
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

#[cfg(test)]
#[path = "context_pack_tests.rs"]
mod tests;
