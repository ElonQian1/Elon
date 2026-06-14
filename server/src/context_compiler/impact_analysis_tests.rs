use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    impact_analysis,
    model::{
        CodeRelationship, RankedSymbol, RelationshipKind, RepoContextIndex, RustIndex, RustSymbol,
        SymbolGraphSummary, SymbolKind, SymbolVisibility,
    },
};

#[test]
fn builds_rust_refactor_impact_facts() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "elon_context_impact_{}_{}",
        std::process::id(),
        nonce
    ));
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("lib.rs"),
        r#"pub trait Runner {
    fn run(&self);
}

pub struct Job {
    pub name: String,
    pub status: Status,
}

pub enum Status {
    Ready,
    Done,
}

impl Runner for Job {
    pub async fn run(&self) {
        process(self.name.clone()).await;
    }
}

pub async fn process(name: String) -> String {
    name
}

fn caller(job: &mut Job) {
    job.name = "queued".to_string();
    let _value = process(job.name.clone());
    match Status::Ready {
        Status::Ready => {}
        Status::Done => {}
    }
}

#[test]
fn process_test() {
    let _ = process("a".to_string());
}
"#,
    )
    .unwrap();

    let symbols = vec![
        symbol(
            "src/lib.rs:1:trait:Runner",
            "Runner",
            SymbolKind::Trait,
            1,
            3,
            "pub trait Runner",
        ),
        symbol(
            "src/lib.rs:5:struct:Job",
            "Job",
            SymbolKind::Struct,
            5,
            8,
            "pub struct Job",
        ),
        symbol(
            "src/lib.rs:10:enum:Status",
            "Status",
            SymbolKind::Enum,
            10,
            13,
            "pub enum Status",
        ),
        symbol(
            "src/lib.rs:15:impl:impl Runner for Job",
            "impl Runner for Job",
            SymbolKind::Impl,
            15,
            19,
            "impl Runner for Job",
        ),
        symbol(
            "src/lib.rs:16:function:run",
            "run",
            SymbolKind::Function,
            16,
            18,
            "pub async fn run(&self)",
        ),
        symbol(
            "src/lib.rs:21:function:process",
            "process",
            SymbolKind::Function,
            21,
            23,
            "pub async fn process(name: String) -> String",
        ),
        symbol(
            "src/lib.rs:25:function:caller",
            "caller",
            SymbolKind::Function,
            25,
            31,
            "fn caller(job: &mut Job)",
        ),
        RustSymbol {
            role: "test",
            ..symbol(
                "src/lib.rs:34:function:process_test",
                "process_test",
                SymbolKind::Function,
                34,
                36,
                "#[test] fn process_test",
            )
        },
    ];
    let index = RepoContextIndex {
        rust: RustIndex {
            files_scanned: 1,
            symbols,
            ..RustIndex::default()
        },
        graph: SymbolGraphSummary {
            ranked_symbols: vec![
                ranked(
                    "src/lib.rs:21:function:process",
                    "process",
                    SymbolKind::Function,
                    21,
                    23,
                ),
                ranked(
                    "src/lib.rs:10:enum:Status",
                    "Status",
                    SymbolKind::Enum,
                    10,
                    13,
                ),
                ranked("src/lib.rs:5:struct:Job", "Job", SymbolKind::Struct, 5, 8),
                ranked(
                    "src/lib.rs:16:function:run",
                    "run",
                    SymbolKind::Function,
                    16,
                    18,
                ),
            ],
            relationships: vec![CodeRelationship {
                from_path: "src/lib.rs".to_string(),
                to_symbol_id: "src/lib.rs:21:function:process".to_string(),
                to_symbol_name: "process".to_string(),
                to_path: "src/lib.rs".to_string(),
                kind: RelationshipKind::TestCovers,
                line: 34,
                reason: "test mentions process".to_string(),
            }],
            ..SymbolGraphSummary::default()
        },
        ..RepoContextIndex::default()
    };

    let impact = impact_analysis::build_rust_impact_analysis(&dir, &index);

    assert!(!impact.trait_implementations.is_empty());
    assert!(!impact.function_call_sites.is_empty());
    assert!(!impact.enum_match_sites.is_empty());
    assert!(!impact.field_accesses.is_empty());
    assert!(!impact.test_links.is_empty());
    assert!(!impact.async_boundaries.is_empty());

    fs::remove_dir_all(dir).unwrap();
}

fn symbol(
    id: &str,
    name: &str,
    kind: SymbolKind,
    line_start: usize,
    line_end: usize,
    signature: &str,
) -> RustSymbol {
    RustSymbol {
        id: id.to_string(),
        name: name.to_string(),
        kind,
        path: "src/lib.rs".to_string(),
        line_start,
        line_end,
        visibility: SymbolVisibility::Public,
        signature: signature.to_string(),
        parent: None,
        docs: None,
        role: "source",
        safety_notes: if signature.contains("async") {
            vec!["await boundary".to_string()]
        } else {
            Vec::new()
        },
    }
}

fn ranked(
    id: &str,
    name: &str,
    kind: SymbolKind,
    line_start: usize,
    line_end: usize,
) -> RankedSymbol {
    RankedSymbol {
        id: id.to_string(),
        name: name.to_string(),
        kind,
        path: "src/lib.rs".to_string(),
        line_start,
        line_end,
        score: 10.0,
        reasons: Vec::new(),
    }
}
