//! Physical-SHM-only actual bridge for final Keep/ViewUnmap/AfterSuccessKnown.
//!
//! This deliberately does not construct a full dynamic `Case`: SQLite connection, registry,
//! logical-name, main-file, lease, callback and action-count facts are outside this observer.

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;
use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestTargetSnapshot,
};

use super::{
    invariants,
    model::{
        CallbackKind, Case, DmsCustody, EvidenceKind, FailureClass, NodePrecondition, Path, Phase,
        SqliteOutcome, TargetScope, Timing, TopologyKind, UnmapMode,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ViewUnmapPhysicalSubsetActual {
    pub(in super::super) callback_result_code: i32,
    pub(in super::super) pre: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) post: ManagedSqliteShmTestTargetSnapshot,
    pub(in super::super) pending_before: u8,
    pub(in super::super) pending_after: u8,
    pub(in super::super) triggered_before: bool,
    pub(in super::super) triggered_after: bool,
}

pub(in super::super) fn validate_view_unmap_after_success_physical_subset(
    actual: ViewUnmapPhysicalSubsetActual,
) -> Result<(), &'static str> {
    let inventory = invariants::inventory();
    invariants::validate(&inventory)?;
    let expected = select_frozen_case(&inventory)?;
    validate_static_boundary(expected)?;

    check(
        actual.callback_result_code,
        rusqlite::ffi::SQLITE_IOERR,
        "direct xShmUnmap callback result",
    )?;
    validate_pre(actual.pre, expected)?;
    validate_post(actual.post, expected)?;
    check(actual.pending_before, 1, "pre fault pending count")?;
    check(
        actual.pending_after,
        expected.counts.fault_pending,
        "post fault pending count",
    )?;
    check(actual.triggered_before, false, "pre fault trigger state")?;
    check(
        actual.triggered_after,
        expected.counts.fault_trigger == 1,
        "post fault trigger state",
    )
}

fn select_frozen_case(inventory: &[Case]) -> Result<&Case, &'static str> {
    let mut selected = inventory.iter().filter(|case| {
        case.path == Path::Unmap
            && case.topology_kind == TopologyKind::FinalConnection
            && case.unmap_mode == UnmapMode::Keep
            && case.phase == Phase::ViewUnmap
            && case.timing == Timing::AfterSuccessKnown
            && case.class == FailureClass::MutatedButKnown
            && case.node_precondition == NodePrecondition::Live
            && case.cause_phase.is_none()
            && case.target.scope == TargetScope::RouteMain
            && case.target.role == Some(ManagedSqliteLogicalFileRole::Main)
            && case.target.callback == Some(CallbackKind::Shm)
            && case.target.occurrence == 1
            && case.variant == 0
            && case.pre_shared_mask == 0
            && case.pre_exclusive_mask == 0
    });
    let case = selected.next().ok_or("frozen ViewUnmap case missing")?;
    if selected.next().is_some() {
        return Err("frozen ViewUnmap case is not unique");
    }
    Ok(case)
}

fn validate_static_boundary(expected: &Case) -> Result<(), &'static str> {
    check(
        expected.evidence,
        EvidenceKind::StaticContract,
        "frozen evidence kind",
    )?;
    check(
        expected.sqlite_outcome,
        SqliteOutcome::Ioerr,
        "frozen SQLite outcome",
    )?;
    check(
        expected.counts.fault_observe,
        1,
        "frozen fault observation count",
    )?;
    check(
        expected.counts.fault_trigger,
        1,
        "frozen fault trigger count",
    )?;
    check(
        expected.counts.fault_pending,
        0,
        "frozen fault pending count",
    )?;
    check(
        expected.pre.shm_connections,
        1,
        "frozen pre SHM connections",
    )?;
    check(
        expected.post.shm_connections,
        1,
        "frozen post SHM connections",
    )?;
    check(expected.retained.views, 0, "frozen retained view count")?;
    check(
        expected.retained.mappings,
        1,
        "frozen retained mapping count",
    )?;
    check(
        expected.retained.dms,
        DmsCustody::Shared,
        "frozen DMS custody",
    )?;
    check(
        expected.mutation_may_have_occurred,
        true,
        "frozen mutation bit",
    )?;
    check(
        expected.lock_outcome_uncertain,
        false,
        "frozen lock uncertainty",
    )?;
    check(expected.domain_terminal, true, "frozen domain terminal bit")
}

