//! Process-wide serialization for update-recovery ledger transactions.

use std::sync::{Mutex, MutexGuard, OnceLock};

use anyhow::Result;

use super::{now_ms, UpdateInstallGate, UpdateRecoveryStore};

static UPDATE_RECOVERY_LEDGER_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn ledger_mutation_guard() -> MutexGuard<'static, ()> {
    UPDATE_RECOVERY_LEDGER_MUTATION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl UpdateRecoveryStore {
    pub(crate) fn update_install_gate(&self, gate: UpdateInstallGate) -> Result<()> {
        let _guard = ledger_mutation_guard();
        let mut ledger = self.load()?;
        ledger.install_gate = gate;
        self.save(&ledger)
    }

    pub(crate) fn set_install_gate_phase(&self, phase: &str, reason: Option<&str>) -> Result<()> {
        let _guard = ledger_mutation_guard();
        let mut ledger = self.load()?;
        ledger.install_gate.phase = phase.to_string();
        ledger.install_gate.reason = reason.map(str::to_string);
        ledger.install_gate.updated_at_ms = now_ms();
        self.save(&ledger)
    }
}
