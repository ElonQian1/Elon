use anyhow::Result;
use rusqlite::{params, Connection};

use crate::node_compute_sharing::endpoint_authority::PreparedNodeEndpointOwnerReauthenticationConsumption;

/// Inserts the consumption before credential mutation closes its deferred result references. This
/// function deliberately performs no eager FK/readback check; the composite IMMEDIATE transaction
/// must write the exact credential result and only then read back and commit.
pub(in crate::store::node_credentials::endpoint_authority) fn insert_on(
    connection: &Connection,
    prepared: &PreparedNodeEndpointOwnerReauthenticationConsumption,
) -> Result<()> {
    let envelope = prepared.envelope();
    connection.execute(
        "INSERT INTO node_endpoint_owner_reauthentication_consumptions (
            consumption_id, consumption_schema, consumption_digest, consumption_json,
            canonicalization, digest_algorithm, reauthentication_receipt_id,
            reauthentication_digest, owner_user_id, authorization_action,
            credential_mutation_request_id, credential_mutation_request_digest,
            authorization_target_digest, current_credential_id,
            current_credential_revision, current_credential_digest,
            current_credential_status, issued_credential_id, issued_credential_revision,
            issued_credential_digest, revocation_id, revocation_digest, consumed_at, recorded_at
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,
            ?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24
         )",
        params![
            envelope.consumption_id(),
            envelope.schema(),
            prepared.consumption_digest(),
            prepared.consumption_json(),
            prepared.canonicalization(),
            prepared.digest_algorithm(),
            envelope.reauthentication_receipt_id(),
            envelope.reauthentication_digest(),
            envelope.owner_user_id(),
            envelope.authorization_action(),
            envelope.credential_mutation_request_id(),
            envelope.credential_mutation_request_digest(),
            envelope.authorization_target_digest(),
            envelope.current_credential_id(),
            envelope.current_credential_revision(),
            envelope.current_credential_digest(),
            envelope.current_credential_status(),
            envelope.issued_credential_id(),
            envelope.issued_credential_revision(),
            envelope.issued_credential_digest(),
            envelope.revocation_id(),
            envelope.revocation_digest(),
            envelope.consumed_at(),
            envelope.recorded_at(),
        ],
    )?;
    Ok(())
}
