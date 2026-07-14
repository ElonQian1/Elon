use std::{collections::HashSet, fs, path::Path};

use sha2::{Digest, Sha256};

use super::{
    model::{
        BuildCommand, ContextEvidence, ContextFact, EvidenceSnippet, FeatureFlagFact,
        NeighborSummary, RankedSymbol, RelationshipKind, RepoContextIndex, RustSymbol,
        SymbolVisibility, TaskProfile, TestTarget,
    },
    relevance::RelevantFile,
};

const MAX_SNIPPETS: usize = 8;
const MAX_SNIPPET_LINES: usize = 160;
const MAX_NEIGHBORS: usize = 12;
const MAX_FACTS: usize = 20;

pub(crate) fn build_context_evidence(
    workspace: &Path,
    index: &RepoContextIndex,
    relevant_files: &[RelevantFile],
) -> ContextEvidence {
    let mut snippets = collect_snippets(workspace, index, relevant_files);
    snippets.sort_by(|left, right| {
        role_priority(right.role)
            .cmp(&role_priority(left.role))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    snippets.truncate(MAX_SNIPPETS);

    let snippet_paths = snippets
        .iter()
        .map(|snippet| snippet.path.clone())
        .collect::<HashSet<_>>();
    let neighbor_summaries = collect_neighbor_summaries(index, &snippet_paths);
    let test_targets = collect_test_targets(index, &snippet_paths);
    let build_commands = collect_build_commands(index, &snippet_paths);
    let invariants = collect_invariants(index);
    let public_api_contracts = collect_public_api_contracts(index);
    let unsafe_boundaries = collect_unsafe_boundaries(index);
    let feature_flags = collect_feature_flags(index);
    let missing_context = collect_missing_context(index, &snippets, &test_targets);
    let recommended_actions = recommended_actions(&index.task);

    ContextEvidence {
        snippets,
        neighbor_summaries,
        test_targets,
        build_commands,
        invariants,
        public_api_contracts,
        unsafe_boundaries,
        feature_flags,
        missing_context,
        recommended_actions,
    }
}

#[path = "context_evidence_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
