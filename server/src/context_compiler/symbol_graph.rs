use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::Path,
};

use super::{
    model::{
        CodeRelationship, RankedFile, RankedSymbol, RelationshipKind, RustIndex, RustSymbol,
        SymbolGraphSummary, SymbolKind, SymbolVisibility,
    },
    repo_map_tags,
    repo_snapshot::{relative_path, source_role},
};

const MAX_RELATIONSHIP_SCAN_BYTES: u64 = 384 * 1024;

pub(crate) fn build_symbol_graph(
    workspace: &Path,
    rust: &RustIndex,
    user_message: &str,
    max_symbols: usize,
    max_relationships: usize,
) -> SymbolGraphSummary {
    if rust.symbols.is_empty() {
        return SymbolGraphSummary {
            warnings: vec!["未抽取到 Rust 符号，跳过符号图。".to_string()],
            ..SymbolGraphSummary::default()
        };
    }

    let task_terms = extract_task_terms(user_message);
    let tag_index =
        repo_map_tags::build_repo_map_tag_index(workspace, &rust.symbols, max_relationships);
    let relationships = repo_map_tags::merge_relationships(
        tag_index.relationships.clone(),
        collect_relationships(workspace, &rust.symbols, max_relationships),
        max_relationships,
    );
    let file_rank = page_rank(&rust.symbols, &relationships);
    let ranked_symbols = rank_symbols(
        &rust.symbols,
        &relationships,
        &file_rank,
        &task_terms,
        max_symbols,
    );
    let ranked_files = rank_files(&rust.symbols, &ranked_symbols, &file_rank, &task_terms);

    SymbolGraphSummary {
        ranked_files,
        ranked_symbols,
        relationships,
        repo_map_tags: tag_index.summary,
        warnings: tag_index.warnings,
    }
}

#[path = "symbol_graph_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
