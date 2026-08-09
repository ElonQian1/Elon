use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::node_compute_sharing::endpoint_authority::{
    canonical_node_endpoint_capability_set, NodeEndpointCredentialBinding,
    NodeEndpointSessionAuthenticationReceiptEnvelope, NodeEndpointSessionBinding,
    NodeEndpointSessionHeadSnapshot, PreparedNodeEndpointSessionAuthentication,
};

pub(super) struct StoredSessionReceipt {
    envelope: NodeEndpointSessionAuthenticationReceiptEnvelope,
    authentication_json: String,
    authentication_digest: String,
    capability_set_json: String,
    canonicalization: String,
    digest_algorithm: String,
}

impl StoredSessionReceipt {
    pub(super) fn envelope(&self) -> &NodeEndpointSessionAuthenticationReceiptEnvelope {
        &self.envelope
    }

    pub(super) fn recorded_at(&self) -> &str {
        self.envelope.recorded_at()
    }

    pub(super) fn credential_binding(&self) -> Result<NodeEndpointCredentialBinding> {
        NodeEndpointCredentialBinding::from_store_readback(
            self.envelope.credential_id().to_string(),
            self.envelope.agent_id().to_string(),
            self.envelope.owner_user_id().to_string(),
            self.envelope.install_id().to_string(),
            self.envelope.installation_binding_digest().to_string(),
            self.envelope.credential_revision(),
            self.envelope.credential_digest().to_string(),
            "active".to_string(),
        )
    }

    pub(super) fn session_binding(&self) -> Result<NodeEndpointSessionBinding> {
        NodeEndpointSessionBinding::from_store_readback(
            self.envelope.agent_id().to_string(),
            self.envelope.credential_id().to_string(),
            self.envelope.credential_revision(),
            self.envelope.credential_digest().to_string(),
            self.envelope.authentication_receipt_id().to_string(),
            self.authentication_digest.clone(),
            self.envelope.session_id().to_string(),
            self.envelope.session_generation(),
            self.envelope.server_instance_id().to_string(),
        )
    }

    pub(super) fn predecessor_snapshot(&self) -> Result<NodeEndpointSessionHeadSnapshot> {
        NodeEndpointSessionHeadSnapshot::from_store_readback(
            self.session_binding()?,
            "closed".to_string(),
            self.envelope.authenticated_at().to_string(),
            self.envelope.expires_at().to_string(),
            self.envelope.recorded_at().to_string(),
            self.envelope.recorded_at().to_string(),
            Some(self.envelope.recorded_at().to_string()),
            Some("historical_predecessor".to_string()),
        )
    }

    pub(super) fn into_envelope(self) -> NodeEndpointSessionAuthenticationReceiptEnvelope {
        self.envelope
    }

    pub(super) fn ensure_exact(
        &self,
        prepared: &PreparedNodeEndpointSessionAuthentication,
    ) -> Result<()> {
        if &self.envelope != prepared.envelope()
            || self.authentication_json != prepared.authentication_json()
            || self.authentication_digest != prepared.authentication_digest()
            || self.capability_set_json != prepared.capability_set_json()
            || self.canonicalization != prepared.canonicalization()
            || self.digest_algorithm != prepared.digest_algorithm()
        {
            bail!("NODE_ENDPOINT_SESSION_AUTHENTICATION_REPLAY_MISMATCH");
        }
        Ok(())
    }
}

struct RawSessionReceipt {
    authentication_receipt_id: String,
    authentication_schema: String,
    authentication_digest: String,
    authentication_json: String,
    canonicalization: String,
    digest_algorithm: String,
    credential_id: String,
    credential_revision: i64,
    credential_digest: String,
    agent_id: String,
    owner_user_id: String,
    install_id: String,
    installation_binding_digest: String,
    session_id: String,
    session_generation: i64,
    previous_id: Option<String>,
    previous_digest: Option<String>,
    server_instance_id: String,
    authentication_method: String,
    agent_version: String,
    protocol_version: i64,
    capability_count: i64,
    capability_set_json: String,
    capability_set_digest: String,
    transport_scheme: String,
    transport_security_source: String,
    transport_security_evidence_schema: String,
    transport_security_evidence_id: String,
    transport_security_evidence_digest: String,
    transport_verifier_revision: i64,
    transport_verifier_digest: String,
    transport_verified_at: String,
    authenticated_at: String,
    expires_at: String,
    recorded_at: String,
}