fn validate_pre(
    actual: ManagedSqliteShmTestTargetSnapshot,
    expected: &Case,
) -> Result<(), &'static str> {
    check(
        actual.topology.shm_connections,
        expected.pre.shm_connections,
        "pre SHM connections",
    )?;
    check(actual.target_attached, true, "pre target attachment")?;
    check(
        actual.shared_mask,
        expected.pre_shared_mask,
        "pre shared mask",
    )?;
    check(
        actual.exclusive_mask,
        expected.pre_exclusive_mask,
        "pre exclusive mask",
    )?;
    check(actual.topology.node_present, true, "pre node presence")?;
    check(actual.topology.views, 1, "pre view count")?;
    check(actual.topology.mappings, 1, "pre mapping count")?;
    check(
        actual.topology.dms,
        ManagedSqliteShmTestDmsCustody::Shared,
        "pre DMS custody",
    )?;
    check(
        actual.topology.shm_file_present,
        true,
        "pre SHM-file custody",
    )?;
    check(actual.topology.poisoned, false, "pre poison bit")?;
    check(
        actual.topology.mutation_may_have_occurred,
        false,
        "pre mutation bit",
    )?;
    check(
        actual.topology.lock_outcome_uncertain,
        false,
        "pre lock uncertainty",
    )?;
    check(
        actual.topology.domain_terminal,
        false,
        "pre domain terminal bit",
    )?;
    check(
        actual.topology.quarantined_file_closes,
        0,
        "pre quarantined closes",
    )
}

fn validate_post(
    actual: ManagedSqliteShmTestTargetSnapshot,
    expected: &Case,
) -> Result<(), &'static str> {
    check(
        actual.topology.shm_connections,
        expected.post.shm_connections,
        "post SHM connections",
    )?;
    check(actual.target_attached, true, "post target remains attached")?;
    check(
        actual.shared_mask,
        expected.pre_shared_mask,
        "post shared mask",
    )?;
    check(
        actual.exclusive_mask,
        expected.pre_exclusive_mask,
        "post exclusive mask",
    )?;
    check(
        actual.topology.node_present,
        expected.retained.node,
        "post node presence",
    )?;
    check(
        actual.topology.views,
        u16::from(expected.retained.views),
        "post view count",
    )?;
    check(
        actual.topology.mappings,
        u16::from(expected.retained.mappings),
        "post mapping count",
    )?;
    check(
        actual.topology.dms,
        ManagedSqliteShmTestDmsCustody::Shared,
        "post DMS custody",
    )?;
    check(
        actual.topology.shm_file_present,
        expected.retained.shm_file,
        "post SHM-file custody",
    )?;
    check(actual.topology.poisoned, true, "post poison bit")?;
    check(
        actual.topology.mutation_may_have_occurred,
        expected.mutation_may_have_occurred,
        "post mutation bit",
    )?;
    check(
        actual.topology.lock_outcome_uncertain,
        expected.lock_outcome_uncertain,
        "post lock uncertainty",
    )?;
    check(
        actual.topology.domain_terminal,
        expected.domain_terminal,
        "post domain terminal bit",
    )?;
    check(
        actual.topology.quarantined_file_closes,
        0,
        "post quarantined closes",
    )
}

fn check<T: PartialEq>(actual: T, expected: T, message: &'static str) -> Result<(), &'static str> {
    if actual == expected {
        Ok(())
    } else {
        Err(message)
    }
}
