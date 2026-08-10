use std::{
    collections::BTreeMap,
    io,
    num::{NonZeroU32, NonZeroU64},
};

use super::super::types::{
    ManagedSqliteShmFailure, ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase,
};

const MAX_TEST_FAULT_STEPS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedSqliteShmTestFaultTarget {
    generation: NonZeroU64,
    connection_id: u64,
}

impl ManagedSqliteShmTestFaultTarget {
    pub(super) fn new(generation: NonZeroU64, connection_id: u64) -> Self {
        Self {
            generation,
            connection_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedSqliteShmTestFaultTiming {
    BeforeCall,
    AfterSuccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ManagedSqliteShmTestFaultStep {
    target: ManagedSqliteShmTestFaultTarget,
    phase: ManagedSqliteShmFailurePhase,
    ordinal: NonZeroU32,
    timing: ManagedSqliteShmTestFaultTiming,
    class: ManagedSqliteShmFailureClass,
}

#[derive(Default)]
pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) struct ManagedSqliteShmTestFaultController
{
    installed_target: Option<ManagedSqliteShmTestFaultTarget>,
    pending: Vec<ManagedSqliteShmTestFaultStep>,
    triggered: Vec<ManagedSqliteShmTestFaultStep>,
    observed_ordinals: BTreeMap<(NonZeroU64, u64, usize), u32>,
}

impl ManagedSqliteShmTestFaultController {
    pub(super) fn install(
        &mut self,
        target: ManagedSqliteShmTestFaultTarget,
        before_call: &[(ManagedSqliteShmFailurePhase, u32)],
        after_success: &[(
            ManagedSqliteShmFailurePhase,
            u32,
            ManagedSqliteShmFailureClass,
        )],
    ) -> Result<(), &'static str> {
        if self.installed_target.is_some() {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_SCRIPT_ALREADY_INSTALLED");
        }
        if target.connection_id == 0 {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_CONNECTION_ZERO");
        }
        let count = before_call
            .len()
            .checked_add(after_success.len())
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_SCRIPT_SIZE_INVALID")?;
        if count == 0 || count > MAX_TEST_FAULT_STEPS {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_SCRIPT_SIZE_INVALID");
        }

        let mut pending = Vec::with_capacity(count);
        for &(phase, ordinal) in before_call {
            push_step(
                &mut pending,
                target,
                phase,
                ordinal,
                ManagedSqliteShmTestFaultTiming::BeforeCall,
                ManagedSqliteShmFailureClass::IoBeforeMutation,
            )?;
        }
        for &(phase, ordinal, class) in after_success {
            if !matches!(
                class,
                ManagedSqliteShmFailureClass::MutatedButKnown
                    | ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned
            ) {
                return Err("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_AFTER_CLASS_UNSUPPORTED");
            }
            push_step(
                &mut pending,
                target,
                phase,
                ordinal,
                ManagedSqliteShmTestFaultTiming::AfterSuccess,
                class,
            )?;
        }
        self.installed_target = Some(target);
        self.pending = pending;
        Ok(())
    }

    pub(super) fn observe(
        &mut self,
        target: ManagedSqliteShmTestFaultTarget,
        phase: ManagedSqliteShmFailurePhase,
    ) -> Result<Option<ManagedSqliteShmMatchedTestFault>, &'static str> {
        if self.installed_target != Some(target)
            || !self
                .pending
                .iter()
                .any(|step| step.target == target && step.phase == phase)
        {
            return Ok(None);
        }
        let index =
            phase_index(phase).ok_or("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_PHASE_UNSUPPORTED")?;
        let key = (target.generation, target.connection_id, index);
        let ordinal = self
            .observed_ordinals
            .get(&key)
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_ORDINAL_EXHAUSTED")?;
        self.observed_ordinals.insert(key, ordinal);
        let ordinal =
            NonZeroU32::new(ordinal).ok_or("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_ORDINAL_ZERO")?;
        Ok(self
            .pending
            .iter()
            .copied()
            .find(|step| step.target == target && step.phase == phase && step.ordinal == ordinal)
            .map(ManagedSqliteShmMatchedTestFault))
    }

    pub(super) fn activate(
        &mut self,
        matched: ManagedSqliteShmMatchedTestFault,
        known_mutation: bool,
    ) -> Result<ManagedSqliteShmTriggeredTestFault, &'static str> {
        if !matched.is_before_call() && !known_mutation {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_AFTER_WITHOUT_MUTATION");
        }
        let index = self
            .pending
            .iter()
            .position(|step| *step == matched.0)
            .ok_or("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_MATCH_DISAPPEARED")?;
        let step = self.pending.remove(index);
        self.triggered.push(step);
        Ok(ManagedSqliteShmTriggeredTestFault(step))
    }

