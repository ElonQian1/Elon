//! Same-transaction obligation for an authenticated receipt and its typed terminal closure.

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use anyhow::{ensure, Result};
use rusqlite::Transaction;

pub(super) struct ExternalPoolAdapterTaskIngressSession {
    connection_key: usize,
    pending_receipts: Rc<RefCell<HashSet<String>>>,
}

pub(super) struct PendingTaskIngressObligation {
    connection_key: usize,
    receipt_id: String,
    pending_receipts: Rc<RefCell<HashSet<String>>>,
}

impl ExternalPoolAdapterTaskIngressSession {
    pub(super) fn new(transaction: &Transaction<'_>) -> Self {
        Self {
            connection_key: connection_key(transaction),
            pending_receipts: Rc::new(RefCell::new(HashSet::new())),
        }
    }

    pub(super) fn register(
        &self,
        transaction: &Transaction<'_>,
        receipt_id: &str,
    ) -> Result<PendingTaskIngressObligation> {
        ensure!(
            self.connection_key == connection_key(transaction),
            "V278 ingress obligation changed SQLite transaction connection"
        );
        ensure!(
            self.pending_receipts
                .borrow_mut()
                .insert(receipt_id.to_string()),
            "V278 receipt already has a pending terminal obligation"
        );
        Ok(PendingTaskIngressObligation {
            connection_key: self.connection_key,
            receipt_id: receipt_id.to_string(),
            pending_receipts: Rc::clone(&self.pending_receipts),
        })
    }

    pub(super) fn ensure_resolved(&self, transaction: &Transaction<'_>) -> Result<()> {
        ensure!(
            self.connection_key == connection_key(transaction),
            "V278 ingress session changed SQLite transaction connection"
        );
        ensure!(
            self.pending_receipts.borrow().is_empty(),
            "V278 authenticated receipt was not closed in its insertion transaction"
        );
        Ok(())
    }
}

impl PendingTaskIngressObligation {
    pub(super) fn resolve(self, transaction: &Transaction<'_>) -> Result<()> {
        ensure!(
            self.connection_key == connection_key(transaction),
            "V278 terminal obligation changed SQLite transaction connection"
        );
        ensure!(
            self.pending_receipts.borrow_mut().remove(&self.receipt_id),
            "V278 terminal obligation was already resolved or replaced"
        );
        Ok(())
    }
}

fn connection_key(connection: &rusqlite::Connection) -> usize {
    // SAFETY: the handle is used only as identity while the transaction borrow is alive.
    unsafe { connection.handle() as usize }
}
