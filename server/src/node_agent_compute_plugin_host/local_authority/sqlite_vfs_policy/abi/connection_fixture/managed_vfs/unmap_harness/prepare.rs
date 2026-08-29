//! Real two-Connection WAL preparation for SharedNonFinal Unmap observations.

use std::{ops::Deref, path::Path, time::Duration};

use anyhow::{anyhow, Context};

use super::super::ManagedSqliteMultiConnectionFixture;

pub(super) const SELECTED: usize = 0;
pub(super) const SIBLING: usize = 1;
pub(super) const PROBE_VALUE: i64 = 941;

pub(super) struct RetainedUnmapFixture {
    fixture: Option<ManagedSqliteMultiConnectionFixture>,
}

impl RetainedUnmapFixture {
    pub(super) fn prepare(root: &Path) -> anyhow::Result<Self> {
        let fixture = ManagedSqliteMultiConnectionFixture::open(root, [0xb3; 16])?;
        for index in [SELECTED, SIBLING] {
            fixture.connection(index)?.busy_timeout(Duration::ZERO)?;
        }
        let mode: String = fixture
            .connection(SELECTED)?
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .context("enable WAL on selected Unmap route")?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(anyhow!("Unmap fixture did not enter WAL mode"));
        }
        fixture
            .route(SELECTED)?
            .into_schema_migration()
            .context("authorize selected Unmap schema migration")?;
        fixture
            .connection(SELECTED)?
            .execute_batch(
                "CREATE TABLE unmap_probe (
                     probe_id INTEGER PRIMARY KEY,
                     value INTEGER NOT NULL
                 );",
            )
            .context("create selected Unmap probe table")?;
        fixture
            .route(SELECTED)?
            .into_runtime()
            .context("return selected Unmap route to runtime")?;
        fixture
            .route(SIBLING)?
            .into_schema_migration()
            .context("authorize sibling Unmap schema migration")?;
        fixture
            .route(SIBLING)?
            .into_runtime()
            .context("return sibling Unmap route to runtime")?;
        fixture
            .connection(SELECTED)?
            .execute(
                "INSERT INTO unmap_probe(probe_id, value) VALUES (1, ?1)",
                [PROBE_VALUE],
            )
            .context("insert selected Unmap probe row")?;
        fixture.verify_unmap_sibling_sql(SIBLING, PROBE_VALUE)?;

        let selected = fixture.observe_unmap_route(SELECTED)?;
        let sibling = fixture.observe_unmap_route(SIBLING)?;
        let selected_logical_names = fixture
            .route(SELECTED)?
            .barrier_logical_route_snapshot()?
            .exact_route_names();
        let sibling_logical_names = fixture
            .route(SIBLING)?
            .barrier_logical_route_snapshot()?
            .exact_route_names();
        if !selected.physical.target_attached
            || !sibling.physical.target_attached
            || selected.physical.shared_mask != 0
            || selected.physical.exclusive_mask != 0
            || sibling.physical.shared_mask != 0
            || sibling.physical.exclusive_mask != 0
            || selected.physical.topology.shm_connections != 2
            || sibling.physical.topology.shm_connections != 2
            || selected.target.runtime_generation() != sibling.target.runtime_generation()
            || selected.target.shm_connection_id() == sibling.target.shm_connection_id()
            || !selected.physical.topology.node_present
            || selected.physical.topology.views != 1
            || selected.physical.topology.mappings != 1
        {
            return Err(anyhow!(
                "Unmap fixture did not establish two exact shared SHM connections"
            ));
        }
        let (routes, logical_names) = fixture.logical_route_counts()?;
        if fixture.live_connection_count() != 2
            || routes != 2
            || logical_names != 6
            || selected_logical_names == 0
            || sibling_logical_names == 0
            || selected_logical_names.checked_add(sibling_logical_names) != Some(logical_names)
        {
            return Err(anyhow!("Unmap fixture route topology is not exact 2/2/6"));
        }
        let registration = fixture.live_registration_snapshot()?;
        if !registration.registered()
            || !registration.table_present()
            || !registration.name_present()
            || !registration.context_present()
        {
            return Err(anyhow!("Unmap fixture registration custody is incomplete"));
        }
        Ok(Self {
            fixture: Some(fixture),
        })
    }
}

impl Deref for RetainedUnmapFixture {
    type Target = ManagedSqliteMultiConnectionFixture;

    fn deref(&self) -> &Self::Target {
        self.fixture.as_ref().expect("retained Unmap fixture")
    }
}

impl Drop for RetainedUnmapFixture {
    fn drop(&mut self) {
        if let Some(fixture) = self.fixture.take() {
            // Every dynamic child retains the selected route until process exit. This prevents
            // Drop from issuing a second xShmUnmap after an observed terminal or detached state.
            std::mem::forget(fixture);
        }
    }
}
