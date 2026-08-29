//! One-connection WAL preparation for final-connection Unmap observations.

use std::{ops::Deref, path::Path, time::Duration};

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use super::super::super::{
    a2b2_cases::UnmapSelector, multi_connection::ManagedTestUnmapRouteObservation,
    ManagedSqliteMultiConnectionFixture, ManagedTestShmTargetWitness,
};
use super::outcome;

pub(super) const SELECTED: usize = 0;
pub(super) const PROBE_VALUE: i64 = 1_049;
const NODE_ABSENT_REGION: i32 = 256;
const NODE_ABSENT_REGION_SIZE: i32 = 32 * 1024;
const NODE_ABSENT_RAW_EXTEND: i32 = 0;

pub(super) struct FinalUnmapFixture {
    fixture: Option<ManagedSqliteMultiConnectionFixture>,
    node_absent_setup: Option<NodeAbsentSetupReceipt>,
}

struct NodeAbsentSetupReceipt {
    target: ManagedTestShmTargetWitness,
    region: i32,
    region_size: i32,
    raw_extend: i32,
    result_code: i32,
    output_was_cleared: bool,
    raw_slots_retained: bool,
}

impl FinalUnmapFixture {
    pub(super) fn prepare(root: &Path, selector: UnmapSelector) -> anyhow::Result<Self> {
        let node_absent = outcome::node_absent(selector);
        let fixture = ManagedSqliteMultiConnectionFixture::open_single(root, [0xf7; 16])?;
        fixture.connection(SELECTED)?.busy_timeout(Duration::ZERO)?;
        let node_absent_setup = if node_absent {
            Some(prepare_node_absent(&fixture)?)
        } else {
            prepare_live_node(&fixture)?;
            None
        };
        if outcome::requires_main_exclusive(selector) {
            let code = fixture
                .route(SELECTED)?
                .call_main_file_lock_exclusive()
                .map_err(anyhow::Error::msg)?;
            if code != ffi::SQLITE_OK {
                return Err(anyhow!(
                    "final-delete main-file xLock(EXCLUSIVE) failed with SQLite code {code}"
                ));
            }
        }
        validate_exact_final_topology(&fixture, node_absent)?;
        Ok(Self {
            fixture: Some(fixture),
            node_absent_setup,
        })
    }

    pub(super) fn validate_node_absent_setup(
        &self,
        selector: UnmapSelector,
        pre: ManagedTestUnmapRouteObservation,
    ) -> anyhow::Result<()> {
        let physical = pre.physical;
        let topology = physical.topology;
        match (&self.node_absent_setup, outcome::node_absent(selector)) {
            (Some(receipt), true)
                if receipt.target == pre.target
                    && receipt.region == NODE_ABSENT_REGION
                    && receipt.region_size == NODE_ABSENT_REGION_SIZE
                    && receipt.raw_extend == NODE_ABSENT_RAW_EXTEND
                    && receipt.result_code == ffi::SQLITE_IOERR_SHMMAP
                    && receipt.output_was_cleared
                    && receipt.raw_slots_retained
                    && physical.target_attached
                    && physical.shared_mask == 0
                    && physical.exclusive_mask == 0
                    && topology.shm_connections == 1
                    && !topology.node_present
                    && topology.views == 0
                    && topology.mappings == 0
                    && !topology.shm_file_present
                    && !topology.poisoned
                    && !topology.mutation_may_have_occurred
                    && !topology.lock_outcome_uncertain
                    && !topology.domain_terminal
                    && topology.quarantined_file_closes == 0 =>
            {
                Ok(())
            }
            (None, false) => Ok(()),
            _ => Err(anyhow!(
                "final Unmap node-absent SQLite setup receipt mismatch"
            )),
        }
    }
}

