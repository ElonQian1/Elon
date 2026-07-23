//! Durable receipts for explicit update-gate reconciliation requests.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{now_ms, UpdateRecoveryStore};

const MAX_RECONCILE_RECEIPTS: usize = 200;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct UpdateGateReconcileReceipt {
    pub(crate) operation_id: String,
    pub(crate) status: String,
    pub(crate) started_at_ms: u128,
    #[serde(default)]
    pub(crate) finished_at_ms: Option<u128>,
    #[serde(default)]
    pub(crate) result: Option<Value>,
    #[serde(default)]
    pub(crate) error: Option<String>,
}

impl UpdateRecoveryStore {
    pub(crate) fn begin_reconcile(&self) -> Result<UpdateGateReconcileReceipt> {
        let now = now_ms();
        let receipt = UpdateGateReconcileReceipt {
            operation_id: format!("reconcile-request-{now}-{}", std::process::id()),
            status: "running".to_string(),
            started_at_ms: now,
            finished_at_ms: None,
            result: None,
            error: None,
        };
        let mut ledger = self.load()?;
        ledger.reconcile_receipts.push(receipt.clone());
        if ledger.reconcile_receipts.len() > MAX_RECONCILE_RECEIPTS {
            let drop_count = ledger.reconcile_receipts.len() - MAX_RECONCILE_RECEIPTS;
            ledger.reconcile_receipts.drain(..drop_count);
        }
        self.save(&ledger)?;
        Ok(receipt)
    }

    pub(crate) fn finish_reconcile(
        &self,
        operation_id: &str,
        result: Result<Value, String>,
    ) -> Result<UpdateGateReconcileReceipt> {
        let mut ledger = self.load()?;
        let receipt = ledger
            .reconcile_receipts
            .iter_mut()
            .find(|item| item.operation_id == operation_id)
            .context("update gate reconcile receipt not found")?;
        if receipt.status != "running" {
            return Ok(receipt.clone());
        }
        receipt.finished_at_ms = Some(now_ms());
        match result {
            Ok(value) => {
                receipt.status = "completed".to_string();
                receipt.result = Some(value);
            }
            Err(error) => {
                receipt.status = "failed".to_string();
                receipt.error = Some(error);
            }
        }
        let receipt = receipt.clone();
        self.save(&ledger)?;
        Ok(receipt)
    }

    pub(crate) fn interrupt_incomplete_reconciles(&self) -> Result<usize> {
        let mut ledger = self.load()?;
        let mut changed = 0;
        for receipt in &mut ledger.reconcile_receipts {
            if receipt.status == "running" {
                receipt.status = "interrupted".to_string();
                receipt.finished_at_ms = Some(now_ms());
                receipt.error =
                    Some("node runtime restarted before reconciliation completed".to_string());
                changed += 1;
            }
        }
        if changed > 0 {
            self.save(&ledger)?;
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_marks_unfinished_reconcile_as_readable_interrupted_receipt() {
        let root = std::env::temp_dir().join(format!("elon-reconcile-receipt-{}", now_ms()));
        let store = UpdateRecoveryStore::new(root.join("recovery.json"));
        let running = store.begin_reconcile().unwrap();
        assert_eq!(store.interrupt_incomplete_reconciles().unwrap(), 1);
        let ledger = store.load().unwrap();
        let recovered = ledger
            .reconcile_receipts
            .iter()
            .find(|item| item.operation_id == running.operation_id)
            .unwrap();
        assert_eq!(recovered.status, "interrupted");
        assert!(recovered.finished_at_ms.is_some());
        let _ = std::fs::remove_dir_all(root);
    }
}
