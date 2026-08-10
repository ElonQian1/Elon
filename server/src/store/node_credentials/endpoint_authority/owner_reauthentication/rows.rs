use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::node_compute_sharing::endpoint_authority::{
    NodeEndpointOwnerReauthenticationEnvelope, PreparedNodeEndpointOwnerReauthentication,
};

pub(super) struct StoredOwnerReauthentication {
    envelope: NodeEndpointOwnerReauthenticationEnvelope,
    receipt_json: String,
    receipt_digest: String,
}

impl StoredOwnerReauthentication {
    pub(super) fn envelope(&self) -> &NodeEndpointOwnerReauthenticationEnvelope {
        &self.envelope
    }

    pub(super) fn ensure_exact(
        &self,
        prepared: &PreparedNodeEndpointOwnerReauthentication,
    ) -> Result<()> {
        if self.envelope != *prepared.envelope()
            || self.receipt_json != prepared.receipt_json()
            || self.receipt_digest != prepared.receipt_digest()
        {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_REPLAY_MISMATCH");
        }
        Ok(())
    }

    pub(super) fn into_envelope(self) -> NodeEndpointOwnerReauthenticationEnvelope {
        self.envelope
    }
}

pub(super) fn insert_on(
    connection: &Connection,
    prepared: &PreparedNodeEndpointOwnerReauthentication,
) -> Result<()> {
    let envelope = prepared.envelope();
    connection.execute(
        "INSERT INTO node_endpoint_owner_reauthentication_receipts (
            reauthentication_receipt_id, reauthentication_schema, reauthentication_digest,
            reauthentication_json, canonicalization, digest_algorithm, owner_user_id,
            account_session_id, session_binding_digest, account_auth_state_digest,
            authentication_method, authentication_factor_id,
            authentication_factor_binding_digest, authentication_evidence_id,
            authentication_evidence_digest, authorization_issuance_request_id,
            authorization_action, credential_mutation_request_id,
            credential_mutation_request_digest, authorization_target_digest, agent_id,
            install_id, expected_credential_id, expected_credential_revision,
            expected_credential_digest, secure_transport_source,
            secure_transport_evidence_schema, secure_transport_evidence_id,
            secure_transport_evidence_digest, secure_transport_verifier_revision,
            secure_transport_verifier_digest, secure_transport_server_instance_id,
            secure_transport_request_binding_digest, secure_transport_verified_at,
            reauthenticated_at, expires_at, recorded_at
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,
            ?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37
         )",
        params![
            envelope.reauthentication_receipt_id(),
            envelope.schema(),
            prepared.receipt_digest(),
            prepared.receipt_json(),
            "rfc8785_jcs",
            "sha256",
            envelope.owner_user_id(),
            envelope.account_session_id(),
            envelope.session_binding_digest(),
            envelope.account_auth_state_digest(),
            envelope.authentication_method(),
            envelope.authentication_factor_id(),
            envelope.authentication_factor_binding_digest(),
            envelope.authentication_evidence_id(),
            envelope.authentication_evidence_digest(),
            envelope.authorization_issuance_request_id(),
            envelope.authorization_action(),
            envelope.credential_mutation_request_id(),
            envelope.credential_mutation_request_digest(),
            envelope.authorization_target_digest(),
            envelope.agent_id(),
            envelope.install_id(),
            envelope.expected_credential_id(),
            envelope.expected_credential_revision(),
            envelope.expected_credential_digest(),
            envelope.secure_transport_source(),
            envelope.secure_transport_evidence_schema(),
            envelope.secure_transport_evidence_id(),
            envelope.secure_transport_evidence_digest(),
            envelope.secure_transport_verifier_revision(),
            envelope.secure_transport_verifier_digest(),
            envelope.secure_transport_server_instance_id(),
            envelope.secure_transport_request_binding_digest(),
            envelope.secure_transport_verified_at(),
            envelope.reauthenticated_at(),
            envelope.expires_at(),
            envelope.recorded_at(),
        ],
    )?;
    Ok(())
}

