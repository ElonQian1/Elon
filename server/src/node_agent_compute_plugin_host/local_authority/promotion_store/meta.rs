use anyhow::{bail, Context, Result};
use rusqlite::{named_params, Transaction};

use super::{
    super::plan_application::AuthorityPlanApplicationState,
    projection::CandidatePromotionProjection,
};

pub(super) fn update_promotion_authority_meta(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    projection: &CandidatePromotionProjection,
    promoted_at_ms: i64,
) -> Result<()> {
    let authorization_ref = authority
        .sharing_authorization
        .as_ref()
        .map(|binding| binding.authorization_ref.as_str());
    let authorization_revision = authority
        .sharing_authorization
        .as_ref()
        .map(|binding| binding.revision);
    let authorization_digest = authority
        .sharing_authorization
        .as_ref()
        .map(|binding| binding.digest.as_str());
    let updated = transaction
        .execute(
            r#"UPDATE authority_meta SET
                state_revision = :new_state_revision,
                inventory_revision = :new_inventory_revision,
                inventory_digest = :new_inventory_digest,
                inventory_json = :new_inventory_json,
                authority_epoch = :new_authority_epoch
            WHERE singleton = 1
              AND installation_id_digest = :installation_id_digest
              AND state_revision = :old_state_revision
              AND inventory_revision = :old_inventory_revision
              AND inventory_digest = :old_inventory_digest
              AND inventory_json = :old_inventory_json
              AND desired_policy_revision = :desired_policy_revision
              AND sharing_enabled = 1
              AND sharing_authorization_ref IS :authorization_ref
              AND sharing_authorization_revision IS :authorization_revision
              AND sharing_authorization_digest IS :authorization_digest
              AND node_profile_digest = :node_profile_digest
              AND manifest_catalog_revision = :manifest_catalog_revision
              AND target_id = :target_id
              AND host_api_protocol_id = :host_api_protocol_id
              AND host_api_revision = :host_api_revision
              AND active_bundle_revision = :bundle_revision
              AND publisher_keyring_revision = :publisher_revision
              AND publisher_keyring_digest = :publisher_digest
              AND control_keyring_revision = :control_revision
              AND control_keyring_digest = :control_digest
              AND authority_epoch = :old_authority_epoch
              AND process_owner_epoch = :process_owner_epoch
              AND trusted_time_high_water_ms = :promoted_at
              AND clock_status = 'trusted' AND updated_at_ms = :promoted_at"#,
            named_params! {
                ":new_state_revision": projection.state_revision,
                ":new_inventory_revision": projection.inventory.inventory_revision,
                ":new_inventory_digest": &projection.inventory_digest,
                ":new_inventory_json": &projection.inventory_json,
                ":new_authority_epoch": projection.authority_epoch,
                ":installation_id_digest": &authority.installation_id_digest,
                ":old_state_revision": authority.state_revision,
                ":old_inventory_revision": authority.inventory.inventory_revision,
                ":old_inventory_digest": &authority.inventory_digest,
                ":old_inventory_json": &authority.inventory_json,
                ":desired_policy_revision": authority.desired_policy_revision,
                ":authorization_ref": authorization_ref,
                ":authorization_revision": authorization_revision,
                ":authorization_digest": authorization_digest,
                ":node_profile_digest": &authority.node_profile_digest,
                ":manifest_catalog_revision": authority.manifest_catalog_revision,
                ":target_id": &authority.target_id,
                ":host_api_protocol_id": &authority.host_api_protocol_id,
                ":host_api_revision": i64::from(authority.host_api_revision),
                ":bundle_revision": authority.keyring_bundle_revision,
                ":publisher_revision": authority.publisher_keyring.revision,
                ":publisher_digest": &authority.publisher_keyring.digest,
                ":control_revision": authority.control_keyring.revision,
                ":control_digest": &authority.control_keyring.digest,
                ":old_authority_epoch": authority.authority_epoch,
                ":process_owner_epoch": authority.process_owner_epoch,
                ":promoted_at": promoted_at_ms,
            },
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_AUTHORITY_UPDATE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_AUTHORITY_CAS");
    }
    Ok(())
}
