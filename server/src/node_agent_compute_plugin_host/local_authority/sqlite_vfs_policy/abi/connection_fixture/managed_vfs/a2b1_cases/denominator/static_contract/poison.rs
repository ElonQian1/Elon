//! Canonical production stored-poison projection shared by Map and Lock.

use std::collections::BTreeSet;

use super::model::MutationState;

mod mutex_absence;

pub(super) use mutex_absence::{
    coordinator_mutex_poison_proof, owner_mutex_poison_proof, validate_mutex_poison_absence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct StoredPoisonCell {
    pub(super) phase: &'static str,
    pub(super) mutation: MutationState,
    pub(super) lock_outcome_uncertain: bool,
}

impl StoredPoisonCell {
    pub(super) fn label(self) -> String {
        format!(
            "{}.mutation-{}.lock-{}",
            self.phase,
            match self.mutation {
                MutationState::None => "false",
                MutationState::Known | MutationState::Uncertain => "true",
            },
            self.lock_outcome_uncertain
        )
        .to_ascii_lowercase()
    }
}

pub(super) const STORED_POISON_CELLS: &[StoredPoisonCell] = &[
    cell("Gate", MutationState::None, false),
    cell("FileClose", MutationState::None, false),
    cell("ExactSiblingDelete", MutationState::None, false),
    cell("ExactSiblingOpen", MutationState::Uncertain, false),
    cell("DmsTruncate", MutationState::Uncertain, false),
    cell("FileClose", MutationState::Uncertain, false),
    cell("ExactSiblingDelete", MutationState::Uncertain, false),
    cell("FileGrow", MutationState::Uncertain, false),
    cell("MappingClose", MutationState::Uncertain, false),
    cell("ViewUnmap", MutationState::Uncertain, false),
    cell("LockRelease", MutationState::None, true),
    cell("ConnectionDetach", MutationState::None, true),
    cell("DeleteAuthorization", MutationState::None, true),
    cell("DmsExclusiveRelease", MutationState::Uncertain, true),
    cell("DmsSharedRelease", MutationState::Uncertain, true),
];

const fn cell(
    phase: &'static str,
    mutation: MutationState,
    lock_outcome_uncertain: bool,
) -> StoredPoisonCell {
    StoredPoisonCell {
        phase,
        mutation,
        lock_outcome_uncertain,
    }
}

pub(super) fn validate_manifest() {
    let unique = STORED_POISON_CELLS.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), 15, "stored-poison projection width drift");
    assert_eq!(
        unique.len(),
        STORED_POISON_CELLS.len(),
        "duplicate stored-poison cell"
    );
    assert!(STORED_POISON_CELLS.iter().all(|cell| {
        cell.mutation != MutationState::Known
            && (cell.lock_outcome_uncertain
                || matches!(
                    cell.mutation,
                    MutationState::None | MutationState::Uncertain
                ))
    }));
}
