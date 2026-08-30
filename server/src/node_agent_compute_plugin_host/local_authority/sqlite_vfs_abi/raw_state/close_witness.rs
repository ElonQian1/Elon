//! Test-only durable witness for the raw SQLite xClose ownership transition.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Debug, Default)]
struct RawCloseWitnessState {
    next_order: AtomicUsize,
    raw_close_entries: AtomicUsize,
    raw_close_entry_order: AtomicUsize,
    state_take_attempts: AtomicUsize,
    state_take_attempt_order: AtomicUsize,
    methods_clears: AtomicUsize,
    methods_clear_order: AtomicUsize,
    state_take_successes: AtomicUsize,
    state_take_success_order: AtomicUsize,
    state_close_custody_retentions: AtomicUsize,
    state_close_custody_retention_order: AtomicUsize,
    state_close_attempts: AtomicUsize,
    state_close_attempt_order: AtomicUsize,
    state_abandons: AtomicUsize,
    state_abandon_order: AtomicUsize,
}

/// Cloneable observation handle that never exposes the SQLite allocation or its Rust payload.
#[derive(Debug, Clone)]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct HandleBoundSqliteAbiRawCloseWitness
{
    state: Arc<RawCloseWitnessState>,
}

/// Redacted counts and first-event order from one exact raw SQLite file allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority) struct HandleBoundSqliteAbiRawCloseWitnessSnapshot
{
    pub(in crate::node_agent_compute_plugin_host::local_authority) raw_close_entries: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) raw_close_entry_order: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_take_attempts: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_take_attempt_order: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) methods_clears: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) methods_clear_order: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_take_successes: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_take_success_order: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_close_custody_retentions:
        usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_close_custody_retention_order:
        usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_close_attempts: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_close_attempt_order: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_abandons: usize,
    pub(in crate::node_agent_compute_plugin_host::local_authority) state_abandon_order: usize,
}

impl HandleBoundSqliteAbiRawCloseWitness {
    pub(super) fn new() -> Self {
        Self {
            state: Arc::new(RawCloseWitnessState::default()),
        }
    }

    pub(super) fn record_raw_close_entry(&self) {
        self.record(
            &self.state.raw_close_entries,
            &self.state.raw_close_entry_order,
        );
    }

    pub(super) fn record_state_take_attempt(&self) {
        self.record(
            &self.state.state_take_attempts,
            &self.state.state_take_attempt_order,
        );
    }

    pub(super) fn record_methods_clear(&self) {
        self.record(&self.state.methods_clears, &self.state.methods_clear_order);
    }

    pub(super) fn record_state_take_success(&self) {
        self.record(
            &self.state.state_take_successes,
            &self.state.state_take_success_order,
        );
    }

    pub(super) fn record_state_close_custody_retention(&self) {
        self.record(
            &self.state.state_close_custody_retentions,
            &self.state.state_close_custody_retention_order,
        );
    }

    pub(super) fn record_state_close_attempt(&self) {
        self.record(
            &self.state.state_close_attempts,
            &self.state.state_close_attempt_order,
        );
    }

    pub(super) fn record_state_abandon(&self) {
        self.record(&self.state.state_abandons, &self.state.state_abandon_order);
    }

    fn record(&self, count: &AtomicUsize, first_order: &AtomicUsize) {
        let prior = count.fetch_add(1, Ordering::SeqCst);
        if prior == 0 {
            let order = self.state.next_order.fetch_add(1, Ordering::SeqCst) + 1;
            first_order.store(order, Ordering::SeqCst);
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn snapshot(
        &self,
    ) -> HandleBoundSqliteAbiRawCloseWitnessSnapshot {
        HandleBoundSqliteAbiRawCloseWitnessSnapshot {
            raw_close_entries: self.state.raw_close_entries.load(Ordering::SeqCst),
            raw_close_entry_order: self.state.raw_close_entry_order.load(Ordering::SeqCst),
            state_take_attempts: self.state.state_take_attempts.load(Ordering::SeqCst),
            state_take_attempt_order: self.state.state_take_attempt_order.load(Ordering::SeqCst),
            methods_clears: self.state.methods_clears.load(Ordering::SeqCst),
            methods_clear_order: self.state.methods_clear_order.load(Ordering::SeqCst),
            state_take_successes: self.state.state_take_successes.load(Ordering::SeqCst),
            state_take_success_order: self.state.state_take_success_order.load(Ordering::SeqCst),
            state_close_custody_retentions: self
                .state
                .state_close_custody_retentions
                .load(Ordering::SeqCst),
            state_close_custody_retention_order: self
                .state
                .state_close_custody_retention_order
                .load(Ordering::SeqCst),
            state_close_attempts: self.state.state_close_attempts.load(Ordering::SeqCst),
            state_close_attempt_order: self.state.state_close_attempt_order.load(Ordering::SeqCst),
            state_abandons: self.state.state_abandons.load(Ordering::SeqCst),
            state_abandon_order: self.state.state_abandon_order.load(Ordering::SeqCst),
        }
    }
}