fn prepare_node_absent(
    fixture: &ManagedSqliteMultiConnectionFixture,
) -> anyhow::Result<NodeAbsentSetupReceipt> {
    let mode: String = fixture
        .connection(SELECTED)?
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .context("enable WAL for node-absent final Unmap")?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!(
            "node-absent final Unmap fixture did not enter WAL mode"
        ));
    }
    fixture
        .route(SELECTED)?
        .into_schema_migration()
        .context("enter node-absent route schema phase")?;
    fixture
        .route(SELECTED)?
        .into_runtime()
        .context("return node-absent route to runtime")?;
    // This is the installed xShmMap on the exact live SQLite main file. Region 256 is the first
    // region outside the production authority budget. The route therefore performs real WAL-main
    // promotion and SHM attachment before production request validation rejects the map, leaving
    // the required attached/active connection with no coordinator node.
    let map = fixture
        .route(SELECTED)?
        .call_main_shm_map_raw(
            NODE_ABSENT_REGION,
            NODE_ABSENT_REGION_SIZE,
            NODE_ABSENT_RAW_EXTEND,
        )
        .map_err(anyhow::Error::msg)?;
    let before = map.before();
    let after = map.after();
    let raw_slots_retained = before.methods_installed
        && before.state_installed
        && after.methods_installed
        && after.state_installed;
    if map.region() != NODE_ABSENT_REGION
        || map.region_size() != NODE_ABSENT_REGION_SIZE
        || map.raw_extend() != NODE_ABSENT_RAW_EXTEND
        || map.result_code() != ffi::SQLITE_IOERR_SHMMAP
        || !map.output_was_cleared()
        || !raw_slots_retained
        || !fixture.connection(SELECTED)?.is_autocommit()
    {
        return Err(anyhow!(
            "node-absent installed xShmMap did not stop at production pre-node validation"
        ));
    }
    let live: i64 = fixture
        .connection(SELECTED)?
        .query_row("SELECT 1", [], |row| row.get(0))
        .context("probe node-absent SQLite connection after rejected xShmMap")?;
    if live != 1 {
        return Err(anyhow!(
            "node-absent SQLite connection liveness probe changed"
        ));
    }
    let binding = fixture
        .route(SELECTED)?
        .installed_shm_fault_witness()
        .map_err(anyhow::Error::msg)?;
    Ok(NodeAbsentSetupReceipt {
        target: binding.target_witness().map_err(anyhow::Error::msg)?,
        region: map.region(),
        region_size: map.region_size(),
        raw_extend: map.raw_extend(),
        result_code: map.result_code(),
        output_was_cleared: map.output_was_cleared(),
        raw_slots_retained,
    })
}

fn prepare_live_node(fixture: &ManagedSqliteMultiConnectionFixture) -> anyhow::Result<()> {
    let mode: String = fixture
        .connection(SELECTED)?
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .context("enable WAL for final Unmap")?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(anyhow!("final Unmap fixture did not enter WAL mode"));
    }
    fixture
        .route(SELECTED)?
        .into_schema_migration()
        .context("authorize final Unmap schema migration")?;
    fixture.connection(SELECTED)?.execute_batch(
        "CREATE TABLE final_unmap_probe (
             probe_id INTEGER PRIMARY KEY,
             value INTEGER NOT NULL
         );",
    )?;
    fixture
        .route(SELECTED)?
        .into_runtime()
        .context("return final Unmap route to runtime")?;
    fixture.connection(SELECTED)?.execute(
        "INSERT INTO final_unmap_probe(probe_id, value) VALUES (1, ?1)",
        [PROBE_VALUE],
    )?;
    Ok(())
}

fn validate_exact_final_topology(
    fixture: &ManagedSqliteMultiConnectionFixture,
    node_absent: bool,
) -> anyhow::Result<()> {
    let observed = fixture.observe_unmap_route(SELECTED)?;
    let physical = observed.physical.topology;
    if !observed.physical.target_attached
        || observed.physical.shared_mask != 0
        || observed.physical.exclusive_mask != 0
        || physical.shm_connections != 1
        || physical.node_present == node_absent
        || (!node_absent
            && (physical.views != 1 || physical.mappings != 1 || !physical.shm_file_present))
        || (node_absent
            && (physical.views != 0 || physical.mappings != 0 || physical.shm_file_present))
    {
        return Err(anyhow!(
            "final Unmap fixture did not establish its exact node precondition"
        ));
    }
    let (routes, names) = fixture.logical_route_counts()?;
    let exact_names = fixture
        .route(SELECTED)?
        .barrier_logical_route_snapshot()?
        .exact_route_names();
    if fixture.live_connection_count() != 1 || routes != 1 || names != 3 || exact_names != 3 {
        return Err(anyhow!("final Unmap route topology is not exact 1/1/3"));
    }
    let registration = fixture.live_registration_snapshot()?;
    if !registration.registered()
        || !registration.table_present()
        || !registration.name_present()
        || !registration.context_present()
    {
        return Err(anyhow!("final Unmap registration custody is incomplete"));
    }
    Ok(())
}

impl Deref for FinalUnmapFixture {
    type Target = ManagedSqliteMultiConnectionFixture;

    fn deref(&self) -> &Self::Target {
        self.fixture.as_ref().expect("retained final Unmap fixture")
    }
}

impl Drop for FinalUnmapFixture {
    fn drop(&mut self) {
        if let Some(fixture) = self.fixture.take() {
            // Every child owns one raw xShmUnmap call. Retaining until process exit prevents
            // SQLite Drop from issuing a second call after terminal or detached custody.
            std::mem::forget(fixture);
        }
    }
}