    pub(super) fn pending_count(&self, target: ManagedSqliteShmTestFaultTarget) -> usize {
        self.pending
            .iter()
            .filter(|step| step.target == target)
            .count()
    }

    pub(super) fn was_triggered(
        &self,
        target: ManagedSqliteShmTestFaultTarget,
        phase: ManagedSqliteShmFailurePhase,
        ordinal: u32,
    ) -> bool {
        let Some(ordinal) = NonZeroU32::new(ordinal) else {
            return false;
        };
        self.triggered
            .iter()
            .any(|step| step.target == target && step.phase == phase && step.ordinal == ordinal)
    }
}

#[derive(Clone, Copy)]
pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) struct ManagedSqliteShmMatchedTestFault(
    ManagedSqliteShmTestFaultStep,
);

impl ManagedSqliteShmMatchedTestFault {
    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn is_before_call(self) -> bool {
        self.0.timing == ManagedSqliteShmTestFaultTiming::BeforeCall
    }

    pub(super) fn phase(self) -> ManagedSqliteShmFailurePhase {
        self.0.phase
    }
}

pub(super) struct ManagedSqliteShmTriggeredTestFault(ManagedSqliteShmTestFaultStep);

impl ManagedSqliteShmTriggeredTestFault {
    pub(super) fn into_failure(self, known_mutation: bool) -> ManagedSqliteShmFailure {
        let phase = self.0.phase;
        match (self.0.timing, self.0.class) {
            (ManagedSqliteShmTestFaultTiming::BeforeCall, _) if known_mutation => {
                ManagedSqliteShmFailure::new(
                    phase,
                    ManagedSqliteShmFailureClass::MutatedButKnown,
                    io::Error::other(
                        "NODE_MANAGED_SQLITE_SHM_TEST_FAULT_BEFORE_WITH_PRIOR_MUTATION",
                    ),
                )
            }
            (
                ManagedSqliteShmTestFaultTiming::BeforeCall,
                ManagedSqliteShmFailureClass::IoBeforeMutation,
            ) => ManagedSqliteShmFailure::new(
                phase,
                ManagedSqliteShmFailureClass::IoBeforeMutation,
                io::Error::other("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_BEFORE_MUTATION"),
            ),
            (
                ManagedSqliteShmTestFaultTiming::AfterSuccess,
                ManagedSqliteShmFailureClass::MutatedButKnown,
            ) => ManagedSqliteShmFailure::new(
                phase,
                ManagedSqliteShmFailureClass::MutatedButKnown,
                io::Error::other("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_AFTER_KNOWN_MUTATION"),
            ),
            (
                ManagedSqliteShmTestFaultTiming::AfterSuccess,
                ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned,
            ) => ManagedSqliteShmFailure::poisoned(
                phase,
                io::Error::other("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_AFTER_OUTCOME_UNCERTAIN"),
                true,
                phase == ManagedSqliteShmFailurePhase::DmsSharedRelease,
            ),
            _ => ManagedSqliteShmFailure::poisoned_code(
                phase,
                "NODE_MANAGED_SQLITE_SHM_TEST_FAULT_CONTRACT_INVALID",
                known_mutation,
                false,
            ),
        }
    }
}

fn push_step(
    pending: &mut Vec<ManagedSqliteShmTestFaultStep>,
    target: ManagedSqliteShmTestFaultTarget,
    phase: ManagedSqliteShmFailurePhase,
    ordinal: u32,
    timing: ManagedSqliteShmTestFaultTiming,
    class: ManagedSqliteShmFailureClass,
) -> Result<(), &'static str> {
    phase_index(phase).ok_or("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_PHASE_UNSUPPORTED")?;
    let ordinal =
        NonZeroU32::new(ordinal).ok_or("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_ORDINAL_ZERO")?;
    let step = ManagedSqliteShmTestFaultStep {
        target,
        phase,
        ordinal,
        timing,
        class,
    };
    if pending.iter().any(|candidate| {
        candidate.target == target && candidate.phase == phase && candidate.ordinal == ordinal
    }) {
        return Err("NODE_MANAGED_SQLITE_SHM_TEST_FAULT_TARGET_DUPLICATE");
    }
    pending.push(step);
    Ok(())
}

fn phase_index(phase: ManagedSqliteShmFailurePhase) -> Option<usize> {
    match phase {
        ManagedSqliteShmFailurePhase::ViewUnmap => Some(0),
        ManagedSqliteShmFailurePhase::MappingClose => Some(1),
        ManagedSqliteShmFailurePhase::DmsSharedRelease => Some(2),
        ManagedSqliteShmFailurePhase::FileClose => Some(3),
        _ => None,
    }
}
