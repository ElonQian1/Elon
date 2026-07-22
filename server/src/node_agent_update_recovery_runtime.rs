//! Runtime release binding for the durable update recovery ledger.

use anyhow::Result;

use super::{now_ms, ReleaseIdentity, UpdateRecoveryState, UpdateRecoveryStore};

impl UpdateRecoveryStore {
    pub(crate) fn mark_runtime_online_if_target(&self, current_release: &str) -> Result<bool> {
        let mut ledger = self.load()?;
        if ledger.receipts.iter().any(|receipt| {
            receipt.state == UpdateRecoveryState::Applying
                && release_identity_matches(current_release, &receipt.from_release)
                && !release_identity_matches(current_release, &receipt.to_release)
        }) {
            return Ok(false);
        }
        let target = ledger.install_gate.target_git_sha.trim();
        if !target.is_empty()
            && current_release.trim() != target
            && !current_release.trim().ends_with(&format!("+{target}"))
        {
            return Ok(false);
        }
        ledger.install_gate.phase = "runtime_online".to_string();
        ledger.install_gate.reason = Some(if target.is_empty() {
            "node runtime is online without a pending target release".to_string()
        } else {
            "intended target node runtime is online".to_string()
        });
        ledger.install_gate.updated_at_ms = now_ms();
        self.save(&ledger)?;
        Ok(true)
    }
}

fn release_identity_matches(current_release: &str, expected: &ReleaseIdentity) -> bool {
    let current_release = current_release.trim();
    let version_matches = expected.version.trim().is_empty()
        || current_release == expected.version.trim()
        || current_release.starts_with(&format!("{}+", expected.version.trim()));
    let sha_matches = expected.git_sha.trim().is_empty()
        || current_release == expected.git_sha.trim()
        || current_release.ends_with(&format!("+{}", expected.git_sha.trim()));
    version_matches && sha_matches
}