pub(super) fn by_receipt_id_on(
    connection: &Connection,
    receipt_id: &str,
) -> Result<Option<StoredOwnerReauthentication>> {
    query_one(
        connection,
        "SELECT reauthentication_receipt_id, reauthentication_schema,
                reauthentication_digest, reauthentication_json, canonicalization,
                digest_algorithm, owner_user_id, account_session_id, session_binding_digest,
                account_auth_state_digest, authentication_method, authentication_factor_id,
                authentication_factor_binding_digest, authentication_evidence_id,
                authentication_evidence_digest, authorization_issuance_request_id,
                authorization_action, credential_mutation_request_id,
                credential_mutation_request_digest, authorization_target_digest, agent_id,
                install_id, expected_credential_id, expected_credential_revision,
                expected_credential_digest, secure_transport_source,
                secure_transport_evidence_schema, secure_transport_evidence_id,
                secure_transport_evidence_digest, secure_transport_verifier_revision,
                secure_transport_verifier_digest, secure_transport_server_instance_id,
                secure_transport_request_binding_digest, secure_transport_verified_at,
                reauthenticated_at, expires_at, recorded_at
           FROM node_endpoint_owner_reauthentication_receipts
          WHERE reauthentication_receipt_id=?1",
        receipt_id,
    )
}

pub(super) fn by_issuance_request_on(
    connection: &Connection,
    owner_user_id: &str,
    issuance_request_id: &str,
) -> Result<Option<StoredOwnerReauthentication>> {
    let mut statement = connection.prepare(
        "SELECT reauthentication_receipt_id, reauthentication_schema,
                reauthentication_digest, reauthentication_json, canonicalization,
                digest_algorithm, owner_user_id, account_session_id, session_binding_digest,
                account_auth_state_digest, authentication_method, authentication_factor_id,
                authentication_factor_binding_digest, authentication_evidence_id,
                authentication_evidence_digest, authorization_issuance_request_id,
                authorization_action, credential_mutation_request_id,
                credential_mutation_request_digest, authorization_target_digest, agent_id,
                install_id, expected_credential_id, expected_credential_revision,
                expected_credential_digest, secure_transport_source,
                secure_transport_evidence_schema, secure_transport_evidence_id,
                secure_transport_evidence_digest, secure_transport_verifier_revision,
                secure_transport_verifier_digest, secure_transport_server_instance_id,
                secure_transport_request_binding_digest, secure_transport_verified_at,
                reauthenticated_at, expires_at, recorded_at
           FROM node_endpoint_owner_reauthentication_receipts
          WHERE owner_user_id=?1 AND authorization_issuance_request_id=?2",
    )?;
    statement
        .query_row(params![owner_user_id, issuance_request_id], raw_from_row)
        .optional()?
        .map(RawOwnerReauthentication::validate)
        .transpose()
}

fn query_one(
    connection: &Connection,
    sql: &str,
    value: &str,
) -> Result<Option<StoredOwnerReauthentication>> {
    let mut statement = connection.prepare(sql)?;
    statement
        .query_row(params![value], raw_from_row)
        .optional()?
        .map(RawOwnerReauthentication::validate)
        .transpose()
}

struct RawOwnerReauthentication {
    receipt_id: String,
    schema: String,
    digest: String,
    json: String,
    canonicalization: String,
    digest_algorithm: String,
    owner_user_id: String,
    account_session_id: String,
    session_binding_digest: String,
    account_auth_state_digest: String,
    authentication_method: String,
    authentication_factor_id: String,
    authentication_factor_binding_digest: String,
    authentication_evidence_id: String,
    authentication_evidence_digest: String,
    issuance_request_id: String,
    authorization_action: String,
    mutation_request_id: String,
    mutation_request_digest: String,
    target_digest: String,
    agent_id: String,
    install_id: String,
    expected_credential_id: Option<String>,
    expected_credential_revision: Option<i64>,
    expected_credential_digest: Option<String>,
    transport_source: String,
    transport_evidence_schema: String,
    transport_evidence_id: String,
    transport_evidence_digest: String,
    transport_verifier_revision: i64,
    transport_verifier_digest: String,
    transport_server_instance_id: String,
    transport_request_binding_digest: String,
    transport_verified_at: String,
    reauthenticated_at: String,
    expires_at: String,
    recorded_at: String,
}

