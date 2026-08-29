//! Route-bound real SQLite liveness receipt captured after the one raw xShmUnmap call.

use anyhow::anyhow;

use super::super::super::{
    a2b2_cases::UnmapSelector, ManagedSqliteMultiConnectionFixture, ManagedTestShmTargetWitness,
};
use rusqlite::{Connection, Statement};

use super::prepare as fixture_layout;

const SCALAR_NONCE: i64 = 0x2a_49;

pub(super) struct FinalSqliteLivenessReceipt {
    selector: UnmapSelector,
    target: ManagedTestShmTargetWitness,
    observed: i64,
}

pub(super) struct FinalSqliteLivenessProbe<'connection> {
    selector: UnmapSelector,
    target: ManagedTestShmTargetWitness,
    connection: &'connection Connection,
    statement: Statement<'connection>,
}

pub(super) fn prepare(
    fixture: &ManagedSqliteMultiConnectionFixture,
    selector: UnmapSelector,
    target: ManagedTestShmTargetWitness,
) -> anyhow::Result<FinalSqliteLivenessProbe<'_>> {
    let connection = fixture.connection(fixture_layout::SELECTED)?;
    if !connection.is_autocommit() {
        return Err(anyhow!(
            "final Unmap SQLite liveness began inside a transaction"
        ));
    }
    // Preparing while the route is still active performs SQLite's authorizer decision before the
    // one raw xShmUnmap call. The constant VM can then be stepped afterward without pager or VFS
    // access, including when the callback has made the route terminal.
    let statement = connection.prepare("SELECT ?1")?;
    Ok(FinalSqliteLivenessProbe {
        selector,
        target,
        connection,
        statement,
    })
}

impl FinalSqliteLivenessProbe<'_> {
    pub(super) fn probe_after_unmap(
        mut self,
    ) -> anyhow::Result<Option<FinalSqliteLivenessReceipt>> {
        let observed = self.statement.query_row([SCALAR_NONCE], |row| row.get(0))?;
        if observed != SCALAR_NONCE || !self.connection.is_autocommit() {
            return Err(anyhow!(
                "final Unmap SQLite liveness probe changed value or transaction state"
            ));
        }
        Ok(Some(FinalSqliteLivenessReceipt {
            selector: self.selector,
            target: self.target,
            observed,
        }))
    }
}

pub(super) fn validate(
    receipt: Option<&FinalSqliteLivenessReceipt>,
    selector: UnmapSelector,
    target: ManagedTestShmTargetWitness,
) -> anyhow::Result<()> {
    let Some(receipt) = receipt else {
        return Err(anyhow!("final Unmap SQLite liveness receipt is missing"));
    };
    if receipt.selector != selector || receipt.target != target || receipt.observed != SCALAR_NONCE
    {
        return Err(anyhow!("final Unmap SQLite liveness receipt mismatch"));
    }
    Ok(())
}
