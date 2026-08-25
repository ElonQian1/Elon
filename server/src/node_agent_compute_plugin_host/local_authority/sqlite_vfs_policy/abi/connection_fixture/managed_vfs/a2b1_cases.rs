//! Static A2b1 review records. The original 28 records are a legacy non-denominator subset. The
//! adjacent module validates the new schema, an explicitly incomplete candidate branch-atom
//! scaffold, the commit-bound source-owner graph, and a Map-only terminal-template review ledger
//! whose pending gates stay explicit. Terminal source closure, exact keys, Expected,
//! StaticContract, denominator, and dynamic SQLite/Win32 evidence remain pending.

mod denominator;
mod lock;
mod map;
mod model;
mod operation;

#[test]
fn a2b1_legacy_non_denominator_subset_is_self_consistent() {
    model::validate_matrix(map::CASES, lock::CASES).expect("valid A2b1 static case matrix");
}

#[test]
fn a2b1_candidate_branch_atom_scaffold_is_self_consistent() {
    denominator::validate_candidate_branch_atom_scaffold()
        .expect("self-consistent incomplete A2a/A2b1 branch-atom review scaffold");
}

#[test]
fn a2b1_source_owner_graph_is_self_consistent() {
    denominator::validate_source_owner_graph()
        .expect("self-consistent commit-bound A2a/A2b1 source owner graph");
}

#[test]
fn a2b1_map_terminal_review_ledger_is_self_consistent() {
    denominator::validate_map_terminal_review_ledger()
        .expect("self-consistent source-review-only A2 Map terminal-template ledger");
}
