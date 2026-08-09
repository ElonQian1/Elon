use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use crate::node_agent_compute_plugin_host::{
    candidate_promotion_contract::DurableInstalledPluginSlot,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn validate_installed_projection(
    transaction: &Transaction<'_>,
    installed: &DurableInstalledPluginSlot<'_>,
    installation_id_digest: &str,
) -> Result<()> {
    installed.receipts().validate()?;
    let install = installed.receipts().install();
    let promotion = installed.receipts().promotion();
    let install_body = install.receipt();
    let promotion_body = promotion.receipt();
    let release_json = serde_json::to_string(install_body.release())?;
    let install_json = serde_json::to_string(install_body)?;
    let promotion_json = serde_json::to_string(promotion_body)?;
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*)
            FROM candidate_install_receipts AS installation
            JOIN candidate_promotion_receipts AS promotion
              ON promotion.promotion_id = installation.promotion_id
             AND promotion.install_id = installation.install_id
             AND promotion.candidate_token = installation.candidate_token
             AND promotion.install_receipt_digest = installation.receipt_digest
            JOIN candidate_owners AS owner
              ON owner.candidate_token = installation.candidate_token
            WHERE installation.install_id = ?1
              AND installation.receipt_digest = ?2
              AND promotion.promotion_id = ?3
              AND promotion.receipt_digest = ?4
              AND installation.installation_id_digest = ?5
              AND promotion.installation_id_digest = ?5
              AND installation.plugin_id = ?6 AND promotion.plugin_id = ?6
              AND installation.slot_ref = ?7 AND promotion.slot_ref = ?7
              AND installation.release_json = ?8 AND promotion.release_json = ?8
              AND installation.install_generation_after = ?9
              AND promotion.install_generation_after = ?9
              AND promotion.activation_generation_after = ?10
              AND installation.signed_manifest_envelope_digest = ?11
              AND promotion.signed_manifest_envelope_digest = ?11
              AND installation.receipt_json = ?12
              AND promotion.receipt_json = ?13
              AND owner.state = 'promoted' AND owner.plugin_id = ?6
              AND owner.slot_ref = ?7 AND owner.release_json = ?8"#,
            params![
                install_body.install_receipt_id(),
                install.receipt_digest(),
                promotion_body.promotion_receipt_id(),
                promotion.receipt_digest(),
                installation_id_digest,
                install_body.plugin_id(),
                install_body.slot_ref(),
                release_json,
                install_body.install_generation_after(),
                promotion_body.activation_generation_after(),
                install_body.signed_manifest_envelope_digest(),
                install_json,
                promotion_json,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_INSTALLED_PROJECTION_READ")?;
    if count != 1
        || install_body.installation_id_digest() != installation_id_digest
        || promotion_body.installation_id_digest() != installation_id_digest
        || install_body.plugin_id() != promotion_body.plugin_id()
        || install_body.slot_ref() != promotion_body.slot_ref()
        || install_body.release() != promotion_body.release()
        || jcs_sha256_hex(install_body)? != install.receipt_digest()
        || jcs_sha256_hex(promotion_body)? != promotion.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_INSTALLED_PROJECTION_CHANGED");
    }
    Ok(())
}
