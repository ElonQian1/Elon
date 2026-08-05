use std::time::{Duration, Instant};

use anyhow::{bail, Result};

const HARD_MAX_HASH_BYTES: u64 = 64 * 1_024 * 1_024 * 1_024 * 1_024;
const HARD_MAX_HASH_ELAPSED: Duration = Duration::from_secs(24 * 60 * 60);

/// One-shot local resource policy for a single candidate artifact-set hash.
///
/// The signed closure says what must be hashed. This policy independently limits what the node is
/// willing to spend and is consumed when hashing starts, so no unbudgeted retry path exists.
#[must_use = "candidate hash budget must be consumed by the authorized hash operation"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateVerificationHashBudget {
    max_total_bytes: u64,
    max_elapsed: Duration,
}

pub(super) struct ActiveCandidateVerificationHashBudget {
    expected_total_bytes: u64,
    max_total_bytes: u64,
    hashed_bytes: u64,
    deadline: Instant,
}

impl CandidateVerificationHashBudget {
    pub(in crate::node_agent_compute_plugin_host) fn new(
        max_total_bytes: u64,
        max_elapsed: Duration,
    ) -> Result<Self> {
        if max_total_bytes == 0
            || max_total_bytes > HARD_MAX_HASH_BYTES
            || max_elapsed.is_zero()
            || max_elapsed > HARD_MAX_HASH_ELAPSED
        {
            bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_BUDGET_INVALID");
        }
        Ok(Self {
            max_total_bytes,
            max_elapsed,
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn max_total_bytes(&self) -> u64 {
        self.max_total_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn max_elapsed(&self) -> Duration {
        self.max_elapsed
    }

    pub(super) fn activate(
        self,
        expected_total_bytes: u64,
    ) -> Result<ActiveCandidateVerificationHashBudget> {
        if expected_total_bytes == 0 || expected_total_bytes > self.max_total_bytes {
            bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_BYTE_BUDGET_EXCEEDED");
        }
        let deadline = Instant::now()
            .checked_add(self.max_elapsed)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_HASH_DEADLINE_INVALID"))?;
        Ok(ActiveCandidateVerificationHashBudget {
            expected_total_bytes,
            max_total_bytes: self.max_total_bytes,
            hashed_bytes: 0,
            deadline,
        })
    }
}

impl ActiveCandidateVerificationHashBudget {
    pub(super) fn ensure_current(&self) -> Result<()> {
        if Instant::now() >= self.deadline {
            bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_DEADLINE_EXCEEDED");
        }
        Ok(())
    }

    pub(super) fn record_hashed(&mut self, artifact_bytes: u64) -> Result<()> {
        self.ensure_current()?;
        self.hashed_bytes = self
            .hashed_bytes
            .checked_add(artifact_bytes)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_HASH_BYTES_OVERFLOW"))?;
        if self.hashed_bytes > self.max_total_bytes || self.hashed_bytes > self.expected_total_bytes
        {
            bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_BYTE_BUDGET_EXCEEDED");
        }
        Ok(())
    }

    pub(super) fn finish(self, observed_total_bytes: u64) -> Result<()> {
        self.ensure_current()?;
        if self.hashed_bytes != self.expected_total_bytes
            || observed_total_bytes != self.expected_total_bytes
        {
            bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_BUDGET_ACCOUNTING_MISMATCH");
        }
        Ok(())
    }
}
