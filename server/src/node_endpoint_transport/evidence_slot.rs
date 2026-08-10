use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{bail, Result};

use crate::node_compute_sharing::endpoint_authority::VerifiedDirectTlsConnectionEvidence;

const EVIDENCE_LIFETIME: Duration = Duration::from_secs(30);

/// Cloneable connection extension around one non-cloneable neutral evidence value. Only the first
/// caller can take it, and unused evidence expires against a monotonic clock.
#[derive(Clone)]
pub(super) struct VerifiedSecureTransportSlot {
    evidence: Arc<Mutex<Option<VerifiedDirectTlsConnectionEvidence>>>,
    deadline: Instant,
}

impl VerifiedSecureTransportSlot {
    pub(super) fn new(evidence: VerifiedDirectTlsConnectionEvidence) -> Self {
        Self {
            evidence: Arc::new(Mutex::new(Some(evidence))),
            deadline: Instant::now() + EVIDENCE_LIFETIME,
        }
    }

    pub(super) fn take(&self) -> Result<VerifiedDirectTlsConnectionEvidence> {
        let mut guard = self
            .evidence
            .lock()
            .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_DIRECT_TLS_EVIDENCE_LOCK_POISONED"))?;
        let evidence = guard
            .take()
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_DIRECT_TLS_EVIDENCE_ALREADY_TAKEN"))?;
        if Instant::now() > self.deadline {
            bail!("NODE_ENDPOINT_DIRECT_TLS_EVIDENCE_EXPIRED");
        }
        Ok(evidence)
    }
}
