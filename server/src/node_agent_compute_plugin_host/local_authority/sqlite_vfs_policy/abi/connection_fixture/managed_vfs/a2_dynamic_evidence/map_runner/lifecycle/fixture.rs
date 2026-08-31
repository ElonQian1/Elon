//! Exact WAL prestates and lower-ledger validation for positive Map lifecycles.

use std::{os::raw::c_void, path::Path};

use anyhow::anyhow;
use rusqlite::ffi;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestMapReceipt,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::{
    connection::ManagedTestShmMapCallbackObservation, ManagedSqliteMultiConnectionFixture,
};
use super::{map_expectation, MapRunnerLifecyclePathV1, REGION_SIZE, SELECTED};

const PROMOTION_REGION_SIZE_GUARD: i32 = 65_537;

#[derive(Clone, Copy)]
pub(super) struct PreparedMapPrestate {
    setup_pointer: Option<*mut c_void>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ValidatedPointerRelationV1 {
    pub(super) output_present: bool,
    pub(super) selection_present: bool,
    pub(super) selection_equal: bool,
}

pub(super) fn prepare(root: &Path) -> anyhow::Result<ManagedSqliteMultiConnectionFixture> {
    let fixture = ManagedSqliteMultiConnectionFixture::open_single(root, [0xa6; 16])?;
    let mode: String =
        fixture
            .connection(SELECTED)?
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!("Map lifecycle fixture did not enter WAL mode"));
    }
    fixture.route(SELECTED)?.into_schema_migration()?;
    fixture.route(SELECTED)?.into_runtime()?;
    if fixture
        .route(SELECTED)?
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?
    {
        return Err(anyhow!(
            "Map lifecycle target existed before explicit prestate installation"
        ));
    }
    Ok(fixture)
}

pub(super) fn install_prestate(
    fixture: &ManagedSqliteMultiConnectionFixture,
    path: MapRunnerLifecyclePathV1,
) -> anyhow::Result<PreparedMapPrestate> {
    let callback = if path.is_empty() {
        fixture
            .route(SELECTED)?
            .call_main_shm_map_raw(0, PROMOTION_REGION_SIZE_GUARD, 0)
            .map_err(anyhow::Error::msg)?
    } else {
        fixture
            .route(SELECTED)?
            .call_main_shm_map_raw(0, REGION_SIZE as i32, 1)
            .map_err(anyhow::Error::msg)?
    };
    let before = callback.before();
    let after = callback.after();
    let slots_installed = before.methods_installed
        && before.state_installed
        && after.methods_installed
        && after.state_installed;
    if path.is_empty() {
        if callback.region() != 0
            || callback.region_size() != PROMOTION_REGION_SIZE_GUARD
            || callback.raw_extend() != 0
            || callback.result_code() != ffi::SQLITE_IOERR_SHMMAP
            || !callback.output_was_cleared()
            || !callback.output_pointer().is_null()
            || !slots_installed
        {
            return Err(anyhow!(
                "Map lifecycle empty prestate target promotion mismatch"
            ));
        }
    } else if callback.region() != 0
        || callback.region_size() != REGION_SIZE as i32
        || callback.raw_extend() != 1
        || callback.result_code() != ffi::SQLITE_OK
        || callback.output_was_cleared()
        || callback.output_pointer().is_null()
        || !slots_installed
    {
        return Err(anyhow!(
            "Map lifecycle mapped prestate installation mismatch"
        ));
    }
    if !fixture
        .route(SELECTED)?
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?
    {
        return Err(anyhow!(
            "Map lifecycle exact target was not attached by prestate installation"
        ));
    }
    Ok(PreparedMapPrestate {
        setup_pointer: (!path.is_empty()).then(|| callback.output_pointer()),
    })
}

