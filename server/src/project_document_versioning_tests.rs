use super::*;
use std::{fs, path::Path};

fn repository() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_doc_versions_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    run(&root, &["init", "--initial-branch=main"]);
    run(&root, &["config", "user.name", "Doc Test"]);
    run(&root, &["config", "user.email", "doc-test@example.invalid"]);
    fs::write(root.join("README.md"), "# v1\n").unwrap();
    run(&root, &["add", "README.md"]);
    run(&root, &["commit", "-m", "docs: v1"]);
    root
}

fn run(root: &Path, args: &[&str]) -> String {
    let output = crate::git_command_error::git_command()
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
fn document_commit_can_be_diffed_and_reverted() {
    let root = repository();
    fs::write(root.join("README.md"), "# v2\n").unwrap();
    run(&root, &["add", "README.md"]);
    run(&root, &["commit", "-m", "docs: v2"]);
    let commit = run(&root, &["rev-parse", "HEAD"]);

    let versions = list_document_versions(&root, 10).unwrap();
    assert!(versions
        .iter()
        .any(|version| version.commit == commit && version.reversible));
    let diff = document_version_diff(&root, &commit, Some("README.md")).unwrap();
    assert!(diff["diff"].as_str().unwrap().contains("# v2"));
    let restored = restore_document_version(&root, &commit).unwrap();
    assert_eq!(restored["mode"], "document_revert");
    assert_eq!(
        fs::read_to_string(root.join("README.md"))
            .unwrap()
            .replace("\r\n", "\n"),
        "# v1\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn mixed_code_commit_is_never_one_click_reversible() {
    let root = repository();
    fs::write(root.join("README.md"), "# mixed\n").unwrap();
    fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
    run(&root, &["add", "README.md", "main.rs"]);
    run(&root, &["commit", "-m", "feat: mixed"]);
    let commit = run(&root, &["rev-parse", "HEAD"]);
    let version = list_document_versions(&root, 10)
        .unwrap()
        .into_iter()
        .find(|item| item.commit == commit)
        .unwrap();
    assert!(!version.document_only);
    assert!(!version.reversible);
    assert!(restore_document_version(&root, &commit).is_err());
    fs::remove_dir_all(root).unwrap();
}
