use super::classify_project_document;

#[test]
fn path_sets_authority_ceiling_and_default_retrieval() {
    let policy = classify_project_document(".github/copilot-instructions.md", "# Rules\n", 8);
    assert_eq!(policy.authority, "repository_policy");
    assert!(policy.default_retrieval);

    let instruction = classify_project_document(
        ".github/instructions/backend.instructions.md",
        "# Backend\n",
        10,
    );
    assert_eq!(instruction.role, "instruction");
    assert!(!instruction.default_retrieval);

    let project_guide = classify_project_document("AI_PROJECT.md", "# Project\n", 10);
    assert_eq!(project_guide.role, "project_guide");
    assert!(!project_guide.default_retrieval);

    let archive = classify_project_document(
        "docs/archive/requirements/old.md",
        "---\nstatus: active\nauthority: normative\n---\n# Old\n",
        64,
    );
    assert_eq!(archive.lifecycle, "archived");
    assert_eq!(archive.authority, "historical");
    assert!(!archive.default_retrieval);

    let historical =
        classify_project_document("documentation/historical/old-requirements.md", "# Old\n", 6);
    assert_eq!(historical.lifecycle, "archived");
    assert_eq!(historical.authority, "historical");
}

#[test]
fn flat_discussion_is_excluded_and_unknown_note_is_ambiguous() {
    let discussion = classify_project_document("docs/新的架构讨论.md", "# 讨论\n", 8);
    assert_eq!(discussion.role, "discussion");
    assert!(!discussion.default_retrieval);

    let unknown = classify_project_document("docs/misc.md", "# Misc\n", 8);
    assert!(unknown.ambiguous);
    assert_eq!(unknown.lifecycle, "unclassified");
}

#[test]
fn frontmatter_can_only_narrow_lifecycle() {
    let metadata = classify_project_document(
        "docs/current/specs/api.md",
        "---\nstatus: deprecated\nscope: api\n---\n# API\n",
        64,
    );
    assert_eq!(metadata.lifecycle, "deprecated");
    assert_eq!(metadata.scope, "api");
    assert!(!metadata.default_retrieval);
}

#[test]
fn frontmatter_version_status_also_narrows_authoritative_paths() {
    let metadata = classify_project_document(
        "docs/current/specs/api.md",
        "---\nversion_status: superseded\n---\n# API\n",
        52,
    );
    assert_eq!(metadata.lifecycle, "superseded");
    assert!(!metadata.default_retrieval);
}

#[test]
fn customization_assets_and_ai_rules_are_not_unknown_notes() {
    let agent =
        classify_project_document(".github/agents/elon-reviewer.agent.md", "# Reviewer\n", 10);
    assert_eq!(agent.role, "agent_definition");
    assert_eq!(agent.authority, "customization");
    assert!(!agent.ambiguous);

    let prompt =
        classify_project_document(".github/prompts/elon-dev-task.prompt.md", "# Prompt\n", 9);
    assert_eq!(prompt.role, "prompt_template");

    let skill = classify_project_document(
        ".github/skills/modular-long-term-dev/SKILL.md",
        "# Skill\n",
        8,
    );
    assert_eq!(skill.role, "skill");

    let bridge = classify_project_document("AI_RULES.md", "# Bridge\n", 9);
    assert_eq!(bridge.role, "project_guide");
    assert!(!bridge.ambiguous);
}
