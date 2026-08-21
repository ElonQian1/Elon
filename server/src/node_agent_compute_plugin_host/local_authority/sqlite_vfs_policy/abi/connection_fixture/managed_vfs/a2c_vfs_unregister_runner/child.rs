//! Child-only actual capture and sanitized stdout publication.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};

use super::super::{
    a2_dynamic_evidence::{SanitizedChildReport, A2_DYNAMIC_CHILD_NONCE_ENV},
    a2b2_cases::RegistrationShutdownSelector,
    registration_shutdown_harness::exercise_registration_shutdown,
};

pub(super) const CHILD_ROOT_ENV: &str = "ELON_SQLITE_A2_REGISTRATION_SHUTDOWN_CHILD_ROOT";

pub(super) fn selected_child_root() -> anyhow::Result<Option<PathBuf>> {
    let Some(root) = std::env::var_os(CHILD_ROOT_ENV).map(PathBuf::from) else {
        return Ok(None);
    };
    if !root.is_absolute() {
        return Err(anyhow!("registration shutdown child root is not absolute"));
    }
    Ok(Some(root))
}

pub(super) fn exercise_child(
    root: &Path,
    selector: RegistrationShutdownSelector,
) -> anyhow::Result<()> {
    let nonce =
        std::env::var(A2_DYNAMIC_CHILD_NONCE_ENV).context("read parent-created A2 child nonce")?;
    let actual = exercise_registration_shutdown(root, selector)?;
    let payload = actual.to_report_payload();
    let report = SanitizedChildReport::encode_for_current_child(
        &nonce,
        root,
        actual.identity.target.registration_id,
        &payload,
    )
    .map_err(anyhow::Error::msg)?;
    println!("{report}");
    if !root.is_dir() {
        return Err(anyhow!(
            "registration shutdown child root disappeared before parent observation"
        ));
    }
    Ok(())
}
