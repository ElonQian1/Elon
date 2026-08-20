use anyhow::{ensure, Result};
use chrono::{DateTime, SecondsFormat};

/// Caller-owned clock facts for one same-transaction V213/V185/V215 terminal ingress.
pub(in crate::store) struct ComputeAttemptAdapterAckIngressTimes {
    observation_transitioned_at: String,
    ingested_at: String,
    activated_at: String,
    closure_at: String,
}

impl ComputeAttemptAdapterAckIngressTimes {
    pub(in crate::store) fn new(
        observation_transitioned_at: String,
        ingested_at: String,
        activated_at: String,
        closure_at: String,
    ) -> Result<Self> {
        for value in [
            &observation_transitioned_at,
            &ingested_at,
            &activated_at,
            &closure_at,
        ] {
            let parsed = DateTime::parse_from_rfc3339(value)?;
            ensure!(
                parsed.offset().local_minus_utc() == 0
                    && parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == *value,
                "V278 terminal ingress time is not canonical UTC nanoseconds"
            );
        }
        ensure!(
            observation_transitioned_at.as_str() <= ingested_at.as_str()
                && ingested_at.as_str() <= activated_at.as_str()
                && activated_at.as_str() <= closure_at.as_str(),
            "V278 terminal ingress times are not monotonic"
        );
        Ok(Self {
            observation_transitioned_at,
            ingested_at,
            activated_at,
            closure_at,
        })
    }

    pub(super) fn observation_transitioned_at(&self) -> &str {
        &self.observation_transitioned_at
    }

    pub(super) fn ingested_at(&self) -> &str {
        &self.ingested_at
    }

    pub(super) fn activated_at(&self) -> &str {
        &self.activated_at
    }

    pub(super) fn closure_at(&self) -> &str {
        &self.closure_at
    }

    pub(in crate::store) fn ensure_not_after(&self, horizon: &str) -> Result<()> {
        ensure!(
            self.closure_at.as_str() < horizon,
            "V278 terminal ingress exceeds its historical cleanup horizon"
        );
        Ok(())
    }
}
