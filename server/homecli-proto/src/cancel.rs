use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionSource {
    SupervisorIntervention,
    NodeRestart,
    UpdaterApply,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CancelRequestAudit {
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub requested_at_ms: Option<u128>,
    #[serde(default)]
    pub interruption_source: Option<InterruptionSource>,
}

impl CancelRequestAudit {
    pub fn now(
        requested_by: impl Into<String>,
        source: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let requested_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_millis());
        Self {
            requested_by: Some(requested_by.into()),
            source: Some(source.into()),
            reason: Some(reason.into()),
            requested_at_ms,
            interruption_source: None,
        }
    }

    pub fn with_interruption_source(mut self, source: InterruptionSource) -> Self {
        self.interruption_source = Some(source);
        self
    }
}
