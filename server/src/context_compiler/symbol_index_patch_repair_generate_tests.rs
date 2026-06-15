use super::{
    symbol_index_patch_generation_types::{
        PatchApplyReadiness, PatchApplyReadinessLevel, PatchDiffContract, PatchGenerationMode,
        SymbolPatchGeneration,
    },
    symbol_index_patch_repair_generate::{
        build_repair_generation_messages, extract_repair_patch_from_model_output,
    },
};

#[test]
fn extracts_fenced_repair_diff() {
    let raw = r#"Here is the patch:
```diff
diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,3 +1,3 @@
-let status = 500;
+let status = 401;
```
"#;

    let patch = extract_repair_patch_from_model_output(raw).unwrap();

    assert!(patch.starts_with("diff --git a/src/auth.rs"));
    assert!(patch.contains("+let status = 401;"));
}

#[test]
fn extracts_plain_repair_diff_after_prose() {
    let raw = r#"Repair below.

diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,3 +1,3 @@
-let status = 500;
+let status = 401;
"#;

    let patch = extract_repair_patch_from_model_output(raw).unwrap();

    assert!(patch.starts_with("diff --git a/src/auth.rs"));
    assert!(patch.ends_with('\n'));
}

#[test]
fn rejects_output_without_unified_diff() {
    let raw = "我会把状态码从 500 改成 401。";

    assert!(extract_repair_patch_from_model_output(raw).is_none());
}

#[test]
fn repair_generation_messages_include_repair_contract() {
    let generation = sample_generation();

    let messages = build_repair_generation_messages(
        &generation,
        "diff --git a/src/auth.rs b/src/auth.rs",
        Some("compiler said no"),
        1,
        2,
    );
    let prompt = messages[1]["content"].as_str().unwrap();

    assert!(
        messages[0]["content"]
            .as_str()
            .unwrap()
            .contains("unified diff")
    );
    assert!(prompt.contains("Attempt: 1/2"));
    assert!(prompt.contains("src/auth.rs"));
    assert!(prompt.contains("Do not emit forbidden pattern"));
    assert!(prompt.contains("compiler said no"));
}

fn sample_generation() -> SymbolPatchGeneration {
    SymbolPatchGeneration {
        task: "把登录失败时的 500 改成 401".to_string(),
        mode: PatchGenerationMode::GenerateDiff,
        ready_to_generate: true,
        edit_sequence: Vec::new(),
        diff_contract: PatchDiffContract {
            output_format: "unified_diff".to_string(),
            apply_strategy: "git_apply_check".to_string(),
            allowed_files: vec!["src/auth.rs".to_string()],
            inspect_only_files: Vec::new(),
            forbidden_patterns: vec!["Do not emit forbidden pattern".to_string()],
            required_tests: Vec::new(),
            verification_commands: Vec::new(),
            safety_checks: Vec::new(),
        },
        apply_readiness: PatchApplyReadiness {
            level: PatchApplyReadinessLevel::ReadyAfterDiff,
            apply_check_status: "ready".to_string(),
            can_run_apply_check: true,
            requires_generated_diff: true,
            source_requirements: Vec::new(),
            pre_apply_checks: Vec::new(),
            post_apply_checks: Vec::new(),
            rollback_strategy: "git apply -R".to_string(),
            risk_level: "low".to_string(),
            notes: Vec::new(),
        },
        prompt: "Use src/auth.rs and return a diff.".to_string(),
        blocked_reasons: Vec::new(),
        trace: Vec::new(),
    }
}