pub(super) fn validate_prestate(
    path: MapRunnerLifecyclePathV1,
    before: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<()> {
    if exact_snapshot(path, false, before) {
        Ok(())
    } else {
        Err(anyhow!("Map lifecycle exact prestate mismatch"))
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_action(
    path: MapRunnerLifecyclePathV1,
    prestate: PreparedMapPrestate,
    callback: ManagedTestShmMapCallbackObservation,
    before: ManagedSqliteShmTestTargetSnapshot,
    after: ManagedSqliteShmTestTargetSnapshot,
    receipt: ManagedSqliteShmTestMapReceipt,
) -> anyhow::Result<ValidatedPointerRelationV1> {
    validate_prestate(path, before)?;
    let output = callback.output_pointer();
    let output_present = !output.is_null();
    let selection_present = receipt.selected_pointer.is_some();
    let selection_equal = output_present
        && selection_present
        && receipt.selected_pointer_matches(output.cast::<u8>());
    let pointer_relation = ValidatedPointerRelationV1 {
        output_present,
        selection_present,
        selection_equal,
    };
    let expected_relation = if path.is_mapped() {
        ValidatedPointerRelationV1 {
            output_present: true,
            selection_present: true,
            selection_equal: true,
        }
    } else {
        ValidatedPointerRelationV1 {
            output_present: false,
            selection_present: false,
            selection_equal: false,
        }
    };
    let setup_pointer_matches = !path.is_reuse()
        || prestate
            .setup_pointer
            .is_some_and(|setup| setup == callback.output_pointer());
    if callback.region() != path.region() as i32
        || callback.region_size() != REGION_SIZE as i32
        || callback.raw_extend() != path.raw_extend()
        || callback.result_code() != ffi::SQLITE_OK
        || callback.output_was_cleared() != !path.is_mapped()
        || !callback.before().methods_installed
        || !callback.before().state_installed
        || !callback.after().methods_installed
        || !callback.after().state_installed
        || !exact_snapshot(path, true, after)
        || !exact_map_receipt(path, receipt)
        || pointer_relation != expected_relation
        || !setup_pointer_matches
    {
        return Err(anyhow!("Map lifecycle native callback receipt mismatch"));
    }
    Ok(pointer_relation)
}

fn exact_map_receipt(
    path: MapRunnerLifecyclePathV1,
    value: ManagedSqliteShmTestMapReceipt,
) -> bool {
    let created = u16::from(path.is_empty());
    let live = u16::from(!path.is_empty());
    let mapped = u16::from(path.is_mapped());
    let mapped_new = u16::from(path.is_new_mapping());
    let mapped_reuse = u16::from(path.is_reuse());
    let not_present = 1 - mapped;
    value.runtime_generation != 0
        && value.shm_connection_id != 0
        && value.expectation == map_expectation(path)
        && value.managed_attempts == 1
        && value.created_first_shared == created
        && value.node_live == live
        && value.dms_exclusive_acquires == created
        && value.dms_truncates == created
        && value.dms_exclusive_releases == created
        && value.dms_shared_acquires == created
        && value.dms_ready == 1
        && value.file_size_checks == 1
        && value.file_size_before == file_size_before(path)
        && value.logical_end == logical_end(path)
        && value.file_grows == mapped_new
        && value.mapping_creates == mapped_new
        && value.view_maps == mapped_new
        && value.records == mapped_new
        && value.not_present == not_present
        && value.mapped == mapped
        && value.mapped_new == mapped_new
        && value.mapped_reuses == mapped_reuse
        && value.selected_pointer.is_some() == path.is_mapped()
        && value.selected_length
            == if path.is_mapped() {
                REGION_SIZE as usize
            } else {
                0
            }
        && value.selected_region == path.is_mapped().then_some(path.region())
        && value.selected_runtime_generation == path.is_mapped().then_some(value.runtime_generation)
        && value.managed_successes == 1
        && value.finished
}

fn exact_snapshot(
    path: MapRunnerLifecyclePathV1,
    after: bool,
    value: ManagedSqliteShmTestTargetSnapshot,
) -> bool {
    let topology = value.topology;
    let (node_present, views, mappings, dms, shm_file_present) = expected_topology(path, after);
    value.target_attached
        && value.shared_mask == 0
        && value.exclusive_mask == 0
        && topology.shm_connections == 1
        && topology.node_present == node_present
        && topology.views == views
        && topology.mappings == mappings
        && topology.dms == dms
        && topology.shm_file_present == shm_file_present
        && !topology.poisoned
        && !topology.mutation_may_have_occurred
        && !topology.lock_outcome_uncertain
        && !topology.domain_terminal
        && topology.quarantined_file_closes == 0
}

fn expected_topology(
    path: MapRunnerLifecyclePathV1,
    after: bool,
) -> (bool, u16, u16, ManagedSqliteShmTestDmsCustody, bool) {
    if path.is_empty() && !after {
        return (false, 0, 0, ManagedSqliteShmTestDmsCustody::Absent, false);
    }
    let regions = match (path, after) {
        (MapRunnerLifecyclePathV1::EmptyObserveNotPresent, true) => 0,
        (MapRunnerLifecyclePathV1::MissingExtendMapped, true) => 2,
        _ => 1,
    };
    (
        true,
        regions,
        regions,
        ManagedSqliteShmTestDmsCustody::Shared,
        true,
    )
}

pub(super) fn snapshot_values(value: ManagedSqliteShmTestTargetSnapshot) -> [u64; 14] {
    let topology = value.topology;
    [
        u64::from(value.target_attached),
        u64::from(value.shared_mask),
        u64::from(value.exclusive_mask),
        u64::from(topology.shm_connections),
        u64::from(topology.node_present),
        u64::from(topology.views),
        u64::from(topology.mappings),
        dms_tag(topology.dms),
        u64::from(topology.shm_file_present),
        u64::from(topology.poisoned),
        u64::from(topology.mutation_may_have_occurred),
        u64::from(topology.lock_outcome_uncertain),
        u64::from(topology.domain_terminal),
        u64::from(topology.quarantined_file_closes),
    ]
}

pub(super) const fn dms_tag(value: ManagedSqliteShmTestDmsCustody) -> u64 {
    match value {
        ManagedSqliteShmTestDmsCustody::Absent => 0,
        ManagedSqliteShmTestDmsCustody::Shared => 1,
        ManagedSqliteShmTestDmsCustody::SharedOutcomeUncertain => 2,
        ManagedSqliteShmTestDmsCustody::ExclusiveKnown => 3,
        ManagedSqliteShmTestDmsCustody::ExclusiveOutcomeUncertain => 4,
        ManagedSqliteShmTestDmsCustody::Released => 5,
    }
}

pub(super) const fn file_size_before(path: MapRunnerLifecyclePathV1) -> u64 {
    if path.is_empty() {
        0
    } else {
        REGION_SIZE as u64
    }
}

pub(super) const fn logical_end(path: MapRunnerLifecyclePathV1) -> u64 {
    if path.is_missing() {
        (REGION_SIZE as u64) * 2
    } else {
        REGION_SIZE as u64
    }
}
