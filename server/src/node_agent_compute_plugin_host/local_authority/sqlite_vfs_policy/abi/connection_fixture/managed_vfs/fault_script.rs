//! Fixture-owned, one-shot fault selection for test VFS file callbacks.
//!
//! This wrapper admits only `BeforeCall`. Managed-fs runtime scripts own platform teardown phases;
//! a future after-success callback fault needs an exact route/domain terminal seam first.

use std::{
    collections::HashMap,
    fmt,
    num::{NonZeroU32, NonZeroU64},
    sync::Mutex,
};

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

mod file;

pub(super) use file::ManagedTestFaultingFile;

const MAX_FAULT_STEPS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ManagedTestRouteOrdinal(NonZeroU64);

impl ManagedTestRouteOrdinal {
    pub(super) fn from_counter(value: u64) -> Result<Self, ()> {
        NonZeroU64::new(value).map(Self).ok_or(())
    }

    pub(super) fn test_value(value: u64) -> Self {
        Self(NonZeroU64::new(value).expect("fault-matrix route ordinal must be non-zero"))
    }
}

impl fmt::Debug for ManagedTestRouteOrdinal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ManagedTestRouteOrdinal(<opaque>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ManagedTestCallbackFaultOperation {
    ShmMap,
    ShmLock,
    ShmBarrier,
    ShmUnmap,
    FileClose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedTestCallbackFaultTiming {
    BeforeCall,
    AfterSuccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestCallbackFaultStep {
    route: ManagedTestRouteOrdinal,
    role: ManagedSqliteLogicalFileRole,
    operation: ManagedTestCallbackFaultOperation,
    occurrence: NonZeroU32,
    timing: ManagedTestCallbackFaultTiming,
}

impl ManagedTestCallbackFaultStep {
    pub(super) fn new(
        route: ManagedTestRouteOrdinal,
        role: ManagedSqliteLogicalFileRole,
        operation: ManagedTestCallbackFaultOperation,
        occurrence: u32,
        timing: ManagedTestCallbackFaultTiming,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            route,
            role,
            operation,
            occurrence: NonZeroU32::new(occurrence).ok_or("fault occurrence must be non-zero")?,
            timing,
        })
    }

    fn key(self) -> ManagedTestCallbackFaultKey {
        ManagedTestCallbackFaultKey {
            route: self.route,
            role: role_tag(self.role),
            operation: self.operation,
        }
    }

    fn same_occurrence(self, other: Self) -> bool {
        self.key() == other.key() && self.occurrence == other.occurrence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestCallbackFaultObservation {
    step: ManagedTestCallbackFaultStep,
}

impl ManagedTestCallbackFaultObservation {
    pub(super) fn step(self) -> ManagedTestCallbackFaultStep {
        self.step
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ManagedTestCallbackFaultKey {
    route: ManagedTestRouteOrdinal,
    role: u8,
    operation: ManagedTestCallbackFaultOperation,
}

struct ManagedTestCallbackFaultState {
    step: ManagedTestCallbackFaultStep,
    consumed: bool,
}

struct ManagedTestInstalledCallbackFaults {
    steps: Vec<ManagedTestCallbackFaultState>,
    occurrences: HashMap<ManagedTestCallbackFaultKey, u32>,
    observations: Vec<ManagedTestCallbackFaultObservation>,
}

enum ManagedTestCallbackFaultInstallation {
    Empty,
    Installed(ManagedTestInstalledCallbackFaults),
}

pub(super) struct ManagedTestCallbackFaultController {
    installation: Mutex<ManagedTestCallbackFaultInstallation>,
}

impl ManagedTestCallbackFaultController {
    pub(super) fn new() -> Self {
        Self {
            installation: Mutex::new(ManagedTestCallbackFaultInstallation::Empty),
        }
    }

    pub(super) fn install(
        &self,
        steps: &[ManagedTestCallbackFaultStep],
    ) -> Result<(), &'static str> {
        let mut installation = self
            .installation
            .lock()
            .map_err(|_| "callback fault controller poisoned")?;
        if !matches!(&*installation, ManagedTestCallbackFaultInstallation::Empty) {
            return Err("callback fault script already installed");
        }
        if steps.is_empty() || steps.len() > MAX_FAULT_STEPS {
            return Err("callback fault script must contain 1..=32 steps");
        }
        if steps
            .iter()
            .any(|step| step.timing != ManagedTestCallbackFaultTiming::BeforeCall)
        {
            return Err("after-success callback faults require a terminal route/domain seam");
        }
        for (index, step) in steps.iter().copied().enumerate() {
            if steps[..index]
                .iter()
                .copied()
                .any(|prior| step.same_occurrence(prior))
            {
                return Err("duplicate callback fault occurrence");
            }
        }
        *installation =
            ManagedTestCallbackFaultInstallation::Installed(ManagedTestInstalledCallbackFaults {
                steps: steps
                    .iter()
                    .copied()
                    .map(|step| ManagedTestCallbackFaultState {
                        step,
                        consumed: false,
                    })
                    .collect(),
                occurrences: HashMap::new(),
                observations: Vec::new(),
            });
        Ok(())
    }

    pub(super) fn begin_operation(
        &self,
        route: ManagedTestRouteOrdinal,
        role: ManagedSqliteLogicalFileRole,
        operation: ManagedTestCallbackFaultOperation,
    ) -> Result<bool, ()> {
        let key = ManagedTestCallbackFaultKey {
            route,
            role: role_tag(role),
            operation,
        };
        let mut installation = self.installation.lock().map_err(|_| ())?;
        let ManagedTestCallbackFaultInstallation::Installed(installed) = &mut *installation else {
            return Ok(false);
        };
        if !installed.steps.iter().any(|state| state.step.key() == key) {
            return Ok(false);
        }
        let occurrence = installed.occurrences.entry(key).or_insert(0);
        *occurrence = (*occurrence).checked_add(1).ok_or(())?;
        let occurrence = *occurrence;
        Ok(trigger(
            installed,
            key,
            occurrence,
            ManagedTestCallbackFaultTiming::BeforeCall,
        ))
    }

    pub(super) fn pending_count(&self) -> Result<usize, &'static str> {
        let installation = self
            .installation
            .lock()
            .map_err(|_| "callback fault controller poisoned")?;
        match &*installation {
            ManagedTestCallbackFaultInstallation::Empty => Ok(0),
            ManagedTestCallbackFaultInstallation::Installed(installed) => {
                Ok(installed.steps.iter().filter(|step| !step.consumed).count())
            }
        }
    }

    pub(super) fn observations(
        &self,
    ) -> Result<Vec<ManagedTestCallbackFaultObservation>, &'static str> {
        let installation = self
            .installation
            .lock()
            .map_err(|_| "callback fault controller poisoned")?;
        match &*installation {
            ManagedTestCallbackFaultInstallation::Empty => Ok(Vec::new()),
            ManagedTestCallbackFaultInstallation::Installed(installed) => {
                Ok(installed.observations.clone())
            }
        }
    }
}

fn trigger(
    installed: &mut ManagedTestInstalledCallbackFaults,
    key: ManagedTestCallbackFaultKey,
    occurrence: u32,
    timing: ManagedTestCallbackFaultTiming,
) -> bool {
    let Some(state) = installed.steps.iter_mut().find(|state| {
        !state.consumed
            && state.step.key() == key
            && state.step.occurrence.get() == occurrence
            && state.step.timing == timing
    }) else {
        return false;
    };
    let step = state.step;
    state.consumed = true;
    installed
        .observations
        .push(ManagedTestCallbackFaultObservation { step });
    true
}

fn role_tag(role: ManagedSqliteLogicalFileRole) -> u8 {
    match role {
        ManagedSqliteLogicalFileRole::Main => 1,
        ManagedSqliteLogicalFileRole::Journal => 2,
        ManagedSqliteLogicalFileRole::Wal => 3,
    }
}
