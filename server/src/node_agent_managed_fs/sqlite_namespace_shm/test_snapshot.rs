//! Redacted physical SHM custody observations for Windows-only VFS fixtures.

use super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState, ManagedSqliteShmDmsCustody,
        PinnedManagedSqliteWalRuntime,
    },
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailurePhase},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedSqliteShmTestDmsCustody {
    Absent,
    Shared,
    SharedOutcomeUncertain,
    ExclusiveKnown,
    ExclusiveOutcomeUncertain,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestTopologySnapshot {
    pub(crate) shm_connections: u8,
    pub(crate) node_present: bool,
    pub(crate) views: u16,
    pub(crate) mappings: u16,
    pub(crate) dms: ManagedSqliteShmTestDmsCustody,
    pub(crate) shm_file_present: bool,
    pub(crate) poisoned: bool,
    pub(crate) mutation_may_have_occurred: bool,
    pub(crate) lock_outcome_uncertain: bool,
    pub(crate) domain_terminal: bool,
    pub(crate) quarantined_file_closes: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestTargetSnapshot {
    pub(crate) topology: ManagedSqliteShmTestTopologySnapshot,
    pub(crate) target_attached: bool,
    pub(crate) shared_mask: u8,
    pub(crate) exclusive_mask: u8,
}

struct ManagedSqliteShmTestStateSnapshot {
    topology: ManagedSqliteShmTestTopologySnapshot,
    target_masks: Option<(u8, u8)>,
}

impl PinnedManagedSqliteWalRuntime {
    /// Observes coordinator state first and the domain registry second. The result is a
    /// sequential diagnostic observation, not an atomic snapshot across those authorities.
    pub(crate) fn test_topology_snapshot(
        &self,
    ) -> Result<ManagedSqliteShmTestTopologySnapshot, ManagedSqliteShmFailure> {
        test_topology_snapshot(&self.coordinator)
    }
}

fn test_topology_snapshot(
    coordinator: &ManagedSqliteShmCoordinator,
) -> Result<ManagedSqliteShmTestTopologySnapshot, ManagedSqliteShmFailure> {
    let state = coordinator
        .state
        .lock()
        .map_err(|_| snapshot_state_poisoned())?;
    let snapshot = copy_state_snapshot(&state, None);
    drop(state);
    Ok(finish_state_snapshot(coordinator, snapshot)?.topology)
}

pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn test_target_snapshot<F>(
    coordinator: &ManagedSqliteShmCoordinator,
    mut target_matches: F,
) -> Result<ManagedSqliteShmTestTargetSnapshot, ManagedSqliteShmFailure>
where
    F: FnMut(u64) -> bool,
{
    let state = coordinator
        .state
        .lock()
        .map_err(|_| snapshot_state_poisoned())?;
    let target_connection_id = state
        .connections
        .keys()
        .copied()
        .find(|connection_id| target_matches(*connection_id));
    let snapshot = copy_state_snapshot(&state, target_connection_id);
    drop(state);
    let snapshot = finish_state_snapshot(coordinator, snapshot)?;
    let (target_attached, shared_mask, exclusive_mask) = match snapshot.target_masks {
        Some((shared_mask, exclusive_mask)) => (true, shared_mask, exclusive_mask),
        None => (false, 0, 0),
    };
    Ok(ManagedSqliteShmTestTargetSnapshot {
        topology: snapshot.topology,
        target_attached,
        shared_mask,
        exclusive_mask,
    })
}

fn finish_state_snapshot(
    coordinator: &ManagedSqliteShmCoordinator,
    snapshot: Result<ManagedSqliteShmTestStateSnapshot, ManagedSqliteShmFailure>,
) -> Result<ManagedSqliteShmTestStateSnapshot, ManagedSqliteShmFailure> {
    let mut snapshot = snapshot?;
    snapshot.topology.domain_terminal = coordinator.test_domain_terminal()?;
    Ok(snapshot)
}

fn copy_state_snapshot(
    state: &ManagedSqliteShmCoordinatorState,
    target_connection_id: Option<u64>,
) -> Result<ManagedSqliteShmTestStateSnapshot, ManagedSqliteShmFailure> {
    let shm_connections = checked_u8(
        state.connections.len(),
        "NODE_MANAGED_SQLITE_SHM_TEST_CONNECTION_COUNT_OVERFLOW",
    )?;
    let (node_present, views, mappings, dms) = match state.node.as_ref() {
        Some(node) => (
            true,
            checked_u16(
                node.regions
                    .iter()
                    .filter(|region| {
                        region
                            .view
                            .as_ref()
                            .is_some_and(|view| view.test_custody_present())
                    })
                    .count(),
                "NODE_MANAGED_SQLITE_SHM_TEST_VIEW_COUNT_OVERFLOW",
            )?,
            checked_u16(
                node.regions
                    .iter()
                    .filter(|region| region.mapping.test_custody_present())
                    .count(),
                "NODE_MANAGED_SQLITE_SHM_TEST_MAPPING_COUNT_OVERFLOW",
            )?,
            test_dms_custody(node.dms),
        ),
        None => (false, 0, 0, ManagedSqliteShmTestDmsCustody::Absent),
    };
    let (poisoned, mutation_may_have_occurred, lock_outcome_uncertain) = match state.poisoned {
        Some(poison) => (
            true,
            poison.mutation_may_have_occurred,
            poison.lock_outcome_uncertain,
        ),
        None => (false, false, false),
    };
    let quarantined_file_closes = checked_u16(
        state.quarantined_file_close.len(),
        "NODE_MANAGED_SQLITE_SHM_TEST_QUARANTINED_CLOSE_COUNT_OVERFLOW",
    )?;
    let shm_file_present = node_present || quarantined_file_closes != 0;
    let target_masks = target_connection_id.and_then(|connection_id| {
        state
            .connections
            .get(&connection_id)
            .map(|connection| (connection.shared_mask, connection.exclusive_mask))
    });
    Ok(ManagedSqliteShmTestStateSnapshot {
        topology: ManagedSqliteShmTestTopologySnapshot {
            shm_connections,
            node_present,
            views,
            mappings,
            dms,
            shm_file_present,
            poisoned,
            mutation_may_have_occurred,
            lock_outcome_uncertain,
            domain_terminal: false,
            quarantined_file_closes,
        },
        target_masks,
    })
}

fn test_dms_custody(custody: ManagedSqliteShmDmsCustody) -> ManagedSqliteShmTestDmsCustody {
    match custody {
        ManagedSqliteShmDmsCustody::Shared => ManagedSqliteShmTestDmsCustody::Shared,
        ManagedSqliteShmDmsCustody::SharedOutcomeUncertain => {
            ManagedSqliteShmTestDmsCustody::SharedOutcomeUncertain
        }
        ManagedSqliteShmDmsCustody::ExclusiveKnown => {
            ManagedSqliteShmTestDmsCustody::ExclusiveKnown
        }
        ManagedSqliteShmDmsCustody::ExclusiveOutcomeUncertain => {
            ManagedSqliteShmTestDmsCustody::ExclusiveOutcomeUncertain
        }
        ManagedSqliteShmDmsCustody::Released => ManagedSqliteShmTestDmsCustody::Released,
    }
}

fn checked_u8(value: usize, code: &'static str) -> Result<u8, ManagedSqliteShmFailure> {
    u8::try_from(value).map_err(|_| snapshot_overflow(code))
}

fn checked_u16(value: usize, code: &'static str) -> Result<u16, ManagedSqliteShmFailure> {
    u16::try_from(value).map_err(|_| snapshot_overflow(code))
}

fn snapshot_overflow(code: &'static str) -> ManagedSqliteShmFailure {
    ManagedSqliteShmFailure::poisoned_code(ManagedSqliteShmFailurePhase::Gate, code, false, false)
}

fn snapshot_state_poisoned() -> ManagedSqliteShmFailure {
    ManagedSqliteShmFailure::poisoned_code(
        ManagedSqliteShmFailurePhase::Gate,
        "NODE_MANAGED_SQLITE_SHM_TEST_SNAPSHOT_STATE_POISONED",
        false,
        false,
    )
}
