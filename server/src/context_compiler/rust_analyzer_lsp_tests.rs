use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;

use super::{
    model::{
        RepoContextIndex, RustAnalyzerLspLocationRole, SemanticQuery, SemanticQueryMethod,
        SemanticQueryPlan, SemanticQueryProvider,
    },
    rust_analyzer_lsp::{
        file_uri, find_symbol_character, ordered_executable_queries, summarize_lsp_result,
    },
    rust_analyzer_lsp_locations::{parse_lsp_locations, uri_to_workspace_path},
};

#[test]
fn find_symbol_character_prefers_symbol_column() {
    let text = "fn alpha() {}\n    pub fn target_name(value: usize) {}\n";

    let character = find_symbol_character(text, 2, Some("target_name"));

    assert_eq!(character, 11);
}

#[test]
fn find_symbol_character_falls_back_to_line_start() {
    let text = "fn alpha() {}\nfn beta() {}\n";

    let character = find_symbol_character(text, 2, Some("missing"));

    assert_eq!(character, 0);
}

#[test]
fn file_uri_percent_encodes_spaces() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "elon lsp uri test {} {}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("source file.rs");
    fs::write(&file, "fn main() {}\n").unwrap();

    let uri = file_uri(&file);

    assert!(uri.starts_with("file://"));
    assert!(uri.contains("source%20file.rs"));
    assert!(!uri.contains(' '));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn summarize_lsp_result_counts_arrays() {
    let summary = summarize_lsp_result(
        SemanticQueryMethod::References,
        &json!([
            { "uri": "file:///repo/src/lib.rs" },
            { "uri": "file:///repo/src/main.rs" }
        ]),
    );

    assert_eq!(summary, "2 item(s)");
}

#[test]
fn summarize_lsp_result_extracts_hover_contents() {
    let summary = summarize_lsp_result(
        SemanticQueryMethod::Hover,
        &json!({
            "contents": { "kind": "markdown", "value": "```rust\nfn target_name()\n```" }
        }),
    );

    assert!(summary.contains("target_name"));
}

#[test]
fn ordered_executable_queries_prioritize_core_semantics_over_sampling() {
    let mut workspace_symbol = query(SemanticQueryMethod::WorkspaceSymbol, ".");
    workspace_symbol.priority = 2;
    let mut definition = query(SemanticQueryMethod::Definition, "src/lib.rs");
    definition.priority = 1;
    let mut references = query(SemanticQueryMethod::References, "src/lib.rs");
    references.priority = 1;
    let index = RepoContextIndex {
        semantic_plan: SemanticQueryPlan {
            queries: vec![workspace_symbol, definition, references],
            ..SemanticQueryPlan::default()
        },
        ..RepoContextIndex::default()
    };

    let ordered = ordered_executable_queries(&index, 2);

    assert_eq!(ordered.len(), 2);
    assert_eq!(ordered[0].method, SemanticQueryMethod::Definition);
    assert_eq!(ordered[1].method, SemanticQueryMethod::References);
}

#[test]
fn uri_to_workspace_path_decodes_relative_path() {
    let workspace = std::path::Path::new("C:/repo/with space");
    let uri = "file:///C:/repo/with%20space/src/lib.rs";

    let path = uri_to_workspace_path(workspace, uri).unwrap();

    assert_eq!(path, "src/lib.rs");
}

#[test]
fn parse_lsp_locations_extracts_reference_lines() {
    let workspace = std::path::Path::new("C:/repo");
    let query = query(SemanticQueryMethod::References, "src/lib.rs");
    let value = json!([
        {
            "uri": "file:///C:/repo/src/lib.rs",
            "range": { "start": { "line": 9, "character": 4 }, "end": { "line": 9, "character": 12 } }
        },
        {
            "uri": "file:///C:/repo/src/main.rs",
            "range": { "start": { "line": 20, "character": 0 }, "end": { "line": 21, "character": 1 } }
        }
    ]);

    let locations = parse_lsp_locations(workspace, &query, &value);

    assert_eq!(locations.len(), 2);
    assert_eq!(locations[0].role, RustAnalyzerLspLocationRole::Reference);
    assert_eq!(locations[0].path, "src/lib.rs");
    assert_eq!(locations[0].line, 10);
    assert_eq!(locations[1].end_line, Some(22));
}

