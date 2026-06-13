use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    context_evidence::build_context_evidence,
    model::{
        CargoIndex, RankedSymbol, RepoContextIndex, RustIndex, RustSymbol, SymbolGraphSummary,
        SymbolKind, SymbolVisibility, TaskProfile,
    },
};

#[test]
fn builds_snippet_with_hash_and_build_command() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "elon_context_evidence_{}_{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(dir.join("server/src/context_compiler")).unwrap();
    fs::write(
        dir.join("server/src/context_compiler/demo.rs"),
        "pub struct RepoMap;\nimpl RepoMap { pub fn build() {} }\n",
    )
    .unwrap();

    let symbol = RustSymbol {
        id: "server/src/context_compiler/demo.rs:1:struct:RepoMap".to_string(),
        name: "RepoMap".to_string(),
        kind: SymbolKind::Struct,
        path: "server/src/context_compiler/demo.rs".to_string(),
        line_start: 1,
        line_end: 1,
        visibility: SymbolVisibility::Public,
        signature: "pub struct RepoMap".to_string(),
        parent: None,
        docs: None,
        role: "source",
        safety_notes: Vec::new(),
    };
    let index = RepoContextIndex {
        task: TaskProfile {
            keywords: vec!["repo".to_string()],
            ..TaskProfile::default()
        },
        cargo: CargoIndex {
            manifest_path: Some("server/Cargo.toml".to_string()),
            ..CargoIndex::default()
        },
        rust: RustIndex {
            files_scanned: 1,
            symbols: vec![symbol.clone()],
            warnings: Vec::new(),
        },
        graph: SymbolGraphSummary {
            ranked_symbols: vec![RankedSymbol {
                id: symbol.id.clone(),
                name: symbol.name.clone(),
                kind: symbol.kind,
                path: symbol.path.clone(),
                line_start: 1,
                line_end: 1,
                score: 1.0,
                reasons: vec!["task term hits".to_string()],
            }],
            ..SymbolGraphSummary::default()
        },
        rust_analyzer: Default::default(),
        semantic_plan: Default::default(),
        impact: Default::default(),
        evidence: Default::default(),
    };

    let evidence = build_context_evidence(&dir, &index, &[]);

    assert_eq!(
        evidence.snippets[0].path,
        "server/src/context_compiler/demo.rs"
    );
    assert!(evidence.snippets[0].sha256.len() >= 64);
    assert!(evidence
        .build_commands
        .iter()
        .any(|command| command.command.contains("context_compiler")));
    fs::remove_dir_all(dir).unwrap();
}
