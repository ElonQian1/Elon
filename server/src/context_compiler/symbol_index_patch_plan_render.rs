use super::symbol_index_patch_plan_types::{PatchEditTarget, ProposedPatchChange, SymbolPatchPlan};

pub(crate) fn render_patch_plan(plan: &SymbolPatchPlan) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<patch_plan intent=\"{}\" kind=\"{}\" patchRequired=\"{}\" mustEdit=\"{}\" shouldInspect=\"{}\" maybeEdit=\"{}\">\n",
        plan.intent.as_str(),
        xml_escape(&plan.plan_kind),
        plan.patch_required,
        plan.must_edit.len(),
        plan.should_inspect.len(),
        plan.maybe_edit.len()
    ));
    out.push_str("# Patch Plan\n\n");
    render_targets(&mut out, "Must Edit", &plan.must_edit);
    render_targets(&mut out, "Should Inspect", &plan.should_inspect);
    render_test_plan(&mut out, plan);
    if plan.patch_required {
        render_targets(&mut out, "Maybe Edit", &plan.maybe_edit);
        render_proposed_changes(&mut out, &plan.proposed_changes);
    }
    render_list(&mut out, "Risk Notes", &plan.risk_notes);
    render_list(&mut out, "Open Questions", &plan.open_questions);
    if plan.patch_required {
        render_trace(&mut out, plan);
    }
    out.push_str("</patch_plan>\n");
    out
}

fn render_targets(out: &mut String, title: &str, targets: &[PatchEditTarget]) {
    out.push_str(&format!("## {title}\n"));
    if targets.is_empty() {
        out.push_str("- none\n\n");
        return;
    }
    for (index, target) in targets.iter().take(4).enumerate() {
        out.push_str(&format!(
            "{}. {}:{} `{}`\n",
            index + 1,
            xml_escape(&target.file_path),
            target.start_line.unwrap_or_default(),
            xml_escape(target.qualified_name.as_deref().unwrap_or("-"))
        ));
        out.push_str(&format!(
            "   Edit: {} | priority={} | sourceRank={} | rerank={} | compression={}\n",
            target.edit_type.as_str(),
            target.priority.as_str(),
            target.source_rank,
            target.source_decision.as_str(),
            target.compression_level.as_str()
        ));
        out.push_str(&format!(
            "   Reason: {}\n",
            xml_escape(&truncate(&target.reason, 220))
        ));
    }
    if targets.len() > 4 {
        out.push_str(&format!("- plus {} more targets\n", targets.len() - 4));
    }
    out.push('\n');
}

fn render_proposed_changes(out: &mut String, changes: &[ProposedPatchChange]) {
    out.push_str("## Proposed Changes\n");
    if changes.is_empty() {
        out.push_str("- none\n\n");
        return;
    }
    for (index, change) in changes.iter().take(4).enumerate() {
        out.push_str(&format!(
            "{}. {} `{}`\n",
            index + 1,
            xml_escape(&change.target_file_path),
            xml_escape(change.target_symbol.as_deref().unwrap_or("-"))
        ));
        out.push_str(&format!("   Edit type: {}\n", change.edit_type.as_str()));
        if let Some(current) = change.current_behavior.as_deref() {
            out.push_str(&format!("   Current: {}\n", xml_escape(current)));
        }
        out.push_str(&format!(
            "   Desired: {}\n",
            xml_escape(&truncate(&change.desired_behavior, 180))
        ));
        for instruction in change.instructions.iter().take(2) {
            out.push_str(&format!("   - {}\n", xml_escape(instruction)));
        }
        for constraint in change.constraints.iter().take(1) {
            out.push_str(&format!("   Constraint: {}\n", xml_escape(constraint)));
        }
    }
    if changes.len() > 4 {
        out.push_str(&format!(
            "- plus {} more proposed changes\n",
            changes.len() - 4
        ));
    }
    out.push('\n');
}

fn render_test_plan(out: &mut String, plan: &SymbolPatchPlan) {
    out.push_str("## Test Plan\n");
    if plan.test_plan.commands.is_empty() && plan.test_plan.target_tests.is_empty() {
        out.push_str("- no targeted test discovered\n\n");
        return;
    }
    for command in &plan.test_plan.commands {
        out.push_str(&format!("- command: `{}`\n", xml_escape(command)));
    }
    for target in &plan.test_plan.target_tests {
        out.push_str(&format!("- target: `{}`\n", xml_escape(target)));
    }
    for expected in &plan.test_plan.expected_behavior {
        out.push_str(&format!("- expected: {}\n", xml_escape(expected)));
    }
    out.push('\n');
}

fn render_list(out: &mut String, title: &str, values: &[String]) {
    out.push_str(&format!("## {title}\n"));
    if values.is_empty() {
        out.push_str("- none\n\n");
        return;
    }
    for value in values {
        out.push_str(&format!("- {}\n", xml_escape(value)));
    }
    out.push('\n');
}

fn render_trace(out: &mut String, plan: &SymbolPatchPlan) {
    out.push_str("## Planning Trace\n");
    for trace in plan.trace.iter().take(8) {
        out.push_str(&format!(
            "- #{} {} `{}` decision={}\n",
            trace.rank,
            xml_escape(&trace.file_path),
            xml_escape(&trace.label),
            trace.decision.as_str()
        ));
        if !trace.reasons.is_empty() {
            out.push_str(&format!(
                "  reason: {}\n",
                xml_escape(&trace.reasons.join("; "))
            ));
        }
    }
    out.push('\n');
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out = value.chars().take(max_chars).collect::<String>();
    out.push_str("...");
    out
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
