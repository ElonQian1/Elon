use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{bail, Result};

use crate::node_compute_sharing::endpoint_authority::VerifiedSecureNodeEndpointTransport;

const EVIDENCE_LIFETIME: Duration = Duration::from_secs(30);

/// Cloneable connection extension around one non-cloneable proof. Only the first caller can take
/// it, and an unused proof expires against a monotonic clock.
#[derive(Clone)]
pub(super) struct VerifiedSecureTransportSlot {
    proof: Arc<Mutex<Option<VerifiedSecureNodeEndpointTransport>>>,
    deadline: Instant,
}

impl VerifiedSecureTransportSlot {
    pub(super) fn new(proof: VerifiedSecureNodeEndpointTransport) -> Self {
        Self {
            proof: Arc::new(Mutex::new(Some(proof))),
            deadline: Instant::now() + EVIDENCE_LIFETIME,
        }
    }

    pub(super) fn take(&self) -> Result<VerifiedSecureNodeEndpointTransport> {
        let mut guard = self
            .proof
            .lock()
            .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_DIRECT_TLS_EVIDENCE_LOCK_POISONED"))?;
        let proof = guard
            .take()
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_DIRECT_TLS_EVIDENCE_ALREADY_TAKEN"))?;
        if Instant::now() > self.deadline {
            bail!("NODE_ENDPOINT_DIRECT_TLS_EVIDENCE_EXPIRED");
        }
        Ok(proof)
    }
}
