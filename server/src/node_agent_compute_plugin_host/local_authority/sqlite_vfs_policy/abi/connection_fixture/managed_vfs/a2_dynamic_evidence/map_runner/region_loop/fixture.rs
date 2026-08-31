//! Exact WAL prestates and lower-ledger validation for q4 Map region loops.

use std::path::Path;

use anyhow::anyhow;
use rusqlite::ffi;

use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestMapReceipt,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::{
    connection::ManagedTestShmMapCallbackObservation, ManagedSqliteMultiConnectionFixture,
};
use super::{map_expectation, MapRunnerRegionLoopBindingV1, REGION_SIZE, SELECTED};

const PROMOTION_REGION_SIZE_GUARD: i32 = 65_537;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ValidatedPointerRelationV1 {
    pub(super) output_present: bool,
    pub(super) selection_present: bool,
    pub(super) selection_equal: bool,
}

pub(super) fn prepare(root: &Path) -> anyhow::Result<ManagedSqliteMultiConnectionFixture> {
    let fixture = ManagedSqliteMultiConnectionFixture::open_single(root, [0xa7; 16])?;
    let mode: String =
        fixture
            .connection(SELECTED)?
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!("Map region-loop fixture did not enter WAL mode"));
    }
    fixture.route(SELECTED)?.into_schema_migration()?;
    fixture.route(SELECTED)?.into_runtime()?;
    if fixture
        .route(SELECTED)?
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?
    {
        return Err(anyhow!(
            "Map region-loop target existed before explicit prestate installation"
        ));
    }
    Ok(fixture)
}

pub(super) fn install_prestate(
    fixture: &ManagedSqliteMultiConnectionFixture,
    binding: MapRunnerRegionLoopBindingV1,
) -> anyhow::Result<()> {
    let callback = if binding.family.is_empty() {
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
    let exact = if binding.family.is_empty() {
        callback.region() == 0
            && callback.region_size() == PROMOTION_REGION_SIZE_GUARD
            && callback.raw_extend() == 0
            && callback.result_code() == ffi::SQLITE_IOERR_SHMMAP
            && callback.output_was_cleared()
            && callback.output_pointer().is_null()
            && slots_installed
    } else {
        callback.region() == 0
            && callback.region_size() == REGION_SIZE as i32
            && callback.raw_extend() == 1
            && callback.result_code() == ffi::SQLITE_OK
            && !callback.output_was_cleared()
            && !callback.output_pointer().is_null()
            && slots_installed
    };
    if !exact {
        return Err(anyhow!("Map region-loop prestate installation mismatch"));
    }
    if !fixture
        .route(SELECTED)?
        .exact_main_shm_target_presence()
        .map_err(anyhow::Error::msg)?
    {
        return Err(anyhow!(
            "Map region-loop exact target was not attached by prestate installation"
        ));
    }
    Ok(())
}

pub(super) fn validate_prestate(
    binding: MapRunnerRegionLoopBindingV1,
    before: ManagedSqliteShmTestTargetSnapshot,
) -> anyhow::Result<()> {
    if exact_snapshot(binding, false, before) {
        Ok(())
    } else {
        Err(anyhow!("Map region-loop exact prestate mismatch"))
    }
}

pub(super) fn validate_action(
    binding: MapRunnerRegionLoopBindingV1,
    callback: ManagedTestShmMapCallbackObservation,
    before: ManagedSqliteShmTestTargetSnapshot,
    after: ManagedSqliteShmTestTargetSnapshot,
    receipt: ManagedSqliteShmTestMapReceipt,
) -> anyhow::Result<ValidatedPointerRelationV1> {
    validate_prestate(binding, before)?;
    let output = callback.output_pointer();
    let relation = ValidatedPointerRelationV1 {
        output_present: !output.is_null(),
        selection_present: receipt.selected_pointer.is_some(),
        selection_equal: !output.is_null()
            && receipt.selected_pointer.is_some()
            && receipt.selected_pointer_matches(output.cast::<u8>()),
    };
    let expected_relation = ValidatedPointerRelationV1 {
        output_present: true,
        selection_present: true,
        selection_equal: true,
    };
    if callback.region() != binding.target_region as i32
        || callback.region_size() != REGION_SIZE as i32
        || callback.raw_extend() != 1
        || callback.result_code() != ffi::SQLITE_OK
        || callback.output_was_cleared()
        || !callback.before().methods_installed
        || !callback.before().state_installed
        || !callback.after().methods_installed
        || !callback.after().state_installed
        || !exact_snapshot(binding, true, after)
        || !exact_map_receipt(binding, receipt)
        || relation != expected_relation
    {
        return Err(anyhow!("Map region-loop native callback receipt mismatch"));
    }
    Ok(relation)
}

fn exact_map_receipt(
    binding: MapRunnerRegionLoopBindingV1,
    value: ManagedSqliteShmTestMapReceipt,
) -> bool {
    let created = u16::from(binding.family.is_empty());
    value.runtime_generation != 0
        && value.shm_connection_id != 0
        && value.expectation == map_expectation(binding)
        && value.managed_attempts == 1
        && value.created_first_shared == created
        && value.node_live == 1 - created
        && value.dms_exclusive_acquires == created
        && value.dms_truncates == created
        && value.dms_exclusive_releases == created
        && value.dms_shared_acquires == created
        && value.dms_ready == 1
        && value.file_size_checks == 1
        && value.file_size_before == file_size_before(binding)
        && value.logical_end == logical_end(binding)
        && value.file_grows == 1
        && value.mapping_creates == binding.regions_to_create
        && value.view_maps == binding.regions_to_create
        && value.records == binding.regions_to_create
        && value.not_present == 0
        && value.mapped == 1
        && value.mapped_new == 1
        && value.mapped_reuses == 0
        && value.selected_pointer.is_some()
        && value.selected_length == REGION_SIZE as usize
        && value.selected_region == Some(binding.target_region)
        && value.selected_runtime_generation == Some(value.runtime_generation)
        && value.managed_successes == 1
        && value.finished
}

fn exact_snapshot(
    binding: MapRunnerRegionLoopBindingV1,
    after: bool,
    value: ManagedSqliteShmTestTargetSnapshot,
) -> bool {
    let topology = value.topology;
    let (node, regions, dms, file) = expected_topology(binding, after);
    value.target_attached
        && value.shared_mask == 0
        && value.exclusive_mask == 0
        && topology.shm_connections == 1
        && topology.node_present == node
        && topology.views == regions
        && topology.mappings == regions
        && topology.dms == dms
        && topology.shm_file_present == file
        && !topology.poisoned
        && !topology.mutation_may_have_occurred
        && !topology.lock_outcome_uncertain
        && !topology.domain_terminal
        && topology.quarantined_file_closes == 0
}

fn expected_topology(
    binding: MapRunnerRegionLoopBindingV1,
    after: bool,
) -> (bool, u16, ManagedSqliteShmTestDmsCustody, bool) {
    if binding.family.is_empty() && !after {
        return (false, 0, ManagedSqliteShmTestDmsCustody::Absent, false);
    }
    let regions = if after {
        binding.target_region as u16 + 1
    } else {
        1
    };
    (true, regions, ManagedSqliteShmTestDmsCustody::Shared, true)
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

pub(super) const fn file_size_before(binding: MapRunnerRegionLoopBindingV1) -> u64 {
    if binding.family.is_empty() {
        0
    } else {
        REGION_SIZE as u64
    }
}

pub(super) const fn logical_end(binding: MapRunnerRegionLoopBindingV1) -> u64 {
    (binding.target_region as u64 + 1) * REGION_SIZE as u64
}
