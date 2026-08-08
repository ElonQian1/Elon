use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex,
};

use anyhow::{bail, Result};

use super::ComputePluginAuthorityInstanceBinding;

const INITIAL_CLEANUP_GENERATION: u64 = 1;

#[derive(Clone)]
pub(super) struct CandidateCleanupDeletionDomain {
    inner: Arc<CandidateCleanupDeletionDomainInner>,
}

struct CandidateCleanupDeletionDomainInner {
    state: Mutex<CandidateCleanupDeletionDomainState>,
    idle: Condvar,
}

struct CandidateCleanupDeletionDomainState {
    generation: u64,
    process_owner_epoch: i64,
    active_fence: Option<Arc<AtomicBool>>,
    active_operations: usize,
    transitioning: bool,
}

pub(super) struct CandidateCleanupProcessTransition {
    inner: Arc<CandidateCleanupDeletionDomainInner>,
    active: bool,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDeletionOperationLease {
    inner: Arc<CandidateCleanupDeletionDomainInner>,
    active: bool,
}

impl CandidateCleanupDeletionDomain {
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(CandidateCleanupDeletionDomainInner {
                state: Mutex::new(CandidateCleanupDeletionDomainState {
                    generation: INITIAL_CLEANUP_GENERATION,
                    process_owner_epoch: 0,
                    active_fence: None,
                    active_operations: 0,
                    transitioning: false,
                }),
                idle: Condvar::new(),
            }),
        }
    }

    pub(super) fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub(super) fn capture(
        &self,
        process_owner_epoch: i64,
        fence_liveness: &Arc<AtomicBool>,
    ) -> Result<u64> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_CLEANUP_DELETION_GATE_POISONED"))?;
        if state.process_owner_epoch != process_owner_epoch
            || state.transitioning
            || !fence_liveness.load(Ordering::Acquire)
            || !state
                .active_fence
                .as_ref()
                .is_some_and(|active| Arc::ptr_eq(active, fence_liveness))
        {
            bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_PREPARED_BINDING_INVALID");
        }
        Ok(state.generation)
    }

    pub(super) fn enter_operation(
        &self,
        fence_liveness: &Arc<AtomicBool>,
        process_owner_epoch: i64,
        generation: u64,
    ) -> Result<CandidateCleanupDeletionOperationLease> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_CLEANUP_DELETION_GATE_POISONED"))?;
        if state.process_owner_epoch != process_owner_epoch
            || state.generation != generation
            || state.transitioning
            || !fence_liveness.load(Ordering::Acquire)
            || !state
                .active_fence
                .as_ref()
                .is_some_and(|active| Arc::ptr_eq(active, fence_liveness))
        {
            bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_CUSTODY_REVOKED");
        }
        state.active_operations = state.active_operations.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CLEANUP_DELETION_OPERATION_EXHAUSTED")
        })?;
        drop(state);
        Ok(CandidateCleanupDeletionOperationLease {
            inner: Arc::clone(&self.inner),
            active: true,
        })
    }

    pub(super) fn begin_process_transition(&self) -> Result<CandidateCleanupProcessTransition> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_CLEANUP_DELETION_GATE_POISONED"))?;
        if state.transitioning {
            bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_TRANSITION_ACTIVE");
        }
        state.transitioning = true;
        while state.active_operations != 0 {
            state =
                self.inner.idle.wait(state).map_err(|_| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_CLEANUP_DELETION_GATE_POISONED")
                })?;
        }
        drop(state);
        Ok(CandidateCleanupProcessTransition {
            inner: Arc::clone(&self.inner),
            active: true,
        })
    }

    fn close_fence(&self, process_owner_epoch: i64, fence_liveness: &Arc<AtomicBool>) {
        let Ok(mut state) = self.inner.state.lock() else {
            fence_liveness.store(false, Ordering::Release);
            return;
        };
        let is_current = state.process_owner_epoch == process_owner_epoch
            && state
                .active_fence
                .as_ref()
                .is_some_and(|active| Arc::ptr_eq(active, fence_liveness));
        if !is_current || state.transitioning {
            return;
        }
        // Drop is a fail-close signal, not a process-owner handoff. It must never wait on an
        // operation lease that the dropping thread could itself own. A later handoff still enters
        // `begin_process_transition`, waits for the retained operation count to drain, and only
        // then advances the durable process epoch.
        fence_liveness.store(false, Ordering::Release);
        state.active_fence = None;
        self.inner.idle.notify_all();
    }
}

impl CandidateCleanupProcessTransition {
    pub(super) fn activate(
        &mut self,
        process_owner_epoch: i64,
        fence_liveness: Arc<AtomicBool>,
    ) -> Result<()> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_CLEANUP_DELETION_GATE_POISONED"))?;
        if !self.active || !state.transitioning || state.active_operations != 0 {
            bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_TRANSITION_CHANGED");
        }
        if process_owner_epoch <= state.process_owner_epoch {
            bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_PROCESS_EPOCH_NOT_ADVANCED");
        }
        let Some(next_generation) = state.generation.checked_add(1) else {
            drop(state);
            self.fail_closed();
            bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_GENERATION_EXHAUSTED");
        };
        if let Some(previous) = state.active_fence.take() {
            previous.store(false, Ordering::Release);
        }
        state.generation = next_generation;
        state.process_owner_epoch = process_owner_epoch;
        state.active_fence = Some(fence_liveness);
        state.transitioning = false;
        self.active = false;
        self.inner.idle.notify_all();
        Ok(())
    }

    pub(super) fn fail_closed(&mut self) {
        if !self.active {
            return;
        }
        let Ok(mut state) = self.inner.state.lock() else {
            self.active = false;
            return;
        };
        if let Some(previous) = state.active_fence.take() {
            previous.store(false, Ordering::Release);
        }
        state.generation = u64::MAX;
        state.transitioning = false;
        self.active = false;
        self.inner.idle.notify_all();
    }
}

impl Drop for CandidateCleanupProcessTransition {
    fn drop(&mut self) {
        self.fail_closed();
    }
}

impl Drop for CandidateCleanupDeletionOperationLease {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let Ok(mut state) = self.inner.state.lock() else {
            self.active = false;
            return;
        };
        state.active_operations = state.active_operations.saturating_sub(1);
        self.active = false;
        if state.active_operations == 0 {
            self.inner.idle.notify_all();
        }
    }
}

impl ComputePluginAuthorityInstanceBinding {
    pub(super) fn begin_cleanup_process_transition(
        &self,
    ) -> Result<CandidateCleanupProcessTransition> {
        self.cleanup_deletion_domain.begin_process_transition()
    }

    pub(super) fn close_cleanup_process_fence(
        &self,
        process_owner_epoch: i64,
        fence_liveness: &Arc<AtomicBool>,
    ) {
        self.cleanup_deletion_domain
            .close_fence(process_owner_epoch, fence_liveness);
    }
}
