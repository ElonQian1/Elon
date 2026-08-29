//! Exact append-only action, native-adapter and delete-prestate receipt validation.

use anyhow::anyhow;

use super::super::super::a2b2_cases::UnmapSelector;
use crate::node_agent_managed_fs::{
    ManagedSqliteDeleteOutcome as DeleteOutcome, ManagedSqliteObservedLock,
    ManagedSqliteShmFailurePhase as Phase, ManagedSqliteShmTestUnmapActionEvent as Event,
    ManagedSqliteShmTestUnmapActionOutcome as Action,
    ManagedSqliteShmTestUnmapDeletePrestate as Prestate,
    ManagedSqliteShmTestUnmapNativeObservation as NativeObservation,
    ManagedSqliteShmTestUnmapNativeOperation as Native,
    ManagedSqliteShmTestUnmapNativeTiming as NativeTiming, ManagedSqliteShmTestUnmapReceipt,
};

#[derive(Debug, Clone, Copy)]
enum Boundary {
    Before,
    Native,
    After,
}

pub(super) fn validate_and_count(
    selector: UnmapSelector,
    receipt: &ManagedSqliteShmTestUnmapReceipt,
    outer_attempt: u8,
    outer_success: u8,
) -> anyhow::Result<(u8, u8)> {
    if !receipt.finished || receipt.pending != 0 {
        return Err(anyhow!(
            "final Unmap low-level receipt is unsealed or pending"
        ));
    }
    let expected = expected_actions(selector);
    if receipt.actions != expected {
        return Err(anyhow!(
            "final Unmap action ledger disagrees with the selected real boundary"
        ));
    }
    validate_native(selector, receipt)?;
    validate_prestate(selector, receipt)?;
    validate_delete_outcome(selector, receipt)?;
    validate_delete_authority(selector, receipt)?;

    if super::outcome::is_success(selector) {
        return count_actions(&receipt.actions, None);
    }
    if let Some((phase, _)) = selected_physical_boundary(selector) {
        return count_actions(&receipt.actions, Some(phase));
    }
    if is_detach_or_completion(selector) {
        return Ok((outer_attempt, outer_success));
    }
    Ok((0, 0))
}

fn expected_actions(selector: UnmapSelector) -> Vec<Event> {
    if super::outcome::node_absent(selector) || is_authorization(selector) {
        return Vec::new();
    }
    let selected = selected_physical_boundary(selector);
    let mut events = Vec::with_capacity(10);
    for phase in [
        Phase::ViewUnmap,
        Phase::MappingClose,
        Phase::DmsSharedRelease,
        Phase::FileClose,
    ] {
        if let Some((selected_phase, boundary)) = selected {
            if selected_phase == phase {
                push_boundary(&mut events, phase, boundary);
                return events;
            }
        }
        push_success(&mut events, phase);
    }
    if super::outcome::is_delete(selector) {
        if let Some((Phase::ExactSiblingDelete, boundary)) = selected {
            push_boundary(&mut events, Phase::ExactSiblingDelete, boundary);
            return events;
        }
        push_success(&mut events, Phase::ExactSiblingDelete);
    }
    events
}

fn push_boundary(events: &mut Vec<Event>, phase: Phase, boundary: Boundary) {
    match boundary {
        Boundary::Before => {}
        Boundary::Native => events.push(event(phase, Action::Attempt)),
        Boundary::After => push_success(events, phase),
    }
}

fn push_success(events: &mut Vec<Event>, phase: Phase) {
    events.push(event(phase, Action::Attempt));
    events.push(event(phase, Action::Success));
}

fn event(phase: Phase, outcome: Action) -> Event {
    Event {
        phase,
        outcome,
        ordinal: 1,
    }
}

fn count_actions(events: &[Event], selected: Option<Phase>) -> anyhow::Result<(u8, u8)> {
    let count = |outcome| {
        events
            .iter()
            .filter(|event| event.outcome == outcome && selected.is_none_or(|p| event.phase == p))
            .count()
    };
    Ok((
        u8::try_from(count(Action::Attempt))?,
        u8::try_from(count(Action::Success))?,
    ))
}

