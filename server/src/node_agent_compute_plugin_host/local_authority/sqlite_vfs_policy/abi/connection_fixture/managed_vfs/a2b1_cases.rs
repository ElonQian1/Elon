//! Static A2b1 review records. The original 28 records remain a legacy non-denominator subset.
//! The exact source-leaf authority separately freezes every Map/Lock terminal and proved
//! exclusion, binds the current source/profile/range ledgers, and reports the mechanically counted
//! StaticContract denominators. Dynamic SQLite/Win32 evidence remains a separate gate.

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

#[test]
fn a2b1_map_static_denominator_is_exact() {
    let count = denominator::validate_map_static_contract().expect("exact A2 Map static contract");
    assert!(count > 0, "Map static denominator must be non-zero");
    println!("Map StaticContract={count}/{count}");
}

#[test]
fn a2b1_lock_static_denominator_is_exact() {
    let count =
        denominator::validate_lock_static_contract().expect("exact A2 Lock static contract");
    assert!(count > 0, "Lock static denominator must be non-zero");
    println!("Lock StaticContract={count}/{count}");
}

#[test]
fn a2b1_map_lock_static_contract_is_exact() {
    let (map, lock) = denominator::validate_map_lock_static_contract()
        .expect("exact aggregate A2 Map/Lock static contract");
    assert!(
        map > 0 && lock > 0,
        "aggregate static counts must be non-zero"
    );
    println!("Map StaticContract={map}/{map}; Lock StaticContract={lock}/{lock}");
}

#[test]
fn a2b1_map_dynamic_quotient_candidate_is_atomically_program_inventory_blocked() {
    let error = denominator::validate_map_dynamic_quotient_candidate_gate()
        .expect_err("Map quotient candidate must remain closed before complete program review");
    assert!(
        matches!(
            error,
            denominator::DynamicQuotientCandidateGateErrorV1::ProgramInventoryIncomplete {
                missing_member_count: 43_470,
                missing_group_count,
            } if missing_group_count > 0
        ),
        "Map candidate must expose the exact incomplete source-program inventory"
    );
}

#[test]
fn a2b1_map_execution_program_inventory_is_complete_but_non_authorizing() {
    let receipt = denominator::inspect_map_execution_program_inventory_gate()
        .expect("complete pre-manifest Map execution-program inventory");
    assert_eq!(receipt.member_count, 43_476);
    assert_eq!(receipt.source_present_member_count, 6);
    assert_eq!(receipt.source_present_group_count, 6);
    assert_eq!(receipt.planned_missing_member_count, 43_470);
    assert_eq!(
        receipt
            .source_present_member_count
            .checked_add(receipt.planned_missing_member_count),
        Some(receipt.member_count),
    );
    assert_eq!(
        receipt
            .source_present_group_count
            .checked_add(receipt.planned_missing_group_count),
        Some(receipt.program_group_count),
    );
    assert_eq!(receipt.inventory_sha256.len(), 64);
    assert!(
        receipt
            .inventory_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "inventory digest must be lowercase hex"
    );
    assert_ne!(receipt.inventory_sha256, "0".repeat(64));
}

#[test]
fn a2b1_lock_dynamic_quotient_candidate_is_atomically_runner_blocked() {
    let error = denominator::validate_lock_dynamic_quotient_candidate_gate()
        .expect_err("Lock quotient candidate must remain closed without complete observation");
    assert_eq!(
        error,
        denominator::DynamicQuotientCandidateGateErrorV1::RunnerCapabilityMissing {
            count: 8_668,
            gap: denominator::CapabilityGapV1::LockObservationIncomplete,
        },
        "Lock candidate must validate every static member and expose the exact root blocker"
    );
}
