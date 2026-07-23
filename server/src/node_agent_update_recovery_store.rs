//! Atomic persistence and task lookup for update recovery receipts.

use anyhow::{Context, Result};

use super::{
    now_ms, UpdateRecoveryLedger, UpdateRecoveryReceipt, UpdateRecoveryReview, UpdateRecoveryState,
    UpdateRecoveryStore,
};

impl UpdateRecoveryStore {
    pub(crate) fn default() -> Self {
        Self::new(super::super::state_path().with_file_name("node-update-recovery.json"))
    }

    pub(crate) fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(crate) fn load(&self) -> Result<UpdateRecoveryLedger> {
        if !self.path.exists() {
            return Ok(UpdateRecoveryLedger::default());
        }
        let bytes = std::fs::read(&self.path)
            .with_context(|| format!("read update recovery ledger {}", self.path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("parse update recovery ledger {}", self.path.display()))
    }

    pub(crate) fn save(&self, ledger: &UpdateRecoveryLedger) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(ledger)?;
        crate::node_agent_atomic_file::write(&self.path, &bytes)
    }

    pub(crate) fn upsert(&self, receipt: UpdateRecoveryReceipt) -> Result<()> {
        let _guard = super::ledger_mutation_guard();
        let mut ledger = self.load()?;
        match ledger.receipts.iter_mut().find(|current| {
            current.update_id == receipt.update_id
                && current.original_task_id == receipt.original_task_id
        }) {
            Some(current) => *current = receipt,
            None => ledger.receipts.push(receipt),
        }
        self.save(&ledger)
    }

    pub(crate) fn insert_if_absent(
        &self,
        receipt: UpdateRecoveryReceipt,
    ) -> Result<UpdateRecoveryReceipt> {
        let _guard = super::ledger_mutation_guard();
        let mut ledger = self.load()?;
        if let Some(current) = ledger.receipts.iter().find(|current| {
            current.update_id == receipt.update_id
                && current.original_task_id == receipt.original_task_id
        }) {
            return Ok(current.clone());
        }
        ledger.receipts.push(receipt.clone());
        self.save(&ledger)?;
        Ok(receipt)
    }

    pub(crate) fn update<R>(
        &self,
        update_id: &str,
        original_task_id: &str,
        update: impl FnOnce(&mut UpdateRecoveryReceipt) -> Result<R>,
    ) -> Result<R> {
        let _guard = super::ledger_mutation_guard();
        let mut ledger = self.load()?;
        let receipt = ledger
            .receipts
            .iter_mut()
            .find(|receipt| {
                receipt.update_id == update_id && receipt.original_task_id == original_task_id
            })
            .context("update recovery receipt not found")?;
        let result = update(receipt)?;
        self.save(&ledger)?;
        Ok(result)
    }

    pub(crate) fn transition(
        &self,
        update_id: &str,
        original_task_id: &str,
        next: UpdateRecoveryState,
        reason: Option<&str>,
    ) -> Result<bool> {
        let _guard = super::ledger_mutation_guard();
        let mut ledger = self.load()?;
        let receipt = ledger
            .receipts
            .iter_mut()
            .find(|receipt| {
                receipt.update_id == update_id && receipt.original_task_id == original_task_id
            })
            .context("update recovery receipt not found")?;
        let changed = receipt.transition(next, reason)?;
        if changed {
            self.save(&ledger)?;
        }
        Ok(changed)
    }

    pub(crate) fn active(&self) -> Result<Vec<UpdateRecoveryReceipt>> {
        Ok(self
            .load()?
            .receipts
            .into_iter()
            .filter(|receipt| !receipt.state.is_terminal())
            .collect())
    }

    pub(crate) fn receipt_for_task(&self, task_id: &str) -> Result<Option<UpdateRecoveryReceipt>> {
        self.load()?.receipt_for_task(task_id)
    }

    pub(crate) fn receipts_for_task(&self, task_id: &str) -> Result<Vec<UpdateRecoveryReceipt>> {
        Ok(self
            .load()?
            .receipts
            .into_iter()
            .filter(|receipt| {
                receipt.original_task_id == task_id
                    || receipt.resume_task_id.as_deref() == Some(task_id)
            })
            .collect())
    }

    pub(crate) fn record_sidecar_cursor(
        &self,
        update_id: &str,
        original_task_id: &str,
        offset: u64,
        sequence: u64,
    ) -> Result<()> {
        self.update(update_id, original_task_id, |receipt| {
            receipt.sidecar_output_offset = receipt.sidecar_output_offset.max(offset);
            receipt.sidecar_output_sequence = receipt.sidecar_output_sequence.max(sequence);
            receipt.updated_at_ms = now_ms();
            Ok(())
        })
    }

    pub(crate) fn record_final_review(
        &self,
        task_id: &str,
        review: UpdateRecoveryReview,
    ) -> Result<bool> {
        let _guard = super::ledger_mutation_guard();
        let mut ledger = self.load()?;
        let Some(receipt) = ledger
            .receipts
            .iter_mut()
            .filter(|receipt| {
                receipt.original_task_id == task_id
                    || receipt.resume_task_id.as_deref() == Some(task_id)
            })
            .max_by_key(|receipt| receipt.updated_at_ms)
        else {
            return Ok(false);
        };
        receipt.final_review = Some(review);
        receipt.updated_at_ms = now_ms();
        self.save(&ledger)?;
        Ok(true)
    }
}
