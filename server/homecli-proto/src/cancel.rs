use serde::{Deserialize, Serialize};

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
        }
    }
}
