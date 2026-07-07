use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use super::model::{
    ImpactFact, ImpactKind, RelationshipKind, RepoContextIndex, RustImpactAnalysis, RustSymbol,
    SymbolKind, SymbolVisibility,
};

const MAX_SCAN_BYTES: u64 = 512 * 1024;
const MAX_FACTS_PER_KIND: usize = 18;

pub(crate) fn build_rust_impact_analysis(
    workspace: &Path,
    index: &RepoContextIndex,
) -> RustImpactAnalysis {
    let sources = load_sources(workspace, &index.rust.symbols);
    let trait_implementations = collect_trait_implementations(index);
    let function_call_sites = collect_function_call_sites(index, &sources);
    let enum_match_sites = collect_enum_match_sites(index, &sources);
    let field_accesses = collect_field_accesses(index, &sources);
    let public_api_references = collect_public_api_references(index);
    let test_links = collect_test_links(index, &sources);
    let async_boundaries = collect_async_boundaries(index);
    let limitations = collect_limitations(index, &sources);

    RustImpactAnalysis {
        trait_implementations,
        function_call_sites,
        enum_match_sites,
        field_accesses,
        public_api_references,
        test_links,
        async_boundaries,
        limitations,
    }
}


#[path = "impact_analysis_helpers.rs"]
mod helpers;
use self::helpers::*;