#[test]
fn parse_lsp_locations_extracts_workspace_symbols() {
    let workspace = std::path::Path::new("C:/repo");
    let query = query(SemanticQueryMethod::WorkspaceSymbol, ".");
    let value = json!([
        {
            "name": "Runner",
            "kind": 23,
            "location": {
                "uri": "file:///C:/repo/src/lib.rs",
                "range": { "start": { "line": 3, "character": 10 }, "end": { "line": 3, "character": 16 } }
            }
        }
    ]);

    let locations = parse_lsp_locations(workspace, &query, &value);

    assert_eq!(locations.len(), 1);
    assert_eq!(
        locations[0].role,
        RustAnalyzerLspLocationRole::WorkspaceSymbol
    );
    assert_eq!(locations[0].path, "src/lib.rs");
    assert_eq!(locations[0].line, 4);
    assert_eq!(locations[0].symbol.as_deref(), Some("Runner"));
}

#[test]
fn parse_lsp_locations_extracts_location_links() {
    let workspace = std::path::Path::new("C:/repo");
    let query = query(SemanticQueryMethod::Implementation, "src/lib.rs");
    let value = json!([
        {
            "targetUri": "file:///C:/repo/src/impls.rs",
            "targetSelectionRange": { "start": { "line": 14, "character": 8 }, "end": { "line": 14, "character": 18 } }
        }
    ]);

    let locations = parse_lsp_locations(workspace, &query, &value);

    assert_eq!(locations.len(), 1);
    assert_eq!(
        locations[0].role,
        RustAnalyzerLspLocationRole::Implementation
    );
    assert_eq!(locations[0].path, "src/impls.rs");
    assert_eq!(locations[0].line, 15);
}

#[test]
fn parse_lsp_locations_extracts_definition_links() {
    let workspace = std::path::Path::new("C:/repo");
    let query = query(SemanticQueryMethod::Definition, "src/lib.rs");
    let value = json!([
        {
            "targetUri": "file:///C:/repo/src/defs.rs",
            "targetSelectionRange": { "start": { "line": 6, "character": 8 }, "end": { "line": 6, "character": 18 } }
        }
    ]);

    let locations = parse_lsp_locations(workspace, &query, &value);

    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].role, RustAnalyzerLspLocationRole::Definition);
    assert_eq!(locations[0].path, "src/defs.rs");
    assert_eq!(locations[0].line, 7);
}

#[test]
fn parse_lsp_locations_extracts_nested_document_symbols() {
    let workspace = std::path::Path::new("C:/repo");
    let query = query(SemanticQueryMethod::DocumentSymbol, "src/lib.rs");
    let value = json!([
        {
            "name": "Outer",
            "selectionRange": { "start": { "line": 2, "character": 0 }, "end": { "line": 2, "character": 5 } },
            "children": [{
                "name": "inner",
                "selectionRange": { "start": { "line": 8, "character": 4 }, "end": { "line": 8, "character": 9 } }
            }]
        }
    ]);

    let locations = parse_lsp_locations(workspace, &query, &value);

    assert_eq!(locations.len(), 2);
    assert_eq!(locations[0].symbol.as_deref(), Some("Outer"));
    assert_eq!(locations[1].symbol.as_deref(), Some("inner"));
    assert_eq!(locations[1].line, 9);
}

#[test]
fn parse_lsp_locations_extracts_call_hierarchy_items() {
    let workspace = std::path::Path::new("C:/repo");
    let query = query(SemanticQueryMethod::IncomingCalls, "src/lib.rs");
    let value = json!([
        {
            "from": {
                "name": "caller",
                "uri": "file:///C:/repo/src/caller.rs",
                "selectionRange": { "start": { "line": 30, "character": 4 }, "end": { "line": 30, "character": 10 } }
            },
            "fromRanges": []
        }
    ]);

    let locations = parse_lsp_locations(workspace, &query, &value);

    assert_eq!(locations.len(), 1);
    assert_eq!(
        locations[0].role,
        RustAnalyzerLspLocationRole::IncomingCaller
    );
    assert_eq!(locations[0].path, "src/caller.rs");
    assert_eq!(locations[0].symbol.as_deref(), Some("caller"));
}

fn query(method: SemanticQueryMethod, path: &str) -> SemanticQuery {
    SemanticQuery {
        provider: SemanticQueryProvider::RustAnalyzerLsp,
        method,
        path: path.to_string(),
        line: 1,
        symbol: Some("target".to_string()),
        priority: 1,
        reason: "test".to_string(),
    }
}
