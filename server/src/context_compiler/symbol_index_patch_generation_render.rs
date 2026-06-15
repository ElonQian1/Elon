use super::symbol_index_patch_generation_types::{
    PatchApplyReadiness, PatchDiffContract, PatchGenerationStep, SymbolPatchGeneration,
};

pub(crate) fn render_patch_generation(generation: &SymbolPatchGeneration) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<patch_generation mode=\"{}\" ready=\"{}\" steps=\"{}\" allowedFiles=\"{}\" inspectOnlyFiles=\"{}\">\n",
        generation.mode.as_str(),
        generation.ready_to_generate,
        generation.edit_sequence.len(),
        generation.diff_contract.allowed_files.len(),
        generation.diff_contract.inspect_only_files.len()
    ));
    out.push_str("# Patch Generation Contract\n\n");
    render_contract(&mut out, &generation.diff_contract);
    render_apply_readiness(&mut out, &generation.apply_readiness);
    render_steps(&mut out, &generation.edit_sequence);
    render_blocked_reasons(&mut out, &generation.blocked_reasons);
    render_prompt(&mut out, &generation.prompt);
    out.push_str("</patch_generation>\n");
    out
}

fn render_contract(out: &mut String, contract: &PatchDiffContract) {
    out.push_str("## Diff Contract\n");
    out.push_str(&format!(
        "- output_format: `{}`\n",
        xml_escape(&contract.output_format)
    ));
    out.push_str(&format!(
        "- apply_strategy: {}\n",
        xml_escape(&contract.apply_strategy)
    ));
    render_list(out, "allowed_files", &contract.allowed_files);
    render_list(out, "inspect_only_files", &contract.inspect_only_files);
    render_list(out, "forbidden_patterns", &contract.forbidden_patterns);
    render_list(out, "required_tests", &contract.required_tests);
    render_list(
        out,
        "verification_commands",
        &contract.verification_commands,
    );
    render_list(out, "safety_checks", &contract.safety_checks);
    out.push('\n');
}

fn render_apply_readiness(out: &mut String, readiness: &PatchApplyReadiness) {
    out.push_str("## Apply Readiness\n");
    out.push_str(&format!("- level: `{}`\n", readiness.level.as_str()));
    out.push_str(&format!(
        "- apply_check_status: `{}`\n",
        xml_escape(&readiness.apply_check_status)
    ));
    out.push_str(&format!(
        "- can_run_apply_check: {}\n",
        readiness.can_run_apply_check
    ));
    out.push_str(&format!(
        "- requires_generated_diff: {}\n",
        readiness.requires_generated_diff
    ));
    out.push_str(&format!(
        "- risk_level: `{}`\n",
        xml_escape(&readiness.risk_level)
    ));
    out.push_str(&format!(
        "- rollback_strategy: {}\n",
        xml_escape(&readiness.rollback_strategy)
    ));
    render_list(out, "source_requirements", &readiness.source_requirements);
    render_list(out, "pre_apply_checks", &readiness.pre_apply_checks);
    render_list(out, "post_apply_checks", &readiness.post_apply_checks);
    render_list(out, "notes", &readiness.notes);
    out.push('\n');
}

fn render_steps(out: &mut String, steps: &[PatchGenerationStep]) {
    out.push_str("## Edit Sequence\n");
    if steps.is_empty() {
        out.push_str("- none\n\n");
        return;
    }
    for step in steps.iter().take(8) {
        out.push_str(&format!(
            "{}. {}:{} `{}` edit={}\n",
            step.order,
            xml_escape(&step.file_path),
            step.start_line.unwrap_or_default(),
            xml_escape(step.qualified_name.as_deref().unwrap_or("-")),
            step.edit_type.as_str()
        ));
        out.push_str(&format!("   Action: {}\n", xml_escape(&step.action)));
        for constraint in step.constraints.iter().take(3) {
            out.push_str(&format!("   Constraint: {}\n", xml_escape(constraint)));
        }
        for evidence in step.evidence.iter().take(2) {
            out.push_str(&format!("   Evidence: {}\n", xml_escape(evidence)));
        }
    }
    out.push('\n');
}

fn render_blocked_reasons(out: &mut String, reasons: &[String]) {
    out.push_str("## Blocked Reasons\n");
    if reasons.is_empty() {
        out.push_str("- none\n\n");
        return;
    }
    for reason in reasons {
        out.push_str(&format!("- {}\n", xml_escape(reason)));
    }
    out.push('\n');
}

fn render_prompt(out: &mut String, prompt: &str) {
    out.push_str("## Generation Prompt\n");
    out.push_str("```text\n");
    out.push_str(&xml_escape(prompt));
    out.push_str("\n```\n\n");
}

fn render_list(out: &mut String, title: &str, values: &[String]) {
    out.push_str(&format!("- {title}:"));
    if values.is_empty() {
        out.push_str(" none\n");
        return;
    }
    out.push('\n');
    for value in values.iter().take(8) {
        out.push_str(&format!("  - {}\n", xml_escape(value)));
    }
    if values.len() > 8 {
        out.push_str(&format!("  - plus {} more\n", values.len() - 8));
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
