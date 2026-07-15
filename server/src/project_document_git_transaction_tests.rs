use std::{fs, path::Path, path::PathBuf, process::Command};

use sha2::Digest;

use crate::project_document_git_transaction::{
    commit_document_baseline, commit_document_result, verify_document_baseline,
};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
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

fn repository() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_document_git_transaction_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("docs/original.md"), "# Original\n").unwrap();
    fs::write(root.join("src/code.rs"), "fn original() {}\n").unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["add", "."]);
    git(
        &root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "-m",
            "initial",
        ],
    );
    root
}

#[test]
fn before_and_after_document_commits_preserve_staged_code() {
    let root = repository();
    fs::write(root.join("src/code.rs"), "fn staged_change() {}\n").unwrap();
    git(&root, &["add", "src/code.rs"]);
    fs::write(root.join("docs/original.md"), "# Original\n\nMessy note.\n").unwrap();

    let baseline = commit_document_baseline(&root).unwrap();
    let revision = format!("{:x}", sha2::Sha256::digest(b"# Original\n\nMessy note.\n"));
    verify_document_baseline(&root, &baseline, "docs/original.md", &revision).unwrap();
    assert_eq!(
        git(&root, &["diff", "--cached", "--name-only"]),
        "src/code.rs"
    );

    fs::rename(
        root.join("docs/original.md"),
        root.join("docs/organized-note.md"),
    )
    .unwrap();
    let result = commit_document_result(&root, &baseline).unwrap();
    assert_ne!(baseline, result);
    assert_eq!(
        git(&root, &["diff", "--cached", "--name-only"]),
        "src/code.rs"
    );
    assert_eq!(
        git(&root, &["log", "-2", "--pretty=%s"]),
        "chore(docs): apply AI organization\nchore(docs): snapshot before AI organization"
    );
    fs::remove_dir_all(root).unwrap();
}
