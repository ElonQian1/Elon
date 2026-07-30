use super::*;

fn test_workspace(label: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("elon-doc-trigger-{label}-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(root.join(".git")).expect("create fake git workspace");
    root
}

#[test]
fn trigger_is_durable_idempotent_and_claimable() {
    let root = test_workspace("lifecycle");
    let commit = "a".repeat(40);
    let first = enqueue(
        &root,
        &commit,
        "blocking",
        &["docs/large.md".to_string()],
        &["document entered red zone".to_string()],
    )
    .expect("enqueue trigger");
    let repeated = enqueue(
        &root,
        &commit,
        "blocking",
        &["docs/large.md".to_string()],
        &[],
    )
    .expect("repeat trigger");
    assert_eq!(first["trigger_id"], repeated["trigger_id"]);
    assert_eq!(
        get_pending(&root).expect("pending")["trigger"]["status"],
        "pending"
    );

    let claimed = claim(
        &root,
        first["trigger_id"].as_str().expect("trigger id"),
        first["operation_id"].as_str().expect("operation id"),
    )
    .expect("claim trigger");
    assert_eq!(claimed["status"], "claimed");
    assert!(get_pending(&root).expect("claimed pending")["trigger"].is_null());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn newer_commit_replaces_claimed_trigger() {
    let root = test_workspace("replace");
    let first = enqueue(
        &root,
        &"b".repeat(40),
        "warning",
        &["docs/a.md".to_string()],
        &[],
    )
    .expect("first trigger");
    claim(
        &root,
        first["trigger_id"].as_str().expect("trigger id"),
        first["operation_id"].as_str().expect("operation id"),
    )
    .expect("claim first");
    let second = enqueue(
        &root,
        &"c".repeat(40),
        "warning",
        &["docs/b.mdx".to_string()],
        &[],
    )
    .expect("second trigger");
    assert_ne!(first["trigger_id"], second["trigger_id"]);
    assert_eq!(
        get_pending(&root).expect("pending")["trigger"]["commit_sha"],
        "c".repeat(40)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn trigger_rejects_unsafe_document_paths() {
    let root = test_workspace("unsafe");
    let error = enqueue(
        &root,
        &"d".repeat(40),
        "warning",
        &["../outside.md".to_string()],
        &[],
    )
    .expect_err("unsafe path must fail");
    assert!(error.to_string().contains("Markdown"));
    let _ = fs::remove_dir_all(root);
}