impl RawSessionReceipt {
    fn validate(self) -> Result<StoredSessionReceipt> {
        let envelope: NodeEndpointSessionAuthenticationReceiptEnvelope =
            serde_json::from_str(&self.authentication_json)?;
        envelope.validate_store_readback(&self.authentication_json, &self.authentication_digest)?;
        let transport = envelope.transport();
        let capabilities: Vec<String> = serde_json::from_str(&self.capability_set_json)?;
        let (canonical_capabilities, capability_digest) =
            canonical_node_endpoint_capability_set(&capabilities)?;
        if self.authentication_receipt_id != envelope.authentication_receipt_id()
            || self.authentication_schema != envelope.schema()
            || self.canonicalization != "rfc8785_jcs"
            || self.digest_algorithm != "sha256"
            || self.credential_id != envelope.credential_id()
            || u64::try_from(self.credential_revision)? != envelope.credential_revision()
            || self.credential_digest != envelope.credential_digest()
            || self.agent_id != envelope.agent_id()
            || self.owner_user_id != envelope.owner_user_id()
            || self.install_id != envelope.install_id()
            || self.installation_binding_digest != envelope.installation_binding_digest()
            || self.session_id != envelope.session_id()
            || u64::try_from(self.session_generation)? != envelope.session_generation()
            || self.previous_id.as_deref() != envelope.previous_authentication_receipt_id()
            || self.previous_digest.as_deref() != envelope.previous_authentication_digest()
            || self.server_instance_id != envelope.server_instance_id()
            || self.authentication_method != envelope.authentication_method()
            || self.agent_version != envelope.agent_version()
            || u64::try_from(self.protocol_version)? != envelope.protocol_version()
            || u64::try_from(self.capability_count)? != envelope.capability_count()
            || self.capability_set_digest != envelope.capability_set_digest()
            || capability_digest != envelope.capability_set_digest()
            || self.capability_set_json != canonical_capabilities
            || capabilities.len() as u64 != envelope.capability_count()
            || self.transport_scheme != transport.transport_scheme()
            || self.transport_security_source != transport.transport_security_source()
            || self.transport_security_evidence_schema
                != transport.transport_security_evidence_schema()
            || self.transport_security_evidence_id != transport.transport_security_evidence_id()
            || self.transport_security_evidence_digest
                != transport.transport_security_evidence_digest()
            || u64::try_from(self.transport_verifier_revision)?
                != transport.transport_verifier_revision()
            || self.transport_verifier_digest != transport.transport_verifier_digest()
            || self.transport_verified_at != transport.transport_verified_at()
            || self.authenticated_at != envelope.authenticated_at()
            || self.expires_at != envelope.expires_at()
            || self.recorded_at != envelope.recorded_at()
        {
            bail!("NODE_ENDPOINT_SESSION_AUTHENTICATION_PROJECTION_READBACK_MISMATCH");
        }
        Ok(StoredSessionReceipt {
            envelope,
            authentication_json: self.authentication_json,
            authentication_digest: self.authentication_digest,
            capability_set_json: self.capability_set_json,
            canonicalization: self.canonicalization,
            digest_algorithm: self.digest_algorithm,
        })
    }
}

pub(super) fn receipt_by_session_id_on(
    connection: &Connection,
    session_id: &str,
) -> Result<Option<StoredSessionReceipt>> {
    query_receipt_on(connection, "WHERE session_id=?1", params![session_id])
}

pub(super) fn receipt_by_binding_on(
    connection: &Connection,
    binding: &NodeEndpointSessionBinding,
) -> Result<Option<StoredSessionReceipt>> {
    query_receipt_on(
        connection,
        "WHERE authentication_receipt_id=?1 AND authentication_digest=?2",
        params![
            binding.authentication_receipt_id(),
            binding.authentication_digest()
        ],
    )
}

pub(super) fn receipt_by_id_digest_on(
    connection: &Connection,
    receipt_id: &str,
    digest: &str,
) -> Result<Option<StoredSessionReceipt>> {
    query_receipt_on(
        connection,
        "WHERE authentication_receipt_id=?1 AND authentication_digest=?2",
        params![receipt_id, digest],
    )
}

fn query_receipt_on<P: rusqlite::Params>(
    connection: &Connection,
    predicate: &str,
    parameters: P,
) -> Result<Option<StoredSessionReceipt>> {
    let sql = format!(
        "SELECT authentication_receipt_id, authentication_schema, authentication_digest,
                authentication_json,
                canonicalization, digest_algorithm, credential_id, credential_revision,
                credential_digest, agent_id, owner_user_id, install_id,
                installation_binding_digest, session_id, session_generation,
                previous_authentication_receipt_id, previous_authentication_digest,
                server_instance_id, authentication_method, agent_version, protocol_version,
                capability_count, capability_set_json, capability_set_digest, transport_scheme,
                transport_security_source, transport_security_evidence_schema,
                transport_security_evidence_id, transport_security_evidence_digest,
                transport_verifier_revision, transport_verifier_digest, transport_verified_at,
                authenticated_at, expires_at, recorded_at
           FROM node_endpoint_session_authentication_receipts {predicate}"
    );
    connection
        .query_row(&sql, parameters, map_receipt)
        .optional()?
        .map(RawSessionReceipt::validate)
        .transpose()
}