fn selected_physical_boundary(selector: UnmapSelector) -> Option<(Phase, Boundary)> {
    use UnmapSelector as S;
    let value = match selector {
        S::FinalKeepViewUnmapBefore => (Phase::ViewUnmap, Boundary::Before),
        S::FinalKeepViewUnmapNativeUncertain => (Phase::ViewUnmap, Boundary::Native),
        S::FinalKeepViewUnmapAfterKnown | S::FinalKeepViewUnmapAfterUncertain => {
            (Phase::ViewUnmap, Boundary::After)
        }
        S::FinalKeepMappingCloseBefore => (Phase::MappingClose, Boundary::Before),
        S::FinalKeepMappingCloseNativeUncertain => (Phase::MappingClose, Boundary::Native),
        S::FinalKeepMappingCloseAfterKnown | S::FinalKeepMappingCloseAfterUncertain => {
            (Phase::MappingClose, Boundary::After)
        }
        S::FinalKeepDmsReleaseBefore => (Phase::DmsSharedRelease, Boundary::Before),
        S::FinalKeepDmsReleaseNativeUncertain => (Phase::DmsSharedRelease, Boundary::Native),
        S::FinalKeepDmsReleaseAfterKnown | S::FinalKeepDmsReleaseAfterUncertain => {
            (Phase::DmsSharedRelease, Boundary::After)
        }
        S::FinalKeepFileCloseBefore => (Phase::FileClose, Boundary::Before),
        S::FinalKeepFileCloseNativeRetryable | S::FinalKeepFileCloseNativeUncertain => {
            (Phase::FileClose, Boundary::Native)
        }
        S::FinalKeepFileCloseAfterKnown | S::FinalKeepFileCloseAfterUncertain => {
            (Phase::FileClose, Boundary::After)
        }
        S::FinalDeleteSiblingBefore => (Phase::ExactSiblingDelete, Boundary::Before),
        S::FinalDeleteSiblingNativeRetryable | S::FinalDeleteSiblingNativeUncertain => {
            (Phase::ExactSiblingDelete, Boundary::Native)
        }
        S::FinalDeleteSiblingAfterKnown | S::FinalDeleteSiblingAfterUncertain => {
            (Phase::ExactSiblingDelete, Boundary::After)
        }
        _ => return None,
    };
    Some(value)
}

fn validate_native(
    selector: UnmapSelector,
    receipt: &ManagedSqliteShmTestUnmapReceipt,
) -> anyhow::Result<()> {
    let expected = expected_native(selector);
    match (expected, receipt.native) {
        (None, None) => Ok(()),
        (Some(operation), Some(actual))
            if actual.operation == operation
                && actual.timing == operation.timing()
                && actual.triggered
                && actual.witnessed
                && actual.observation
                    == Some(match operation.timing() {
                        NativeTiming::Retryable => NativeObservation::NativeFailureObserved,
                        NativeTiming::OutcomeUncertain => {
                            NativeObservation::ReturnReceiptUnavailable
                        }
                    }) =>
        {
            Ok(())
        }
        _ => Err(anyhow!("final Unmap native adapter receipt mismatch")),
    }
}

fn expected_native(selector: UnmapSelector) -> Option<Native> {
    use UnmapSelector as S;
    match selector {
        S::FinalKeepViewUnmapNativeUncertain => Some(Native::ViewUnmapOutcomeUncertain),
        S::FinalKeepMappingCloseNativeUncertain => Some(Native::MappingCloseOutcomeUncertain),
        S::FinalKeepDmsReleaseNativeUncertain => Some(Native::DmsSharedReleaseOutcomeUncertain),
        S::FinalKeepFileCloseNativeRetryable => Some(Native::FileCloseRetryable),
        S::FinalKeepFileCloseNativeUncertain => Some(Native::FileCloseOutcomeUncertain),
        S::FinalDeleteSiblingNativeRetryable => Some(Native::ExactSiblingDeleteRetryable),
        S::FinalDeleteSiblingNativeUncertain => Some(Native::ExactSiblingDeleteOutcomeUncertain),
        _ => None,
    }
}

fn validate_prestate(
    selector: UnmapSelector,
    receipt: &ManagedSqliteShmTestUnmapReceipt,
) -> anyhow::Result<()> {
    let expected = expected_prestate(selector);
    match (expected, receipt.prestate) {
        (None, None) => Ok(()),
        (Some(prestate), Some(actual))
            if actual.prestate == prestate
                && actual.consumed
                && actual.applied
                && if prestate == Prestate::NotFound {
                    actual.setup_delete_attempts == 1
                        && actual.setup_delete_outcome == Some(DeleteOutcome::Deleted)
                } else {
                    actual.setup_delete_attempts == 0 && actual.setup_delete_outcome.is_none()
                } =>
        {
            Ok(())
        }
        _ => Err(anyhow!("final Unmap delete prestate receipt mismatch")),
    }
}

