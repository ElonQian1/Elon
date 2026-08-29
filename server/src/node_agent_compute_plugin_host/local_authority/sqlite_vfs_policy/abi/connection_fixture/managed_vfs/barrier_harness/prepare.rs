//! Real two-Connection WAL preparation performed before the selected Barrier fault is installed.

use std::{ops::Deref, path::Path, time::Duration};

use anyhow::{anyhow, Context};

use super::super::ManagedSqliteMultiConnectionFixture;

pub(super) struct RetainedBarrierFixture {
    fixture: Option<ManagedSqliteMultiConnectionFixture>,
}

impl RetainedBarrierFixture {
    pub(super) fn prepare(root: &Path) -> anyhow::Result<Self> {
        let fixture = ManagedSqliteMultiConnectionFixture::open(root, [0xb2; 16])?;
        for index in 0..2 {
            fixture.connection(index)?.busy_timeout(Duration::ZERO)?;
        }
        let mode: String = fixture
            .connection(0)?
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
            .context("enable WAL on selected Barrier route")?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(anyhow!("Barrier fixture did not enter WAL mode"));
        }
        // Drive the first registered route into WAL before the sibling touches SHM. This binds
        // the frozen route-1 target to the coordinator's first physical connection identity.
        fixture
            .route(0)?
            .into_schema_migration()
            .context("authorize selected Barrier schema migration")?;
        fixture
            .connection(0)?
            .execute_batch(
                "CREATE TABLE barrier_probe (
                 probe_id INTEGER PRIMARY KEY,
                 value INTEGER NOT NULL
             );",
            )
            .context("create selected Barrier probe table")?;
        fixture
            .route(0)?
            .into_runtime()
            .context("return selected Barrier route to runtime")?;
        fixture
            .route(1)?
            .into_schema_migration()
            .context("authorize sibling Barrier schema migration")?;
        fixture
            .route(1)?
            .into_runtime()
            .context("return sibling Barrier route to runtime")?;
        fixture
            .connection(0)?
            .execute(
                "INSERT INTO barrier_probe(probe_id, value) VALUES (1, 829)",
                [],
            )
            .context("insert selected Barrier probe row")?;
        let sibling_value: i64 = fixture
            .connection(1)?
            .query_row(
                "SELECT value FROM barrier_probe WHERE probe_id=1",
                [],
                |row| row.get(0),
            )
            .context("read selected Barrier row from sibling")?;
        if sibling_value != 829 {
            return Err(anyhow!(
                "Barrier sibling did not share the selected WAL database"
            ));
        }

        let witness = fixture
            .route(0)?
            .installed_shm_fault_witness()
            .map_err(anyhow::Error::msg)?;
        let target = witness.observer().map_err(anyhow::Error::msg)?.snapshot()?;
        if !target.target_attached
            || target.topology.shm_connections != 2
            || !target.topology.node_present
            || target.topology.views != 1
            || target.topology.mappings != 1
        {
            return Err(anyhow!(
                "Barrier fixture did not establish the exact shared SHM topology"
            ));
        }
        let (routes, logical_names) = fixture.logical_route_counts()?;
        if fixture.live_connection_count() != 2 || routes != 2 || logical_names != 6 {
            return Err(anyhow!("Barrier fixture route topology is not exact 2/2/6"));
        }
        let registration = fixture.live_registration_snapshot()?;
        if !registration.registered()
            || !registration.table_present()
            || !registration.name_present()
            || !registration.context_present()
        {
            return Err(anyhow!(
                "Barrier fixture VFS registration custody is incomplete"
            ));
        }
        Ok(Self {
            fixture: Some(fixture),
        })
    }
}

impl Deref for RetainedBarrierFixture {
    type Target = ManagedSqliteMultiConnectionFixture;

    fn deref(&self) -> &Self::Target {
        self.fixture.as_ref().expect("retained Barrier fixture")
    }
}

impl Drop for RetainedBarrierFixture {
    fn drop(&mut self) {
        if let Some(fixture) = self.fixture.take() {
            // A dynamic evidence child keeps all success or terminal custody until process exit.
            // This also prevents a fallible observation from entering SQLite teardown as a retry.
            std::mem::forget(fixture);
        }
    }
}

pub(super) fn checked_u8(value: usize, label: &'static str) -> anyhow::Result<u8> {
    u8::try_from(value).with_context(|| format!("{label} exceeds u8"))
}
