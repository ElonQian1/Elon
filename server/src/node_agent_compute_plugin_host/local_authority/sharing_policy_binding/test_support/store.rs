use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, TransactionBehavior};

use super::super::{
    revocation::{
        insert_prepared_revocation, prepare_revocation, read_exact_revocation,
        validate_terminalized_work, PreparedPolicyCapabilityRevocation,
    },
    types::{
        PolicyBindingAuthorityState, PreparedSharingPolicyBindingRequest,
        ProjectedSharingPolicyBinding,
    },
    validation::{project, read_state, ReadPolicyBindingState},
    write::{insert_receipt, read_exact_receipt},
};
use crate::node_agent_compute_plugin_host::local_authority_schema::ensure_schema;

pub(in crate::node_agent_compute_plugin_host::local_authority::sharing_policy_binding) fn projected_binding(
    request: PreparedSharingPolicyBindingRequest,
    current: ReadPolicyBindingState,
    bound_at: &DateTime<Utc>,
) -> ProjectedSharingPolicyBinding {
    project(request, current, bound_at).unwrap()
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sharing_policy_binding) fn connection(
    before: &PolicyBindingAuthorityState,
) -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .unwrap();
    connection
        .pragma_update(None, "trusted_schema", "OFF")
        .unwrap();
    ensure_schema(&mut connection).unwrap();
    connection
        .execute(
            r#"INSERT INTO authority_meta (
                singleton, schema_version, installation_id_digest,
                state_revision, inventory_revision, inventory_digest, inventory_json,
                desired_policy_revision, sharing_enabled,
                sharing_authorization_ref, sharing_authorization_revision,
                sharing_authorization_digest, node_profile_digest,
                manifest_catalog_revision, target_id, host_api_protocol_id,
                host_api_revision, active_bundle_revision,
                publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest,
                authority_epoch, process_owner_epoch,
                trusted_time_high_water_ms, clock_status, updated_at_ms
            ) VALUES (
                1, 3, ?1,
                ?2, ?3, ?4, ?5,
                ?6, ?7,
                ?8, ?9,
                ?10, ?11,
                0, 'windows_x86_64', 'elon_compute_plugin_host',
                1, NULL,
                NULL, NULL,
                NULL, NULL,
                ?12, ?13,
                ?14, 'trusted', ?15
            )"#,
            params![
                &before.installation_id_digest,
                before.state_revision,
                before.inventory_revision,
                &before.inventory_digest,
                &before.inventory_json,
                before.desired_policy_revision,
                i64::from(before.sharing_enabled),
                before.sharing_authorization_ref.as_deref(),
                before.sharing_authorization_revision,
                before.sharing_authorization_digest.as_deref(),
                "f".repeat(64),
                before.authority_epoch,
                before.process_owner_epoch,
                before.trusted_time_high_water_ms,
                before.updated_at_ms,
            ],
        )
        .unwrap();
    connection
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sharing_policy_binding) fn commit_transition(
    connection: &mut Connection,
    projected: &ProjectedSharingPolicyBinding,
) -> PreparedPolicyCapabilityRevocation {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let prepared = prepare_revocation(&transaction, projected).unwrap();
    insert_prepared_revocation(&transaction, &prepared).unwrap();
    insert_receipt(&transaction, projected).unwrap();

    let binding = read_exact_receipt(&transaction, &projected.request)
        .unwrap()
        .unwrap();
    assert_eq!(binding, projected.hashed_receipt);
    let revocation = read_exact_revocation(&transaction, &projected.request, &binding)
        .unwrap()
        .unwrap();
    assert_eq!(revocation, prepared);
    validate_terminalized_work(&transaction, &revocation).unwrap();
    transaction.commit().unwrap();
    prepared
}

pub(in crate::node_agent_compute_plugin_host::local_authority::sharing_policy_binding) fn read_current_state(
    connection: &mut Connection,
    trusted_now: &DateTime<Utc>,
) -> ReadPolicyBindingState {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();
    let current = read_state(&transaction, trusted_now).unwrap();
    transaction.rollback().unwrap();
    current
}
