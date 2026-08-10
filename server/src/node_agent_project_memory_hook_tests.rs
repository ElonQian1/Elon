use super::*;
use serde_json::json;

#[test]
fn lifecycle_is_private_novel_bounded_and_cleaned() {
    let root = test_workspace();
    let session_id = "private-hook-session";
    let session_dir = session_directory(&root, session_id).unwrap();

    let first_paths = (0..8)
        .map(|index| format!("src/evidence_{index}.rs"))
        .collect::<Vec<_>>();
    for relative in &first_paths {
        fs::write(
            root.join(relative),
            format!("pub fn evidence_{relative:?}() {{}}\n"),
        )
        .unwrap();
    }
    fs::write(root.join("src/response_only.rs"), "pub fn hidden() {}\n").unwrap();
    fs::write(root.join("src/secret_token.rs"), "pub fn secret() {}\n").unwrap();

    record_paths(
        &root,
        &session_dir,
        &hook_input(
            "turn-one",
            "read_file",
            json!({
                "paths": first_paths,
                "response": {"path": "src/response_only.rs", "body": "private response body"},
                "prompt": {"path": "src/response_only.rs", "text": "private prompt text"},
                "file_path": "src/secret_token.rs"
            }),
        ),
    )
    .unwrap();
    record_paths(
        &root,
        &session_dir,
        &hook_input(
            "turn-one",
            "apply_patch",
            json!({"command": "*** Update File: src/evidence_1.rs\nprivate command text"}),
        ),
    )
    .unwrap();

    let first_turn = read_observations(
        &session_dir
            .join("observations")
            .join(short_hash(b"turn-one")),
    );
    assert_eq!(first_turn.len(), 8);
    assert_eq!(first_turn["src/evidence_1.rs"].kind, "write");
    assert!(!first_turn.contains_key("src/response_only.rs"));
    assert!(!first_turn.contains_key("src/secret_token.rs"));
    assert_private_ledger(
        &session_dir,
        &[
            session_id,
            "private response body",
            "private prompt text",
            "private command text",
        ],
    );

    let first = stop_decision(&session_dir, &hook_input("turn-one", "", json!({}))).unwrap();
    assert_eq!(first["decision"], "block");
    let selected = prompt::bounded_paths(
        first_turn
            .values()
            .map(|observation| observation.path.as_str()),
        MAX_PROMPT_PATHS,
        MAX_PROMPT_PATH_CHARS,
    );
    assert_eq!(selected.len(), MAX_PROMPT_PATHS);
    assert!(selected.join(", ").chars().count() <= MAX_PROMPT_PATH_CHARS);
    assert_eq!(
        stop_decision(&session_dir, &hook_input("turn-one", "", json!({}))).unwrap()["continue"],
        true
    );

    record_turn(
        &root,
        &session_dir,
        "turn-same",
        &["src/evidence_0.rs", "src/evidence_2.rs"],
    );
    assert_eq!(
        stop_decision(&session_dir, &hook_input("turn-same", "", json!({}))).unwrap()["continue"],
        true
    );

    for index in 2..=4 {
        let turn = format!("turn-{index}");
        let novel = format!("src/novel_{index}.rs");
        fs::write(root.join(&novel), format!("pub fn novel_{index}() {{}}\n")).unwrap();
        record_turn(&root, &session_dir, &turn, &["src/evidence_0.rs", &novel]);
        let decision = stop_decision(&session_dir, &hook_input(&turn, "", json!({}))).unwrap();
        if index <= 3 {
            assert_eq!(decision["decision"], "block");
        } else {
            assert_eq!(decision["continue"], true);
        }
    }
    assert_eq!(session_prompt_count(&session_dir), MAX_SESSION_PROMPTS);

    end_session(&session_dir);
    assert!(!session_dir.exists());
    fs::remove_dir_all(session_dir.parent().unwrap()).ok();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prompt_path_budget_counts_separators() {
    let mut observations = BTreeMap::new();
    for index in 0..10 {
        let path = format!("src/{index}_{}.rs", "x".repeat(105));
        observations.insert(
            path.clone(),
            PathObservation {
                schema: "elon.project_memory_path_observation.v1".into(),
                path,
                kind: "read".into(),
            },
        );
    }
    let paths = prompt::bounded_paths(
        observations
            .values()
            .map(|observation| observation.path.as_str()),
        MAX_PROMPT_PATHS,
        MAX_PROMPT_PATH_CHARS,
    );
    assert!(paths.len() <= MAX_PROMPT_PATHS);
    assert!(paths.join(", ").chars().count() <= MAX_PROMPT_PATH_CHARS);
}

fn test_workspace() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_project_memory_hook_test_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    root.canonicalize().unwrap()
}

fn hook_input(turn_id: &str, tool_name: &str, tool_input: Value) -> HookInput {
    HookInput {
        session_id: "private-hook-session".into(),
        turn_id: turn_id.into(),
        hook_event_name: if tool_name.is_empty() {
            "Stop"
        } else {
            "PostToolUse"
        }
        .into(),
        tool_name: tool_name.into(),
        tool_input,
        ..Default::default()
    }
}

fn record_turn(root: &Path, session_dir: &Path, turn: &str, paths: &[&str]) {
    record_paths(
        root,
        session_dir,
        &hook_input(turn, "read_file", json!({"paths": paths})),
    )
    .unwrap();
}

fn assert_private_ledger(session_dir: &Path, forbidden: &[&str]) {
    let mut stack = vec![session_dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            for value in forbidden {
                assert!(!path.to_string_lossy().contains(value));
                assert!(!text.contains(value));
            }
        }
    }
}