fn raw_from_row(row: &Row<'_>) -> rusqlite::Result<RawOwnerReauthentication> {
    Ok(RawOwnerReauthentication {
        receipt_id: row.get(0)?,
        schema: row.get(1)?,
        digest: row.get(2)?,
        json: row.get(3)?,
        canonicalization: row.get(4)?,
        digest_algorithm: row.get(5)?,
        owner_user_id: row.get(6)?,
        account_session_id: row.get(7)?,
        session_binding_digest: row.get(8)?,
        account_auth_state_digest: row.get(9)?,
        authentication_method: row.get(10)?,
        authentication_factor_id: row.get(11)?,
        authentication_factor_binding_digest: row.get(12)?,
        authentication_evidence_id: row.get(13)?,
        authentication_evidence_digest: row.get(14)?,
        issuance_request_id: row.get(15)?,
        authorization_action: row.get(16)?,
        mutation_request_id: row.get(17)?,
        mutation_request_digest: row.get(18)?,
        target_digest: row.get(19)?,
        agent_id: row.get(20)?,
        install_id: row.get(21)?,
        expected_credential_id: row.get(22)?,
        expected_credential_revision: row.get(23)?,
        expected_credential_digest: row.get(24)?,
        transport_source: row.get(25)?,
        transport_evidence_schema: row.get(26)?,
        transport_evidence_id: row.get(27)?,
        transport_evidence_digest: row.get(28)?,
        transport_verifier_revision: row.get(29)?,
        transport_verifier_digest: row.get(30)?,
        transport_server_instance_id: row.get(31)?,
        transport_request_binding_digest: row.get(32)?,
        transport_verified_at: row.get(33)?,
        reauthenticated_at: row.get(34)?,
        expires_at: row.get(35)?,
        recorded_at: row.get(36)?,
    })
}

impl RawOwnerReauthentication {
    fn validate(self) -> Result<StoredOwnerReauthentication> {
        let envelope = NodeEndpointOwnerReauthenticationEnvelope::from_store_readback(
            &self.json,
            &self.digest,
        )?;
        let expected_revision = self
            .expected_credential_revision
            .map(u64::try_from)
            .transpose()?;
        if self.receipt_id != envelope.reauthentication_receipt_id()
            || self.schema != envelope.schema()
            || self.canonicalization != "rfc8785_jcs"
            || self.digest_algorithm != "sha256"
            || self.owner_user_id != envelope.owner_user_id()
            || self.account_session_id != envelope.account_session_id()
            || self.session_binding_digest != envelope.session_binding_digest()
            || self.account_auth_state_digest != envelope.account_auth_state_digest()
            || self.authentication_method != envelope.authentication_method()
            || self.authentication_factor_id != envelope.authentication_factor_id()
            || self.authentication_factor_binding_digest
                != envelope.authentication_factor_binding_digest()
            || self.authentication_evidence_id != envelope.authentication_evidence_id()
            || self.authentication_evidence_digest != envelope.authentication_evidence_digest()
            || self.issuance_request_id != envelope.authorization_issuance_request_id()
            || self.authorization_action != envelope.authorization_action()
            || self.mutation_request_id != envelope.credential_mutation_request_id()
            || self.mutation_request_digest != envelope.credential_mutation_request_digest()
            || self.target_digest != envelope.authorization_target_digest()
            || self.agent_id != envelope.agent_id()
            || self.install_id != envelope.install_id()
            || self.expected_credential_id.as_deref() != envelope.expected_credential_id()
            || expected_revision != envelope.expected_credential_revision()
            || self.expected_credential_digest.as_deref() != envelope.expected_credential_digest()
            || self.transport_source != envelope.secure_transport_source()
            || self.transport_evidence_schema != envelope.secure_transport_evidence_schema()
            || self.transport_evidence_id != envelope.secure_transport_evidence_id()
            || self.transport_evidence_digest != envelope.secure_transport_evidence_digest()
            || u64::try_from(self.transport_verifier_revision)?
                != envelope.secure_transport_verifier_revision()
            || self.transport_verifier_digest != envelope.secure_transport_verifier_digest()
            || self.transport_server_instance_id != envelope.secure_transport_server_instance_id()
            || self.transport_request_binding_digest
                != envelope.secure_transport_request_binding_digest()
            || self.transport_verified_at != envelope.secure_transport_verified_at()
            || self.reauthenticated_at != envelope.reauthenticated_at()
            || self.expires_at != envelope.expires_at()
            || self.recorded_at != envelope.recorded_at()
        {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_PROJECTION_MISMATCH");
        }
        envelope.validate_store_readback(&self.json, &self.digest)?;
        Ok(StoredOwnerReauthentication {
            envelope,
            receipt_json: self.json,
            receipt_digest: self.digest,
        })
    }
}
