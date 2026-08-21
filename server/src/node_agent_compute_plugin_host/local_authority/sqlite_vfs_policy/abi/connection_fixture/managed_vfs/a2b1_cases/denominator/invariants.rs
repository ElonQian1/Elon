use std::collections::BTreeSet;

use super::{
    branch_atoms::{
        all_candidate_branch_atoms, RawSourceBranchAtomId, ALL_ABI_LOCK_BRANCHES,
        ALL_ABI_MAP_BRANCHES, ALL_CALLBACK_BRANCHES, ALL_FAULT_CONTROLLER_BRANCHES,
        ALL_INITIALIZATION_BRANCHES, ALL_INVOKERS, ALL_LOCK_BRANCHES, ALL_MAP_BRANCHES,
        ALL_ROUTE_BRIDGE_BRANCHES, INITIALIZATION_INVOKERS,
    },
    case_key::{FailureClass, Path},
    projection::{
        disposition, CandidateAxis, CandidateKeyProjection, ExpectedStatus, RawDisposition,
    },
};

pub(super) fn validate() -> Result<(), &'static str> {
    let raw = all_candidate_branch_atoms();
    validate_product_shape(&raw)?;
    let raw_set = raw.iter().copied().collect::<BTreeSet<_>>();
    if raw_set.len() != raw.len() {
        return Err("duplicate A2a/A2b1 candidate branch atom id");
    }

    let mut included = BTreeSet::new();
    let mut excluded = BTreeSet::new();
    for id in raw.iter().copied() {
        match disposition(id) {
            RawDisposition::Included(candidate) => {
                validate_candidate(candidate)?;
                if candidate.raw != id || !included.insert(id) {
                    return Err("duplicate or detached included candidate branch atom");
                }
            }
            RawDisposition::Excluded(exclusion) => {
                if exclusion.raw != id || !excluded.insert(id) {
                    return Err("duplicate or detached excluded candidate branch atom");
                }
            }
        }
    }

    if !included.is_disjoint(&excluded) {
        return Err("included and excluded candidate branch atoms overlap");
    }
    let partition = included.union(&excluded).copied().collect::<BTreeSet<_>>();
    if partition != raw_set {
        return Err("candidate branch-atom table partition is not self-consistent");
    }
    Ok(())
}

fn validate_product_shape(raw: &[RawSourceBranchAtomId]) -> Result<(), &'static str> {
    let expected = ALL_ABI_MAP_BRANCHES.len()
        + ALL_ABI_LOCK_BRANCHES.len()
        + ALL_INVOKERS.len() * ALL_CALLBACK_BRANCHES.len()
        + ALL_ROUTE_BRIDGE_BRANCHES.len()
        + INITIALIZATION_INVOKERS.len() * ALL_INITIALIZATION_BRANCHES.len()
        + 2 * ALL_MAP_BRANCHES.len()
        + 4 * ALL_LOCK_BRANCHES.len()
        + ALL_FAULT_CONTROLLER_BRANCHES.len();
    if raw.len() != expected {
        return Err("candidate branch-atom product omitted or added a listed family");
    }
    Ok(())
}

fn validate_candidate(candidate: CandidateKeyProjection) -> Result<(), &'static str> {
    if candidate.expected != ExpectedStatus::PendingSourceAndRedTeamReview {
        return Err("candidate projection incorrectly claims frozen Expected");
    }
    if candidate
        .invoker
        .is_some_and(|invoker| candidate.path != CandidateAxis::Exact(invoker.path()))
    {
        return Err("candidate invoker projects to the wrong path");
    }
    if matches!(
        candidate.failure,
        CandidateAxis::Exact(FailureClass::PlatformUnsupported)
    ) {
        return Err("non-Windows Unsupported branch entered the candidate projection");
    }
    match candidate.raw {
        RawSourceBranchAtomId::AbiMap(_) if candidate.path != CandidateAxis::Exact(Path::Map) => {
            Err("ABI map branch projected outside Map")
        }
        RawSourceBranchAtomId::AbiLock(_) if candidate.path != CandidateAxis::Exact(Path::Lock) => {
            Err("ABI lock branch projected outside Lock")
        }
        _ => Ok(()),
    }
}
