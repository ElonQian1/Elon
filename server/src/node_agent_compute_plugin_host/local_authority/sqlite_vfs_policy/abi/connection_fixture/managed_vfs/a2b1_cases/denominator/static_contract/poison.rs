//! Canonical production stored-poison projection shared by Map and Lock.

use std::collections::BTreeSet;

use super::model::MutationState;
use super::terminal_descriptor::PhaseV1;

mod mutex_absence;

pub(super) use mutex_absence::{
    coordinator_mutex_poison_proof, owner_mutex_poison_proof, validate_mutex_poison_absence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct StoredPoisonCell {
    pub(super) phase: &'static str,
    pub(super) typed_phase: PhaseV1,
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
    cell(PhaseV1::Gate, MutationState::None, false),
    cell(PhaseV1::FileClose, MutationState::None, false),
    cell(PhaseV1::ExactSiblingDelete, MutationState::None, false),
    cell(PhaseV1::ExactSiblingOpen, MutationState::Uncertain, false),
    cell(PhaseV1::DmsTruncate, MutationState::Uncertain, false),
    cell(PhaseV1::FileClose, MutationState::Uncertain, false),
    cell(PhaseV1::ExactSiblingDelete, MutationState::Uncertain, false),
    cell(PhaseV1::FileGrow, MutationState::Uncertain, false),
    cell(PhaseV1::MappingClose, MutationState::Uncertain, false),
    cell(PhaseV1::ViewUnmap, MutationState::Uncertain, false),
    cell(PhaseV1::LockRelease, MutationState::None, true),
    cell(PhaseV1::ConnectionDetach, MutationState::None, true),
    cell(PhaseV1::DeleteAuthorization, MutationState::None, true),
    cell(PhaseV1::DmsExclusiveRelease, MutationState::Uncertain, true),
    cell(PhaseV1::DmsSharedRelease, MutationState::Uncertain, true),
];

const fn cell(
    typed_phase: PhaseV1,
    mutation: MutationState,
    lock_outcome_uncertain: bool,
) -> StoredPoisonCell {
    StoredPoisonCell {
        phase: typed_phase.static_name(),
        typed_phase,
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
