use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    model::{RustSymbol, SymbolKind, SymbolVisibility},
    repo_map_tags,
};

#[test]
fn builds_aider_style_def_ref_edges() {
    let dir = temp_dir("elon_repo_map_tags");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/lib.rs"),
        "pub struct RepoMap;\npub fn build_repo_map() -> RepoMap { RepoMap }\n",
    )
    .unwrap();
    fs::write(
        dir.join("src/main.rs"),
        "use crate::RepoMap;\nfn main() { let _map: RepoMap = build_repo_map(); }\n",
    )
    .unwrap();

    let symbols = vec![
        symbol("src/lib.rs", 1, "RepoMap", SymbolKind::Struct),
        symbol("src/lib.rs", 2, "build_repo_map", SymbolKind::Function),
        symbol("src/main.rs", 2, "main", SymbolKind::Function),
    ];

    let index = repo_map_tags::build_repo_map_tag_index(&dir, &symbols, 12);

    assert_eq!(index.summary.definitions, 3);
    assert!(index.summary.references >= 2);
    assert!(index
        .summary
        .edges
        .iter()
        .any(|edge| edge.from_path == "src/main.rs"
            && edge.to_path == "src/lib.rs"
            && edge.symbol == "RepoMap"));
}

fn symbol(path: &str, line: usize, name: &str, kind: SymbolKind) -> RustSymbol {
    RustSymbol {
        id: format!("{path}:{line}:{}:{name}", kind.as_str()),
        name: name.to_string(),
        kind,
        path: path.to_string(),
        line_start: line,
        line_end: line,
        visibility: SymbolVisibility::Public,
        signature: name.to_string(),
        parent: None,
        docs: None,
        role: "source",
        safety_notes: Vec::new(),
    }
}

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nonce))
}
