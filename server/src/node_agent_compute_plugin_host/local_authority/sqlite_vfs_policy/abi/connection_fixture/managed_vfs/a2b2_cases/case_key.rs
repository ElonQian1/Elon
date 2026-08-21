use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

use super::model::{
    CallbackKind, Case, FailureClass, NodePrecondition, Path, Phase, TargetScope, Timing,
    TopologyKind, UnmapMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in super::super) struct CaseKey {
    pub(super) path: Path,
    pub(super) topology: TopologyKind,
    pub(super) unmap_mode: UnmapMode,
    pub(super) node: NodePrecondition,
    pub(super) variant: u8,
    pub(super) shared_mask: u8,
    pub(super) exclusive_mask: u8,
    pub(super) phase: Phase,
    pub(super) cause: Option<Phase>,
    pub(super) timing: Timing,
    pub(super) class: FailureClass,
    pub(super) scope: TargetScope,
    pub(super) registration_id: u64,
    pub(super) route_ordinal: u64,
    pub(super) runtime_generation: u64,
    pub(super) shm_connection_id: u64,
    pub(super) role: u8,
    pub(super) callback: Option<CallbackKind>,
    pub(super) occurrence: u32,
}

impl CaseKey {
    pub(super) fn expected(
        path: Path,
        topology: TopologyKind,
        unmap_mode: UnmapMode,
        phase: Phase,
        timing: Timing,
        class: FailureClass,
        callback: Option<CallbackKind>,
    ) -> Self {
        let route_scoped = path != Path::RegistrationShutdown;
        Self {
            path,
            topology,
            unmap_mode,
            node: NodePrecondition::Live,
            variant: 0,
            shared_mask: 0,
            exclusive_mask: 0,
            phase,
            cause: None,
            timing,
            class,
            scope: if route_scoped {
                TargetScope::RouteMain
            } else {
                TargetScope::Registration
            },
            registration_id: 1,
            route_ordinal: u64::from(route_scoped),
            runtime_generation: u64::from(route_scoped),
            shm_connection_id: u64::from(route_scoped),
            role: u8::from(route_scoped),
            callback,
            occurrence: 1,
        }
    }

    pub(super) fn cause(mut self, phase: Phase) -> Self {
        self.cause = Some(phase);
        self
    }

    pub(super) fn variant(mut self, variant: u8) -> Self {
        self.variant = variant;
        self
    }

    pub(super) fn masks(mut self, shared: u8, exclusive: u8) -> Self {
        self.shared_mask = shared;
        self.exclusive_mask = exclusive;
        self
    }

    pub(super) fn node(mut self, node: NodePrecondition) -> Self {
        self.node = node;
        self
    }
}

impl From<&Case> for CaseKey {
    fn from(case: &Case) -> Self {
        Self {
            path: case.path,
            topology: case.topology_kind,
            unmap_mode: case.unmap_mode,
            node: case.node_precondition,
            variant: case.variant,
            shared_mask: case.pre_shared_mask,
            exclusive_mask: case.pre_exclusive_mask,
            phase: case.phase,
            cause: case.cause_phase,
            timing: case.timing,
            class: case.class,
            scope: case.target.scope,
            registration_id: case.target.registration_id,
            route_ordinal: case.target.route_ordinal,
            runtime_generation: case.target.runtime_generation,
            shm_connection_id: case.target.shm_connection_id,
            role: match case.target.role {
                None => 0,
                Some(ManagedSqliteLogicalFileRole::Main) => 1,
                Some(ManagedSqliteLogicalFileRole::Journal) => 2,
                Some(ManagedSqliteLogicalFileRole::Wal) => 3,
            },
            callback: case.target.callback,
            occurrence: case.target.occurrence,
        }
    }
}