fn validate_delete_outcome(
    selector: UnmapSelector,
    receipt: &ManagedSqliteShmTestUnmapReceipt,
) -> anyhow::Result<()> {
    use UnmapSelector as S;

    let expected = match selector {
        S::FinalDeleteSuccessNotFound => Some(DeleteOutcome::NotFound),
        S::FinalDeleteSiblingAfterKnown
        | S::FinalDeleteSiblingAfterUncertain
        | S::FinalDeleteDetachBefore
        | S::FinalDeleteDetachAfterKnown
        | S::FinalDeleteDetachAfterUncertain
        | S::FinalDeleteCompletionNativeUncertain
        | S::FinalDeleteSuccessDeleted => Some(DeleteOutcome::Deleted),
        _ => None,
    };
    if receipt.delete_outcome != expected {
        return Err(anyhow!(
            "final Unmap exact-sibling delete outcome receipt mismatch"
        ));
    }
    Ok(())
}

fn validate_delete_authority(
    selector: UnmapSelector,
    receipt: &ManagedSqliteShmTestUnmapReceipt,
) -> anyhow::Result<()> {
    use UnmapSelector as S;

    if !super::outcome::is_delete(selector) {
        if receipt.delete_authority.is_none() {
            return Ok(());
        }
        return Err(anyhow!(
            "Keep-mode final Unmap unexpectedly recorded Delete authority"
        ));
    }
    let actual = receipt
        .delete_authority
        .ok_or_else(|| anyhow!("final Delete authority receipt is missing"))?;
    let (request_present, identity_matches, generation_matches) = match selector {
        S::FinalDeleteAuthMainIdentityMissing => (false, false, true),
        S::FinalDeleteAuthMainOrGenerationMismatch => (true, true, false),
        _ => (true, true, true),
    };
    let lock_matches = match selector {
        S::FinalDeleteAuthLockStateUncertain => {
            actual.lock_query_unavailable && actual.lock_level.is_none()
        }
        S::FinalDeleteAuthMainNotExclusive => {
            !actual.lock_query_unavailable
                && actual
                    .lock_level
                    .is_some_and(|level| level != ManagedSqliteObservedLock::Exclusive)
        }
        _ => {
            !actual.lock_query_unavailable
                && actual.lock_level == Some(ManagedSqliteObservedLock::Exclusive)
        }
    };
    let correct_request_expected = selector != S::FinalDeleteAuthMainNotExclusive;
    let selected_request_expected = !is_authorization(selector);
    if !actual.stored_identity_present
        || actual.request_identity_present != request_present
        || actual.identity_matches != identity_matches
        || actual.generation_matches != generation_matches
        || !actual.stored_identity_unchanged
        || !actual.selected_request_validation_attempted
        || actual.selected_request_validation_succeeded != selected_request_expected
        || !actual.correct_request_recheck_attempted
        || actual.correct_request_recheck_succeeded != correct_request_expected
        || !lock_matches
    {
        return Err(anyhow!(
            "final Delete authority did not preserve and independently observe identity/generation/lock state"
        ));
    }
    Ok(())
}

fn expected_prestate(selector: UnmapSelector) -> Option<Prestate> {
    use UnmapSelector as S;
    match selector {
        S::FinalDeleteAuthMainIdentityMissing => Some(Prestate::MissingIdentity),
        S::FinalDeleteAuthMainOrGenerationMismatch => Some(Prestate::IdentityMismatch),
        S::FinalDeleteAuthLockStateUncertain => Some(Prestate::LockQueryUnavailable),
        S::FinalDeleteSuccessNotFound => Some(Prestate::NotFound),
        _ => None,
    }
}

fn is_authorization(selector: UnmapSelector) -> bool {
    matches!(
        selector,
        UnmapSelector::FinalDeleteAuthMainIdentityMissing
            | UnmapSelector::FinalDeleteAuthMainOrGenerationMismatch
            | UnmapSelector::FinalDeleteAuthMainNotExclusive
            | UnmapSelector::FinalDeleteAuthLockStateUncertain
    )
}

fn is_detach_or_completion(selector: UnmapSelector) -> bool {
    matches!(
        selector,
        UnmapSelector::FinalKeepDetachBefore
            | UnmapSelector::FinalKeepDetachAfterKnown
            | UnmapSelector::FinalKeepDetachAfterUncertain
            | UnmapSelector::FinalKeepCompletionNativeUncertain
            | UnmapSelector::FinalDeleteDetachBefore
            | UnmapSelector::FinalDeleteDetachAfterKnown
            | UnmapSelector::FinalDeleteDetachAfterUncertain
            | UnmapSelector::FinalDeleteCompletionNativeUncertain
    )
}
