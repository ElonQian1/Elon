use std::{fs, path::Path};

use super::{
    model::{RustIndex, RustSymbol, SymbolKind, SymbolVisibility},
    repo_snapshot::{relative_path, source_role},
    repo_walk, rust_imports,
};

const MAX_RUST_FILE_BYTES: u64 = 512 * 1024;

pub(crate) fn collect_rust_index(workspace: &Path, max_files: usize) -> RustIndex {
    let mut index = RustIndex::default();
    scan_dir(workspace, workspace, max_files, &mut index);
    assign_impl_parents(&mut index.symbols);
    index
}


#[path = "rust_symbols_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
