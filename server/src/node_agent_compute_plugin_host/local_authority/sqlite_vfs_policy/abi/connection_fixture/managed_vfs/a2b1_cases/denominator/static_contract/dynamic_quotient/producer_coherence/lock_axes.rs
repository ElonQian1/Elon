use super::super::super::terminal_descriptor::{
    LockActionV1, LockAxesV1, LockOperationV1, LockPrestateV1, LockTerminalDescriptorV1,
    OccurrenceV1, PrestateV1, ReachabilityV1, SourceSiteV1,
};
use super::super::projector::{ProjectionErrorV1, ProjectionViolationV1};
use super::invalid;

pub(super) fn validate(value: LockTerminalDescriptorV1) -> Result<(), ProjectionErrorV1> {
    if value.occurrence != OccurrenceV1::Natural {
        return Err(invalid(ProjectionViolationV1::LockProducerAxesMismatch));
    }
    let PrestateV1::Lock(prestate) = value.prestate else {
        return Err(invalid(ProjectionViolationV1::LockProducerTupleMismatch));
    };
    let axes = value.axes;
    let native_acquire = value.source_site == SourceSiteV1::LockNativeAcquire
        && value.operation == LockOperationV1::NativeAcquire;
    if native_acquire != matches!(axes.initialization, ReachabilityV1::Reached(_)) {
        return Err(invalid(ProjectionViolationV1::LockProducerAxesMismatch));
    }
    let (action, mask) = match (axes.action, axes.first, axes.count, axes.mask) {
        (
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
        ) => {
            if !all_masks_not_reached(axes) {
                return Err(invalid(ProjectionViolationV1::LockProducerAxesMismatch));
            }
            return Ok(());
        }
        (
            ReachabilityV1::Reached(_),
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
            ReachabilityV1::NotReached,
        ) => {
            if !all_masks_not_reached(axes)
                || !matches!(axes.initialization, ReachabilityV1::NotReached)
            {
                return Err(invalid(ProjectionViolationV1::LockProducerAxesMismatch));
            }
            return Ok(());
        }
        (
            ReachabilityV1::Reached(action),
            ReachabilityV1::Reached(_),
            ReachabilityV1::Reached(count),
            ReachabilityV1::Reached(mask),
        ) => {
            if matches!(
                action,
                LockActionV1::LockShared | LockActionV1::UnlockShared
            ) && count != 1
            {
                return Err(invalid(ProjectionViolationV1::LockProducerAxesMismatch));
            }
            (action, mask)
        }
        _ => return Err(invalid(ProjectionViolationV1::LockProducerAxesMismatch)),
    };
    if expected_masks(prestate, action, mask) != masks(axes) {
        return Err(invalid(ProjectionViolationV1::LockProducerAxesMismatch));
    }
    Ok(())
}

fn masks(axes: LockAxesV1) -> Option<[u8; 4]> {
    let ReachabilityV1::Reached(held_shared) = axes.held_shared_mask else {
        return None;
    };
    let ReachabilityV1::Reached(held_exclusive) = axes.held_exclusive_mask else {
        return None;
    };
    let ReachabilityV1::Reached(sibling_shared) = axes.sibling_shared_mask else {
        return None;
    };
    let ReachabilityV1::Reached(sibling_exclusive) = axes.sibling_exclusive_mask else {
        return None;
    };
    Some([
        held_shared,
        held_exclusive,
        sibling_shared,
        sibling_exclusive,
    ])
}

fn expected_masks(prestate: LockPrestateV1, action: LockActionV1, mask: u8) -> Option<[u8; 4]> {
    let shared = matches!(
        action,
        LockActionV1::LockShared | LockActionV1::UnlockShared
    );
    match prestate {
        LockPrestateV1::NotReached | LockPrestateV1::StoredPoison => None,
        LockPrestateV1::NoHeldLocks => Some([0, 0, 0, 0]),
        LockPrestateV1::OwnOverlap => Some(if shared {
            [mask, 0, 0, 0]
        } else {
            [0, mask, 0, 0]
        }),
        LockPrestateV1::OwnSharedHeld => Some([mask, 0, 0, 0]),
        LockPrestateV1::OwnExclusiveHeld | LockPrestateV1::ExclusiveRangeMismatch => {
            Some([0, mask, 0, 0])
        }
        LockPrestateV1::SiblingExclusiveContention => Some([0, 0, 0, mask]),
        LockPrestateV1::SiblingAnyContention => Some([0, 0, mask, 0]),
        LockPrestateV1::SiblingSharedCoalesced => Some([
            if action == LockActionV1::UnlockShared {
                mask
            } else {
                0
            },
            0,
            mask,
            0,
        ]),
    }
}

fn all_masks_not_reached(axes: LockAxesV1) -> bool {
    matches!(axes.held_shared_mask, ReachabilityV1::NotReached)
        && matches!(axes.held_exclusive_mask, ReachabilityV1::NotReached)
        && matches!(axes.sibling_shared_mask, ReachabilityV1::NotReached)
        && matches!(axes.sibling_exclusive_mask, ReachabilityV1::NotReached)
}
