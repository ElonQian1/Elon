//! Exact occurrence ledger written at the real connection-detach call boundary.

use super::{
    ManagedSqliteShmFailurePhase as Phase, ManagedSqliteShmTestUnmapActionEvent as Event,
    ManagedSqliteShmTestUnmapActionOutcome as Outcome,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestConnectionDetachReceipt {
    pub(crate) events: Vec<Event>,
}

#[derive(Default)]
pub(super) struct ManagedSqliteShmTestConnectionDetachControl {
    events: Vec<Event>,
}

impl ManagedSqliteShmTestConnectionDetachControl {
    pub(super) fn record(&mut self, outcome: Outcome) -> Result<(), &'static str> {
        let valid = match (self.events.as_slice(), outcome) {
            ([], Outcome::Attempt) => true,
            ([attempt], Outcome::Success) => {
                attempt.phase == Phase::ConnectionDetach
                    && attempt.outcome == Outcome::Attempt
                    && attempt.ordinal == 1
            }
            _ => false,
        };
        if !valid {
            return Err("NODE_MANAGED_SQLITE_SHM_TEST_CONNECTION_DETACH_OCCURRENCE_INVALID");
        }
        self.events.push(Event {
            phase: Phase::ConnectionDetach,
            outcome,
            ordinal: 1,
        });
        Ok(())
    }

    pub(super) fn receipt(&self) -> ManagedSqliteShmTestConnectionDetachReceipt {
        ManagedSqliteShmTestConnectionDetachReceipt {
            events: self.events.clone(),
        }
    }
}
