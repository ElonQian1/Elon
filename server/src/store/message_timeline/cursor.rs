use anyhow::{bail, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
pub(super) struct Cursor {
    pub binding: String,
    pub epoch: String,
    pub sequence: Option<i64>,
    pub before: Option<(String, String)>,
}

pub(super) fn binding(owner: &str, kind: &str, project: &str, id: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(owner, kind, project, id)).unwrap())
    )
}

impl Cursor {
    pub fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).unwrap())
    }

    pub fn decode(value: &str, binding: &str, epoch: &str) -> Result<Self> {
        if value.len() > 1024 {
            bail!("invalid_timeline_cursor");
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| anyhow::anyhow!("invalid_timeline_cursor"))?;
        let decoded: Self = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("invalid_timeline_cursor"))?;
        if decoded.binding == binding && decoded.epoch != epoch {
            bail!("timeline_cursor_expired");
        }
        if decoded.binding != binding
            || decoded.epoch != epoch
            || decoded.sequence.is_some_and(|n| n < 0)
            || decoded
                .before
                .as_ref()
                .is_some_and(|(t, id)| t.len() > 64 || id.len() > 160)
        {
            bail!("invalid_timeline_cursor");
        }
        Ok(decoded)
    }
}
