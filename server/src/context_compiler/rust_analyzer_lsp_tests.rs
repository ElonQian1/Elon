use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;

use super::{
    model::SemanticQueryMethod,
    rust_analyzer_lsp::{file_uri, find_symbol_character, summarize_lsp_result},
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
