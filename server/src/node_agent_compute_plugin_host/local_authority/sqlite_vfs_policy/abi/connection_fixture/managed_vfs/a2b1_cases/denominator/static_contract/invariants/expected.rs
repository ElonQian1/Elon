use super::super::model::{
    ContractGraph, ContractNode, DmsLockCustody, ExclusionProof, Expected, FailureClass,
    LockEffect, MutationState, NodeKind, RootOperation, SqliteResult,
};

pub(super) fn validate_node_payload(
    graph: &ContractGraph,
    node: &ContractNode,
) -> Result<(), String> {
    match &node.kind {
        NodeKind::Decision => {}
        NodeKind::Continuation { expansion_owner } if expansion_owner.trim().is_empty() => {
            return Err(format!(
                "{} {:?}: empty expansion owner",
                graph.name, node.id
            ));
        }
        NodeKind::Terminal {
            leaf_id, expected, ..
        } => {
            if leaf_id.trim().is_empty() {
                return Err(format!("{} {:?}: empty terminal leaf", graph.name, node.id));
            }
            validate_expected(graph.root_operation, *expected)
                .map_err(|error| format!("{} {:?}: {error}", graph.name, node.id))?;
        }
        NodeKind::Excluded { leaf_id, proof }
            if leaf_id.trim().is_empty() || exclusion_reason(proof).trim().is_empty() =>
        {
            return Err(format!("{} {:?}: invalid exclusion", graph.name, node.id));
        }
        _ => {}
    }
    Ok(())
}

fn exclusion_reason(proof: &ExclusionProof) -> &'static str {
    match proof {
        ExclusionProof::TypeInvariant(reason)
        | ExclusionProof::ControlFlow(reason)
        | ExclusionProof::SafetyPremise(reason) => reason,
    }
}

fn validate_expected(root: RootOperation, expected: Expected) -> Result<(), String> {
    if expected.phase.trim().is_empty() {
        return Err("Expected.phase is empty".to_owned());
    }
    match (root, expected.sqlite) {
        (RootOperation::Map, SqliteResult::LockUnavailable | SqliteResult::Busy)
        | (RootOperation::Lock, SqliteResult::MapUnavailable) => {
            return Err("Expected.sqlite belongs to the other root operation".to_owned());
        }
        _ => {}
    }
    if expected.counts.callback_complete > expected.counts.callback_begin {
        return Err("callback completion count exceeds begin count".to_owned());
    }
    if expected.disposition == super::super::model::TerminalDisposition::Abandoned
        && expected.raw_slots != super::super::model::CustodyState::Cleared
    {
        return Err("abandoned raw state did not clear both raw slots".to_owned());
    }
    if expected.lock_outcome_uncertain && expected.failure != FailureClass::OutcomeUncertainPoisoned
    {
        return Err("uncertain lock outcome lacks a poisoned failure class".to_owned());
    }
    if root == RootOperation::Map && expected.lock_effect != LockEffect::NotReached {
        return Err("Map Expected records a request-lock effect".to_owned());
    }
    match expected.lock_effect {
        LockEffect::Acquired { mask: 0, .. }
        | LockEffect::Released { mask: 0, .. }
        | LockEffect::OutcomeUncertain { mask: 0, .. } => {
            return Err("request-lock effect has an empty byte mask".to_owned());
        }
        LockEffect::Acquired { native: true, .. } if expected.counts.native_lock == 0 => {
            return Err("native acquire effect lacks a native lock observation".to_owned());
        }
        LockEffect::Released { native: true, .. } if expected.counts.native_unlock == 0 => {
            return Err("native release effect lacks a native unlock observation".to_owned());
        }
        LockEffect::OutcomeUncertain { .. } if expected.counts.native_unlock == 0 => {
            return Err("uncertain release effect lacks a native unlock observation".to_owned());
        }
        _ => {}
    }
    if expected.dms_lock == DmsLockCustody::ExclusiveKnown {
        return Err(
            "exclusive DMS custody is an intermediate state, not a production terminal".to_owned(),
        );
    }
    if expected.dms_lock == DmsLockCustody::ExclusiveOutcomeUncertain
        && (!expected.lock_outcome_uncertain
            || expected.failure != FailureClass::OutcomeUncertainPoisoned)
    {
        return Err("uncertain exclusive DMS custody lacks the poisoned lock outcome".to_owned());
    }
    if expected.dms_lock == DmsLockCustody::AcquiredShared && expected.counts.native_lock == 0 {
        return Err("new shared DMS custody lacks a native lock observation".to_owned());
    }
    match expected.failure {
        FailureClass::BusyNoMutation if expected.mutation != MutationState::None => {
            Err("BusyNoMutation has a mutating Expected state".to_owned())
        }
        FailureClass::BusyAfterKnownMutation | FailureClass::MutatedButKnown
            if expected.mutation != MutationState::Known =>
        {
            Err("known mutation failure lacks MutationState::Known".to_owned())
        }
        FailureClass::IoBeforeMutation if expected.mutation != MutationState::None => {
            Err("IoBeforeMutation has a mutating Expected state".to_owned())
        }
        FailureClass::PlatformUnsupported if expected.mutation != MutationState::None => {
            Err("PlatformUnsupported has a mutating Expected state".to_owned())
        }
        _ => Ok(()),
    }?;
    if expected.sqlite == SqliteResult::Busy
        && !matches!(
            expected.failure,
            FailureClass::BusyNoMutation | FailureClass::BusyAfterKnownMutation
        )
    {
        return Err("SQLITE_BUSY lacks a busy failure class".to_owned());
    }
    if expected.sqlite == SqliteResult::Ok
        && !matches!(
            expected.failure,
            FailureClass::None | FailureClass::NotPresent
        )
    {
        return Err("SQLITE_OK carries a failure class".to_owned());
    }
    Ok(())
}
