use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn snapshot_reports_docs_and_large_files() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "elon_context_snapshot_{}_{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("AGENTS.md"), "rules").unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]\nname='demo'\n").unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}\n".repeat(802)).unwrap();

    let snapshot = collect_repo_snapshot(&dir);

    assert!(snapshot.instruction_docs.contains(&"AGENTS.md".to_string()));
    assert!(snapshot.manifests.contains(&"Cargo.toml".to_string()));
    assert_eq!(snapshot.large_files[0].path, "src/main.rs");
    assert_eq!(snapshot.large_files[0].lines, 802);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn snapshot_respects_gitignore_when_scanning_sources() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "elon_context_snapshot_ignore_{}_{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::create_dir_all(dir.join("ignored")).unwrap();
    fs::write(dir.join(".gitignore"), "ignored/\n").unwrap();
    fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(
        dir.join("ignored/lib.rs"),
        "pub fn ignored() {}\n".repeat(900),
    )
    .unwrap();

    let snapshot = collect_repo_snapshot(&dir);

    assert_eq!(snapshot.source_file_count, 1);
    assert!(snapshot.large_files.is_empty());

    fs::remove_dir_all(dir).unwrap();
}