fn map_receipt(row: &Row<'_>) -> rusqlite::Result<RawSessionReceipt> {
    Ok(RawSessionReceipt {
        authentication_receipt_id: row.get(0)?,
        authentication_schema: row.get(1)?,
        authentication_digest: row.get(2)?,
        authentication_json: row.get(3)?,
        canonicalization: row.get(4)?,
        digest_algorithm: row.get(5)?,
        credential_id: row.get(6)?,
        credential_revision: row.get(7)?,
        credential_digest: row.get(8)?,
        agent_id: row.get(9)?,
        owner_user_id: row.get(10)?,
        install_id: row.get(11)?,
        installation_binding_digest: row.get(12)?,
        session_id: row.get(13)?,
        session_generation: row.get(14)?,
        previous_id: row.get(15)?,
        previous_digest: row.get(16)?,
        server_instance_id: row.get(17)?,
        authentication_method: row.get(18)?,
        agent_version: row.get(19)?,
        protocol_version: row.get(20)?,
        capability_count: row.get(21)?,
        capability_set_json: row.get(22)?,
        capability_set_digest: row.get(23)?,
        transport_scheme: row.get(24)?,
        transport_security_source: row.get(25)?,
        transport_security_evidence_schema: row.get(26)?,
        transport_security_evidence_id: row.get(27)?,
        transport_security_evidence_digest: row.get(28)?,
        transport_verifier_revision: row.get(29)?,
        transport_verifier_digest: row.get(30)?,
        transport_verified_at: row.get(31)?,
        authenticated_at: row.get(32)?,
        expires_at: row.get(33)?,
        recorded_at: row.get(34)?,
    })
}

pub(super) fn insert_receipt_on(
    transaction: &Transaction<'_>,
    prepared: &PreparedNodeEndpointSessionAuthentication,
) -> Result<()> {
    let envelope = prepared.envelope();
    let transport = envelope.transport();
    transaction.execute(
        "INSERT INTO node_endpoint_session_authentication_receipts (
            authentication_receipt_id, authentication_schema, authentication_digest,
            authentication_json, canonicalization, digest_algorithm, credential_id,
            credential_revision, credential_digest, agent_id, owner_user_id, install_id,
            installation_binding_digest, session_id, session_generation,
            previous_authentication_receipt_id, previous_authentication_digest,
            server_instance_id, authentication_method, agent_version, protocol_version,
            capability_count, capability_set_json, capability_set_digest, transport_scheme,
            transport_security_source, transport_security_evidence_schema,
            transport_security_evidence_id, transport_security_evidence_digest,
            transport_verifier_revision, transport_verifier_digest, transport_verified_at,
            authenticated_at, expires_at, recorded_at
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,
            ?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35
         )",
        params![
            envelope.authentication_receipt_id(),
            envelope.schema(),
            prepared.authentication_digest(),
            prepared.authentication_json(),
            prepared.canonicalization(),
            prepared.digest_algorithm(),
            envelope.credential_id(),
            envelope.credential_revision(),
            envelope.credential_digest(),
            envelope.agent_id(),
            envelope.owner_user_id(),
            envelope.install_id(),
            envelope.installation_binding_digest(),
            envelope.session_id(),
            envelope.session_generation(),
            envelope.previous_authentication_receipt_id(),
            envelope.previous_authentication_digest(),
            envelope.server_instance_id(),
            envelope.authentication_method(),
            envelope.agent_version(),
            envelope.protocol_version(),
            envelope.capability_count(),
            prepared.capability_set_json(),
            envelope.capability_set_digest(),
            transport.transport_scheme(),
            transport.transport_security_source(),
            transport.transport_security_evidence_schema(),
            transport.transport_security_evidence_id(),
            transport.transport_security_evidence_digest(),
            transport.transport_verifier_revision(),
            transport.transport_verifier_digest(),
            transport.transport_verified_at(),
            envelope.authenticated_at(),
            envelope.expires_at(),
            envelope.recorded_at(),
        ],
    )?;
    Ok(())
}
