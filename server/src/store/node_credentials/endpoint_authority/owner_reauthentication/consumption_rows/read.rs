use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::node_compute_sharing::endpoint_authority::NodeEndpointOwnerReauthenticationConsumptionEnvelope;

use super::StoredOwnerReauthenticationConsumption;

const SELECT_COLUMNS: &str =
    "consumption_id, consumption_schema, consumption_digest, consumption_json,
     canonicalization, digest_algorithm, reauthentication_receipt_id,
     reauthentication_digest, owner_user_id, authorization_action,
     credential_mutation_request_id, credential_mutation_request_digest,
     authorization_target_digest, current_credential_id, current_credential_revision,
     current_credential_digest, current_credential_status, issued_credential_id,
     issued_credential_revision, issued_credential_digest, revocation_id, revocation_digest,
     consumed_at, recorded_at";

pub(in crate::store::node_credentials::endpoint_authority) fn by_owner_mutation_request_on(
    connection: &Connection,
    owner_user_id: &str,
    mutation_request_id: &str,
) -> Result<Option<StoredOwnerReauthenticationConsumption>> {
    query_one(
        connection,
        "WHERE owner_user_id=?1 AND credential_mutation_request_id=?2",
        params![owner_user_id, mutation_request_id],
    )
}

pub(in crate::store::node_credentials::endpoint_authority) fn by_consumption_id_on(
    connection: &Connection,
    consumption_id: &str,
) -> Result<Option<StoredOwnerReauthenticationConsumption>> {
    query_one(
        connection,
        "WHERE consumption_id=?1",
        params![consumption_id],
    )
}

fn query_one<P: rusqlite::Params>(
    connection: &Connection,
    predicate: &str,
    parameters: P,
) -> Result<Option<StoredOwnerReauthenticationConsumption>> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS}
           FROM node_endpoint_owner_reauthentication_consumptions
           {predicate}"
    );
    connection
        .query_row(&sql, parameters, raw_from_row)
        .optional()?
        .map(RawOwnerReauthenticationConsumption::validate)
        .transpose()
}

struct RawOwnerReauthenticationConsumption {
    consumption_id: String,
    schema: String,
    digest: String,
    json: String,
    canonicalization: String,
    digest_algorithm: String,
    reauthentication_receipt_id: String,
    reauthentication_digest: String,
    owner_user_id: String,
    authorization_action: String,
    mutation_request_id: String,
    mutation_request_digest: String,
    authorization_target_digest: String,
    current_credential_id: String,
    current_credential_revision: i64,
    current_credential_digest: String,
    current_credential_status: String,
    issued_credential_id: Option<String>,
    issued_credential_revision: Option<i64>,
    issued_credential_digest: Option<String>,
    revocation_id: Option<String>,
    revocation_digest: Option<String>,
    consumed_at: String,
    recorded_at: String,
}

fn raw_from_row(row: &Row<'_>) -> rusqlite::Result<RawOwnerReauthenticationConsumption> {
    Ok(RawOwnerReauthenticationConsumption {
        consumption_id: row.get(0)?,
        schema: row.get(1)?,
        digest: row.get(2)?,
        json: row.get(3)?,
        canonicalization: row.get(4)?,
        digest_algorithm: row.get(5)?,
        reauthentication_receipt_id: row.get(6)?,
        reauthentication_digest: row.get(7)?,
        owner_user_id: row.get(8)?,
        authorization_action: row.get(9)?,
        mutation_request_id: row.get(10)?,
        mutation_request_digest: row.get(11)?,
        authorization_target_digest: row.get(12)?,
        current_credential_id: row.get(13)?,
        current_credential_revision: row.get(14)?,
        current_credential_digest: row.get(15)?,
        current_credential_status: row.get(16)?,
        issued_credential_id: row.get(17)?,
        issued_credential_revision: row.get(18)?,
        issued_credential_digest: row.get(19)?,
        revocation_id: row.get(20)?,
        revocation_digest: row.get(21)?,
        consumed_at: row.get(22)?,
        recorded_at: row.get(23)?,
    })
}

impl RawOwnerReauthenticationConsumption {
    fn validate(self) -> Result<StoredOwnerReauthenticationConsumption> {
        let envelope = NodeEndpointOwnerReauthenticationConsumptionEnvelope::from_store_readback(
            &self.json,
            &self.digest,
        )?;
        let current_revision = u64::try_from(self.current_credential_revision)?;
        let issued_revision = self
            .issued_credential_revision
            .map(u64::try_from)
            .transpose()?;
        if self.consumption_id != envelope.consumption_id()
            || self.schema != envelope.schema()
            || self.canonicalization != "rfc8785_jcs"
            || self.digest_algorithm != "sha256"
            || self.reauthentication_receipt_id != envelope.reauthentication_receipt_id()
            || self.reauthentication_digest != envelope.reauthentication_digest()
            || self.owner_user_id != envelope.owner_user_id()
            || self.authorization_action != envelope.authorization_action()
            || self.mutation_request_id != envelope.credential_mutation_request_id()
            || self.mutation_request_digest != envelope.credential_mutation_request_digest()
            || self.authorization_target_digest != envelope.authorization_target_digest()
            || self.current_credential_id != envelope.current_credential_id()
            || current_revision != envelope.current_credential_revision()
            || self.current_credential_digest != envelope.current_credential_digest()
            || self.current_credential_status != envelope.current_credential_status()
            || self.issued_credential_id.as_deref() != envelope.issued_credential_id()
            || issued_revision != envelope.issued_credential_revision()
            || self.issued_credential_digest.as_deref() != envelope.issued_credential_digest()
            || self.revocation_id.as_deref() != envelope.revocation_id()
            || self.revocation_digest.as_deref() != envelope.revocation_digest()
            || self.consumed_at != envelope.consumed_at()
            || self.recorded_at != envelope.recorded_at()
        {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_CONSUMPTION_PROJECTION_MISMATCH");
        }
        Ok(StoredOwnerReauthenticationConsumption {
            envelope,
            consumption_json: self.json,
            consumption_digest: self.digest,
            canonicalization: self.canonicalization,
            digest_algorithm: self.digest_algorithm,
        })
    }
}
